//! Explicit, pinned model management. This module never receives translation text.
use jingyi_engine::{lock_models, plain_directory, reject_link};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct Asset {
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Segmentation {
    pub filename: String,
    #[serde(flatten)]
    pub asset: Asset,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct ModelFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Model {
    pub id: String,
    pub source: String,
    pub target: String,
    pub version: String,
    pub folder: String,
    pub archive: Asset,
    pub segmentation: Segmentation,
    pub files: Vec<ModelFile>,
}
pub fn catalog() -> Vec<Model> {
    serde_json::from_str(include_str!("../model-catalog.json"))
        .expect("Invalid embedded model catalog")
}
fn find(id: &str) -> Result<Model, String> {
    catalog()
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| "不支持的模型。".into())
}

#[derive(Clone, Default, Serialize)]
pub struct Progress {
    pub active: bool,
    pub id: String,
    pub phase: String,
    pub received: u64,
    pub total: u64,
    pub cancellable: bool,
}
#[derive(Default)]
pub struct ModelManager {
    progress: Mutex<Progress>,
    cancel: AtomicBool,
}
pub struct Operation<'a> {
    manager: &'a ModelManager,
}
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        let mut progress = self.manager.progress.lock().unwrap();
        progress.active = false;
        progress.cancellable = false;
    }
}
impl ModelManager {
    pub fn status(&self) -> Progress {
        self.progress.lock().unwrap().clone()
    }
    pub fn cancel(&self) {
        // Serialize against begin(), so cancellation cannot be lost at startup.
        let progress = self.progress.lock().unwrap();
        if progress.active && progress.cancellable {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn begin(&self, id: &str, downloading: bool) -> Result<Operation<'_>, String> {
        let model = find(id)?;
        let mut progress = self.progress.lock().unwrap();
        if progress.active {
            return Err("已有模型操作正在进行。".into());
        }
        self.cancel.store(false, Ordering::Relaxed);
        *progress = Progress {
            active: true,
            id: model.id,
            phase: if downloading {
                "准备下载"
            } else {
                "删除中"
            }
            .into(),
            total: model.archive.size + model.segmentation.asset.size,
            cancellable: downloading,
            received: 0,
        };
        Ok(Operation { manager: self })
    }
    fn check_cancel(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("下载已取消。".into())
        } else {
            Ok(())
        }
    }
    fn phase(&self, phase: &str) {
        self.progress.lock().unwrap().phase = phase.into();
    }
    async fn cancellable<T>(
        &self,
        future: impl std::future::Future<Output = T>,
    ) -> Result<T, String> {
        tokio::pin!(future);
        loop {
            self.check_cancel()?;
            tokio::select! {
                value = &mut future => return Ok(value),
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }
    }
    async fn download(
        &self,
        client: &reqwest::Client,
        asset: &Asset,
        destination: &Path,
    ) -> Result<(), String> {
        let mut response = self
            .cancellable(client.get(&asset.url).send())
            .await?
            .map_err(|_| "无法连接模型源，请检查网络后重试。")?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(format!(
                "模型源返回 HTTP {}，请稍后重试。",
                response.status().as_u16()
            ));
        }
        if response.content_length().is_some_and(|n| n != asset.size) {
            return Err("模型文件大小与内置清单不符，已停止下载。".into());
        }
        let mut output = File::create(destination)
            .map_err(|_| "无法创建模型临时文件，请检查空间和写入权限。")?;
        let mut received = 0;
        let mut digest = Sha256::new();
        while let Some(chunk) = self
            .cancellable(response.chunk())
            .await?
            .map_err(|_| "下载中断，请检查网络后重试。")?
        {
            received += chunk.len() as u64;
            if received > asset.size {
                return Err("模型超过预期大小，已停止下载。".into());
            }
            output
                .write_all(&chunk)
                .map_err(|_| "模型写入失败，请检查磁盘空间。")?;
            digest.update(&chunk);
            self.progress.lock().unwrap().received += chunk.len() as u64;
        }
        output.sync_all().map_err(|_| "模型写入失败。")?;
        if received != asset.size || format!("{:x}", digest.finalize()) != asset.sha256 {
            return Err("模型 SHA-256 校验失败，未安装。请重新下载。".into());
        }
        Ok(())
    }
    /// The operation guard and OS lock must live until commit completes.
    pub async fn prepare(&self, root: &Path, model: &Model) -> Result<tempfile::TempDir, String> {
        cleanup_staging(root)?;
        let stage = tempfile::Builder::new()
            .prefix("stage-")
            .tempdir_in(root.join(".manager"))
            .map_err(|_| "无法创建模型临时目录。")?;
        fs::write(stage.path().join(".jingyi-stage"), []).map_err(|_| "无法创建模型临时标记。")?;
        let client = client()?;
        self.phase("下载翻译模型");
        self.download(&client, &model.archive, &stage.path().join("package.part"))
            .await?;
        self.phase("下载分句模型");
        self.download(
            &client,
            &model.segmentation.asset,
            &stage.path().join("sbd.part"),
        )
        .await?;
        self.phase("校验与解包");
        self.check_cancel()?;
        extract(
            model,
            &stage.path().join("package.part"),
            &stage.path().join("package"),
            &self.cancel,
        )?;
        self.check_cancel()?;
        Ok(stage)
    }
    pub fn install(
        &self,
        root: &Path,
        model: &Model,
        stage: &Path,
        engine: &mut jingyi_engine::Engine,
    ) -> Result<(), String> {
        {
            let mut progress = self.progress.lock().unwrap();
            self.check_cancel()?;
            progress.cancellable = false;
        }
        self.phase("安装中");
        // Validate by loading the exact staged CTranslate2, SentencePiece and ONNX
        // files. A successful HTTP transfer alone does not make a usable model.
        let probe_root = stage.join("probe");
        plain_directory(&probe_root.join("packages"))?;
        plain_directory(&probe_root.join("minisbd"))?;
        fs::rename(
            stage.join("package"),
            probe_root.join("packages").join(&model.folder),
        )
        .map_err(|_| "无法准备模型。")?;
        fs::rename(
            stage.join("sbd.part"),
            probe_root
                .join("minisbd")
                .join(&model.segmentation.filename),
        )
        .map_err(|_| "无法准备分句模型。")?;
        {
            let mut probe = jingyi_engine::Engine::new(probe_root.clone());
            probe
                .tokenize("test", &model.source, &model.target)
                .map_err(|_| "模型加载验证失败，未安装。")?;
        }
        plain_directory(&root.join("packages"))?;
        plain_directory(&root.join("minisbd"))?;
        let destination = root.join("packages").join(&model.folder);
        reject_link(&destination)?;
        if destination.exists() {
            return Err("此模型目录已存在，请先删除后重新下载。".into());
        }
        engine.refresh(); // unload handles before replacing shared model data
        let sbd = root.join("minisbd").join(&model.segmentation.filename);
        reject_link(&sbd)?;
        // A valid existing shared file is left untouched. A broken file is replaced
        // with rollback on failure, before the package directory becomes visible.
        let staged_sbd = probe_root
            .join("minisbd")
            .join(&model.segmentation.filename);
        if !matches_asset(&sbd, &model.segmentation.asset) {
            if sbd.exists() {
                fs::rename(&sbd, stage.join("old-sbd"))
                    .map_err(|_| "分句模型正在使用，无法替换。")?;
            }
            if fs::rename(&staged_sbd, &sbd).is_err() {
                if stage.join("old-sbd").exists() {
                    let _ = fs::rename(stage.join("old-sbd"), &sbd);
                }
                return Err("无法安装分句模型。".into());
            }
        }
        fs::rename(probe_root.join("packages").join(&model.folder), destination)
            .map_err(|_| "无法安装模型，请检查目录权限。")?;
        bump_revision(root)?;
        engine.refresh();
        Ok(())
    }
    pub fn remove(
        &self,
        root: &Path,
        id: &str,
        engine: &mut jingyi_engine::Engine,
    ) -> Result<(), String> {
        let model = find(id)?;
        let _lock = lock_models(root, true)?;
        cleanup_staging(root)?;
        plain_directory(&root.join("packages"))?;
        plain_directory(&root.join("minisbd"))?;
        let path = root.join("packages").join(&model.folder);
        if !path.exists() {
            reject_link(&path)?;
            return Err("模型已不存在，请刷新列表。".into());
        }
        plain_tree(&path)?;
        engine.refresh();
        // Delete only the catalog's exact package folder, never paths from IPC.
        // Renaming first prevents partial deletions being visible to inference.
        let trash = tempfile::Builder::new()
            .prefix("stage-")
            .tempdir_in(root.join(".manager"))
            .map_err(|_| "无法创建模型清理目录。")?;
        fs::write(trash.path().join(".jingyi-stage"), []).map_err(|_| "无法创建清理标记。")?;
        fs::rename(&path, trash.path().join("removed"))
            .map_err(|_| "模型正在使用或目录不可写，删除失败。")?;
        bump_revision(root)?;
        engine.refresh();
        // Keep shared MiniSBD data (under 1 MB) for manual packages/other directions.
        trash
            .close()
            .map_err(|_| "模型已停用，残留文件将在下次模型操作时清理。".to_owned())
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .https_only(true)
        .referer(false)
        .user_agent(concat!("Jingyi/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(1200))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !allowed_url(attempt.url()) {
                attempt.error("untrusted model redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| "无法初始化模型下载。".into())
}
fn allowed_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "argos-net.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
fn matches_asset(path: &Path, asset: &Asset) -> bool {
    (|| -> std::io::Result<bool> {
        let mut file = File::open(path)?;
        if file.metadata()?.len() != asset.size {
            return Ok(false);
        }
        let mut digest = Sha256::new();
        std::io::copy(&mut file, &mut digest)?;
        Ok(format!("{:x}", digest.finalize()) == asset.sha256)
    })()
    .unwrap_or(false)
}
fn safe_archive_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', ':', '\0'])
        && !name.starts_with('/')
        && name.trim_end_matches('/').split('/').all(|part| {
            !part.is_empty() && part != "." && part != ".." && !part.ends_with(['.', ' '])
        })
}
fn extract(
    model: &Model,
    archive: &Path,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let file = File::open(archive).map_err(|_| "无法读取模型包。")?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "模型压缩包无效。")?;
    if zip.len() > 2048 {
        return Err("模型压缩包文件过多。".into());
    }
    let mut total = 0u64;
    let mut selected = HashSet::new();
    fs::create_dir(destination).map_err(|_| "无法创建模型解包目录。")?;
    for index in 0..zip.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err("下载已取消。".into());
        }
        let mut entry = zip.by_index(index).map_err(|_| "无法读取模型压缩包。")?;
        total = total.saturating_add(entry.size());
        let kind = entry.unix_mode().unwrap_or(0) & 0o170000;
        if !safe_archive_name(entry.name())
            || entry.enclosed_name().is_none()
            || !matches!(kind, 0 | 0o100000 | 0o040000)
            || total > 512 * 1024 * 1024
        {
            return Err("模型包包含不安全路径、链接或超限内容。".into());
        }
        let Some(relative) = entry.name().strip_prefix(&format!("{}/", model.folder)) else {
            return Err("模型包目录不符。".into());
        };
        let Some(expected) = model.files.iter().find(|f| f.path == relative) else {
            continue;
        };
        if entry.is_dir() || entry.size() != expected.size || !selected.insert(relative.to_owned())
        {
            return Err("模型包包含重复或异常文件。".into());
        }
        let output = destination.join(relative);
        fs::create_dir_all(output.parent().unwrap()).map_err(|_| "无法创建模型目录。")?;
        let mut out = File::create(&output).map_err(|_| "模型写入失败。")?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut count = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("下载已取消。".into());
            }
            let size = entry.read(&mut buffer).map_err(|_| "模型解压失败。")?;
            if size == 0 {
                break;
            }
            count += size as u64;
            if count > expected.size {
                return Err("模型解压超过预期大小。".into());
            }
            out.write_all(&buffer[..size])
                .map_err(|_| "模型写入失败，请检查空间。")?;
            digest.update(&buffer[..size]);
        }
        out.sync_all().map_err(|_| "模型写入失败。")?;
        if count != expected.size || format!("{:x}", digest.finalize()) != expected.sha256 {
            return Err("模型内容校验失败，未安装。".into());
        }
    }
    if selected.len() != model.files.len() {
        return Err("模型包缺少必要文件。".into());
    }
    let metadata: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join("metadata.json")).map_err(|_| "缺少模型信息。")?,
    )
    .map_err(|_| "模型信息无效。")?;
    if metadata["from_code"] != model.source
        || metadata["to_code"] != model.target
        || metadata["package_version"] != model.version
    {
        return Err("模型语言或版本不匹配。".into());
    }
    Ok(())
}
fn plain_tree(path: &Path) -> Result<(), String> {
    reject_link(path)?;
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|_| "无法检查模型目录。")? {
            plain_tree(&entry.map_err(|_| "无法检查模型文件。")?.path())?;
        }
    }
    Ok(())
}
fn cleanup_staging(root: &Path) -> Result<(), String> {
    // Caller holds the exclusive model lock: no other live installer owns stages.
    for entry in fs::read_dir(root.join(".manager")).map_err(|_| "无法读取模型管理目录。")?
    {
        let entry = entry.map_err(|_| "无法读取模型临时文件。")?;
        if entry.file_name().to_string_lossy().starts_with("stage-")
            && entry.path().join(".jingyi-stage").is_file()
        {
            plain_tree(&entry.path())?;
            fs::remove_dir_all(entry.path())
                .map_err(|_| "无法清理上次模型操作，请关闭其他实例后重试。")?;
        }
    }
    Ok(())
}
fn bump_revision(root: &Path) -> Result<(), String> {
    let revision = root.join(".manager/revision");
    reject_link(&revision)?;
    fs::write(
        revision,
        format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ),
    )
    .map_err(|_| "模型已变更，但状态刷新失败，请重新启动。".into())
}

pub fn list(root: &Path) -> serde_json::Value {
    let models: Vec<_> = catalog().iter().map(|model| {
        let folder = root.join("packages").join(&model.folder);
        let present = fs::symlink_metadata(&folder).is_ok();
        let installed = model.files.iter().all(|f| folder.join(&f.path).metadata().is_ok_and(|m| m.is_file() && m.len() == f.size))
            && root.join("minisbd").join(&model.segmentation.filename).metadata().is_ok_and(|m| m.len() == model.segmentation.asset.size);
        serde_json::json!({"id":model.id,"source":model.source,"target":model.target,"version":model.version,
            "bytes":model.archive.size + model.segmentation.asset.size,"present":present,"installed":installed})
    }).collect();
    serde_json::json!(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_and_network_boundary() {
        assert_eq!(catalog().len(), 2);
        assert!(find("../../outside").is_err());
        for model in catalog() {
            assert!(allowed_url(&model.archive.url.parse().unwrap()));
            assert!(allowed_url(&model.segmentation.asset.url.parse().unwrap()));
            assert_eq!(model.archive.sha256.len(), 64);
            assert_eq!(model.files.len(), 6);
        }
        for url in [
            "http://argos-net.com/file",
            "https://evil.example/file",
            "https://github.com.evil.example/a",
            "https://user:secret@github.com/a",
            "https://github.com:8443/a",
        ] {
            assert!(!allowed_url(&url.parse().unwrap()));
        }
        for path in [
            "../escape",
            "/absolute",
            "C:/foo",
            "a\\..\\b",
            "a/../b",
            "a//b",
            "a./b",
            "a /b",
        ] {
            assert!(!safe_archive_name(path), "{path}");
        }
    }
    #[test]
    fn cancellation_and_operation_exclusion() {
        let manager = ModelManager::default();
        let first = manager.begin("en-zh", true).unwrap();
        assert!(manager.begin("zh-en", true).is_err());
        manager.cancel();
        assert!(manager.check_cancel().is_err());
        drop(first);
        assert!(!manager.status().active);
        let _second = manager.begin("zh-en", true).unwrap();
        assert!(manager.check_cancel().is_ok());
    }
    #[test]
    fn lock_and_abandoned_stage_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let first = lock_models(root.path(), false).unwrap();
        assert!(lock_models(root.path(), true).is_err());
        drop(first);
        let _exclusive = lock_models(root.path(), true).unwrap();
        assert!(lock_models(root.path(), false).is_err());
        let stale = root.path().join(".manager/stage-abandoned");
        let user = root.path().join(".manager/stage-user");
        fs::create_dir(&stale).unwrap();
        fs::create_dir(&user).unwrap();
        fs::write(stale.join(".jingyi-stage"), []).unwrap();
        cleanup_staging(root.path()).unwrap();
        assert!(!stale.exists());
        assert!(user.exists());
    }

    #[test]
    fn zip_rejects_traversal_links_missing_files_and_wrong_hashes() {
        use zip::write::SimpleFileOptions;
        let root = tempfile::tempdir().unwrap();
        let mut model = catalog().remove(0);
        let data = br#"{"from_code":"en","to_code":"zh","package_version":"1.9"}"#;
        model.files = vec![ModelFile {
            path: "metadata.json".into(),
            size: data.len() as u64,
            sha256: format!("{:x}", Sha256::digest(data)),
        }];
        for (index, bad) in ["traversal", "link", "missing", "hash", "valid"]
            .iter()
            .enumerate()
        {
            let archive = root.path().join(format!("{index}.zip"));
            let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
            let options = SimpleFileOptions::default();
            if *bad == "traversal" {
                writer.start_file("../escape", options).unwrap();
                writer.write_all(b"bad").unwrap();
            } else if *bad == "link" {
                writer
                    .add_symlink(format!("{}/link", model.folder), "../../escape", options)
                    .unwrap();
            } else if *bad == "missing" {
                writer
                    .start_file(format!("{}/ignored.txt", model.folder), options)
                    .unwrap();
                writer.write_all(b"unused").unwrap();
            } else {
                writer
                    .start_file(format!("{}/metadata.json", model.folder), options)
                    .unwrap();
                let wrong = vec![b'x'; data.len()];
                writer
                    .write_all(if *bad == "hash" { &wrong } else { data })
                    .unwrap();
            }
            writer.finish().unwrap();
            let result = extract(
                &model,
                &archive,
                &root.path().join(format!("out-{index}")),
                &AtomicBool::new(false),
            );
            if *bad == "valid" {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result.is_err(), "{bad}");
            }
        }
        assert!(!root.path().join("escape").exists());
    }

    #[tokio::test]
    async fn http_errors_integrity_and_cancel_leave_no_installed_model() {
        use std::net::TcpListener;
        fn serve(response: Vec<u8>, delay: bool) -> (String, std::thread::JoinHandle<()>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/model", listener.local_addr().unwrap());
            let thread = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request);
                let _ = socket.write_all(&response);
                if delay {
                    std::thread::sleep(Duration::from_millis(400));
                }
            });
            (url, thread)
        }
        // Local fault-injection client only. Production client is HTTPS-only and
        // IPC accepts catalog IDs, never a URL or arbitrary destination path.
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        for (index, response) in [
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\nabc".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\na".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nbad".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc".to_vec(),
        ]
        .into_iter()
        .enumerate()
        {
            let manager = ModelManager::default();
            let _operation = manager.begin("en-zh", true).unwrap();
            let (url, server) = serve(response, false);
            let asset = Asset {
                url,
                size: 3,
                sha256: format!("{:x}", Sha256::digest(b"abc")),
            };
            let stage = tempfile::tempdir_in(root.path()).unwrap();
            let result = manager
                .download(&client, &asset, &stage.path().join("part"))
                .await;
            assert_eq!(result.is_ok(), index == 4, "{index}: {result:?}");
            drop(stage);
            server.join().unwrap();
        }
        let manager = ModelManager::default();
        let _operation = manager.begin("en-zh", true).unwrap();
        let (url, server) = serve(
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\na".to_vec(),
            true,
        );
        let asset = Asset {
            url,
            size: 3,
            sha256: String::new(),
        };
        let stage = tempfile::tempdir_in(root.path()).unwrap();
        let part = stage.path().join("part");
        let (result, _) = tokio::join!(manager.download(&client, &asset, &part), async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            manager.cancel();
        });
        assert_eq!(result.unwrap_err(), "下载已取消。");
        drop(stage);
        server.join().unwrap();
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    #[ignore = "Downloads official weights and executes real CPU models; run explicitly"]
    async fn official_download_install_translate_remove() {
        lifecycle(true).await;
    }
    #[tokio::test]
    #[ignore = "Uses maintainer cache and executes real CPU models; run explicitly"]
    async fn cached_install_translate_remove() {
        lifecycle(false).await;
    }
    async fn lifecycle(network: bool) {
        let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".cache");
        let temporary = tempfile::tempdir_in(&parent).unwrap();
        let root = temporary.path().join("models");
        let manager = ModelManager::default();
        let mut engine = jingyi_engine::Engine::new(root.clone());
        assert_eq!(
            list(&root)
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["installed"] == true)
                .count(),
            0
        );
        for model in catalog() {
            let operation = manager.begin(&model.id, true).unwrap();
            let lock = lock_models(&root, true).unwrap();
            let stage = if network {
                manager.prepare(&root, &model).await.unwrap()
            } else {
                cleanup_staging(&root).unwrap();
                let stage = tempfile::Builder::new()
                    .prefix("stage-")
                    .tempdir_in(root.join(".manager"))
                    .unwrap();
                fs::write(stage.path().join(".jingyi-stage"), []).unwrap();
                let cache = parent.join("model-downloads");
                let archive = cache.join(model.archive.url.rsplit('/').next().unwrap());
                assert!(matches_asset(&archive, &model.archive));
                extract(
                    &model,
                    &archive,
                    &stage.path().join("package"),
                    &manager.cancel,
                )
                .unwrap();
                let sbd = cache.join(&model.segmentation.filename);
                assert!(matches_asset(&sbd, &model.segmentation.asset));
                fs::copy(sbd, stage.path().join("sbd.part")).unwrap();
                stage
            };
            manager
                .install(&root, &model, stage.path(), &mut engine)
                .unwrap();
            drop(stage);
            drop(lock);
            drop(operation);
            let sample = if model.source == "en" {
                "Hello, world!"
            } else {
                "你好，世界！"
            };
            let result = engine.request(&serde_json::json!({"op":"text","text":sample,"source":model.source,"target":model.target})).unwrap();
            assert!(!result["text"].as_str().unwrap().trim().is_empty());
            eprintln!("{} translated: {}", model.id, result["text"]);
        }
        assert!(list(&root)
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["installed"] == true));
        let mut other = jingyi_engine::Engine::new(root.clone());
        other
            .request(&serde_json::json!({"op":"text","text":"Hello","source":"en","target":"zh"}))
            .unwrap();
        let unrelated = root.join("packages/user-sentinel.txt");
        fs::write(&unrelated, b"keep").unwrap();
        for model in catalog() {
            let _operation = manager.begin(&model.id, false).unwrap();
            manager.remove(&root, &model.id, &mut engine).unwrap();
            assert!(!root.join("packages").join(&model.folder).exists());
        }
        // A second engine must discard cached models on the revision change.
        assert!(other
            .request(&serde_json::json!({"op":"text","text":"Hello","source":"en","target":"zh"}))
            .is_err());
        assert_eq!(fs::read(unrelated).unwrap(), b"keep");
        assert!(!manager.status().active);
        assert!(list(&root)
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["present"] == false));
        assert!(!fs::read_dir(root.join(".manager"))
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("stage-")));
    }
}
