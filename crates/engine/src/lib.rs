mod ffi;
mod sbd;

use sentencepiece_rs::SentencePieceProcessor;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Package {
    pub from_code: String,
    pub to_code: String,
    pub package_version: String,
    #[serde(default)]
    pub from_name: String,
    #[serde(default)]
    pub to_name: String,
    #[serde(default)]
    pub target_prefix: String,
    #[serde(skip)]
    pub path: PathBuf,
}
struct Loaded {
    tokenizer: SentencePieceProcessor,
    model: ffi::Model,
    sbd: sbd::SentenceDetector,
}
pub struct Engine {
    model_root: PathBuf,
    packages: Vec<Package>,
    loaded: HashMap<String, Loaded>,
    revision: Vec<u8>,
}
impl Engine {
    pub fn new(model_root: PathBuf) -> Self {
        let mut engine = Self {
            model_root,
            packages: Vec::new(),
            loaded: HashMap::new(),
            revision: Vec::new(),
        };
        engine.refresh();
        engine
    }
    pub fn refresh(&mut self) {
        self.revision =
            std::fs::read(self.model_root.join(".manager/revision")).unwrap_or_default();
        self.loaded.clear();
        self.packages.clear();
        let Ok(entries) = std::fs::read_dir(self.model_root.join("packages")) else {
            return;
        };
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let path = entry.path();
            let Ok(bytes) = std::fs::read(path.join("metadata.json")) else {
                continue;
            };
            let Ok(mut package) = serde_json::from_slice::<Package>(&bytes) else {
                continue;
            };
            if package.from_code.is_empty()
                || package.to_code.is_empty()
                || !package.target_prefix.is_empty()
                || !path.join("model/model.bin").is_file()
                || !path.join("sentencepiece.model").is_file()
                || !self
                    .model_root
                    .join("minisbd")
                    .join(format!("{}.onnx", sbd_language(&package.from_code)))
                    .is_file()
            {
                continue;
            }
            package.path = path;
            self.packages.push(package);
        }
        self.packages.sort_by(|a, b| {
            (&a.from_code, &a.to_code, &a.package_version).cmp(&(
                &b.from_code,
                &b.to_code,
                &b.package_version,
            ))
        });
    }
    pub fn info(&self) -> Value {
        let mut languages = BTreeMap::<String, Value>::new();
        for package in &self.packages {
            languages.entry(package.from_code.clone()).or_insert_with(
                || json!({"code":package.from_code,"name":package.from_name,"targets":[]}),
            );
            languages.entry(package.to_code.clone()).or_insert_with(
                || json!({"code":package.to_code,"name":package.to_name,"targets":[]}),
            );
            let targets = languages.get_mut(&package.from_code).unwrap()["targets"]
                .as_array_mut()
                .unwrap();
            if !targets.contains(&json!(package.to_code)) {
                targets.push(json!(package.to_code));
            }
        }
        json!({"languages":languages.into_values().collect::<Vec<_>>(),
            "packages":self.packages.iter().map(|p|json!({"from":p.from_code,"to":p.to_code,"version":p.package_version})).collect::<Vec<_>>(),
            "formats":[], "native":true})
    }
    fn load(&mut self, source: &str, target: &str) -> Result<&mut Loaded, String> {
        if source == target {
            return Err("请选择不同的源语言和目标语言。".into());
        }
        let key = format!("{source}-{target}");
        if !self.loaded.contains_key(&key) {
            let package = self
                .packages
                .iter()
                .find(|p| p.from_code == source && p.to_code == target)
                .ok_or("没有安装对应方向的本地模型。")?;
            let tokenizer = SentencePieceProcessor::open(package.path.join("sentencepiece.model"))
                .map_err(|_| "无法加载 SentencePiece 模型")?;
            let model = ffi::Model::load(&package.path.join("model"))?;
            let language = sbd_language(source);
            let sbd = sbd::SentenceDetector::load(
                &self
                    .model_root
                    .join("minisbd")
                    .join(format!("{language}.onnx")),
            )?;
            self.loaded.insert(
                key.clone(),
                Loaded {
                    tokenizer,
                    model,
                    sbd,
                },
            );
        }
        Ok(self.loaded.get_mut(&key).unwrap())
    }
    pub fn sentences(
        &mut self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<Vec<String>, String> {
        self.load(source, target)?.sbd.sentences(text)
    }
    pub fn tokenize(
        &mut self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<Vec<String>, String> {
        self.load(source, target)?
            .tokenizer
            .encode(text)
            .map_err(|_| "分词失败".into())
    }
    pub fn translate(&mut self, text: &str, source: &str, target: &str) -> Result<String, String> {
        if text.trim().is_empty() || text.chars().count() > 20000 {
            return Err("请输入 1 至 20,000 字文字。".into());
        }
        self.translate_content(text, source, target)
    }
    pub(crate) fn translate_content(
        &mut self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<String, String> {
        if text.trim().is_empty() {
            return Ok(text.to_owned());
        }
        let loaded = self.load(source, target)?;
        let mut paragraphs = Vec::new();
        for paragraph in text.split('\n') {
            let sentences = loaded.sbd.sentences(paragraph)?;
            if sentences.is_empty() {
                paragraphs.push(String::new());
                continue;
            }
            let tokens = sentences
                .iter()
                .map(|s| {
                    loaded
                        .tokenizer
                        .encode(s)
                        .map_err(|_| "分词失败".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let pieces = loaded.model.translate(&tokens)?;
            let value = loaded
                .tokenizer
                .decode(&pieces)
                .map_err(|_| "译文解码失败")?
                .replace(['\u{2581}', '_'], " ");
            paragraphs.push(value.strip_prefix(' ').unwrap_or(&value).to_owned());
        }
        Ok(paragraphs.join("\n"))
    }
    pub fn request(&mut self, payload: &Value) -> Result<Value, String> {
        let _lock = lock_models(&self.model_root, false)?;
        let revision = std::fs::read(self.model_root.join(".manager/revision")).unwrap_or_default();
        if revision != self.revision {
            self.refresh();
        }
        let source = payload["source"].as_str().unwrap_or("");
        let target = payload["target"].as_str().unwrap_or("");
        match payload["op"].as_str().unwrap_or("") {
            "info" => {
                self.refresh();
                Ok(self.info())
            }
            "text" => Ok(
                json!({"text": self.translate(payload["text"].as_str().unwrap_or(""), source, target)?}),
            ),
            "sentences" => Ok(
                json!({"sentences":self.sentences(payload["text"].as_str().unwrap_or(""), source, target)?}),
            ),
            "tokenize" => Ok(
                json!({"tokens":self.tokenize(payload["text"].as_str().unwrap_or(""), source, target)?}),
            ),
            _ => Err("不支持的请求。".into()),
        }
    }
}

fn sbd_language(source: &str) -> &str {
    match source {
        "zh" => "zh-hans",
        "zt" => "zh-hant",
        "pb" => "pt",
        other => other,
    }
}

/// Cross-process advisory lock shared by inference and explicit model changes.
/// Only model-management metadata is written; translation text is never stored.
pub fn lock_models(root: &Path, exclusive: bool) -> Result<std::fs::File, String> {
    plain_directory(root)?;
    let directory = root.join(".manager");
    plain_directory(&directory)?;
    let path = directory.join("lock");
    reject_link(&path)?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| "模型目录不可写，请将 EXE 放到有写入权限的文件夹。")?;
    let result = if exclusive {
        fs2::FileExt::try_lock_exclusive(&file)
    } else {
        fs2::FileExt::try_lock_shared(&file)
    };
    result.map_err(|_| "模型正在被其他操作使用，请稍后重试。")?;
    Ok(file)
}

pub fn reject_link(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            #[cfg(windows)]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let linked = metadata.file_type().is_symlink();
            if linked {
                return Err("模型管理不支持符号链接或目录联接。".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("无法检查模型目录。".into()),
    }
    Ok(())
}

pub fn plain_directory(path: &Path) -> Result<(), String> {
    reject_link(path)?;
    std::fs::create_dir_all(path)
        .map_err(|_| "模型目录不可写，请将 EXE 放到有写入权限的文件夹。".to_owned())
}
/// Explicit command-line mode for offline integration checks; no transcript files.
pub fn run_stdio(model_root: PathBuf) {
    let mut engine = Engine::new(model_root);
    let mut output = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let now = Instant::now();
        let response = serde_json::from_str(&line)
            .map_err(|_| "请求 JSON 无效".to_string())
            .and_then(|p| engine.request(&p));
        let value = match response {
            Ok(data) => json!({"ok":true,"data":data,"elapsed":now.elapsed().as_secs_f64()}),
            Err(error) => json!({"ok":false,"error":error,"elapsed":now.elapsed().as_secs_f64()}),
        };
        if writeln!(output, "{value}")
            .and_then(|_| output.flush())
            .is_err()
        {
            break;
        }
    }
}
