#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Mutex},
    time::Duration,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

struct WorkerIo {
    input: ChildStdin,
    output: mpsc::Receiver<String>,
}
struct Engine {
    root: PathBuf,
    child: Mutex<Option<Child>>,
    io: Mutex<Option<WorkerIo>>,
}
impl Engine {
    fn stop(&self) {
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    fn start(&self) -> Result<WorkerIo, String> {
        let python = self.root.join("runtime/py/python.exe");
        if !python.is_file() || !self.root.join("backend/worker.py").is_file() {
            return Err("运行时缺失。请保留 EXE 旁的 runtime 和 backend 文件夹；开发环境请先执行 npm run prepare:runtime。".into());
        }
        let data = self.root.join("data");
        fs::create_dir_all(&data).map_err(|e| format!("程序目录不可写：{e}"))?;
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(data.join("engine.log"))
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(python);
        command
            .args(["-u", "-B", "-X", "utf8"])
            .arg(self.root.join("backend/worker.py"))
            .current_dir(&self.root)
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .env("PYTHONNOUSERSITE", "1")
            .env("JINGYI_PARENT_PID", std::process::id().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("启动翻译引擎失败：{e}"))?;
        let input = child.stdin.take().ok_or("无法打开翻译进程输入")?;
        let output = child.stdout.take().ok_or("无法打开翻译进程输出")?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else {
                    break;
                };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        *self.child.lock().map_err(|e| e.to_string())? = Some(child);
        Ok(WorkerIo {
            input,
            output: receiver,
        })
    }
    fn request(&self, payload: Value) -> Result<Value, String> {
        let mut slot = self.io.lock().map_err(|e| e.to_string())?;
        if slot.is_none() {
            *slot = Some(self.start()?);
        }
        let io = slot.as_mut().ok_or("引擎尚未就绪")?;
        let result = (|| {
            writeln!(io.input, "{payload}")
                .and_then(|_| io.input.flush())
                .map_err(|e| e.to_string())?;
            let line = io
                .output
                .recv_timeout(Duration::from_secs(600))
                .map_err(|_| {
                    "引擎未响应或已退出。请重试；详细信息见 data/engine.log。".to_string()
                })?;
            serde_json::from_str::<Value>(&line).map_err(|e| format!("引擎响应无效：{e}"))
        })();
        match result {
            Ok(response) if response["ok"] == true => Ok(json!({
                "data": response["data"], "elapsed": response["elapsed"]
            })),
            Ok(response) => Err(response["error"].as_str().unwrap_or("翻译失败").into()),
            Err(error) => {
                self.stop();
                *slot = None;
                Err(error)
            }
        }
    }
}

async fn call(app: tauri::AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Engine>().request(payload))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn engine_info(app: tauri::AppHandle) -> Result<Value, String> {
    call(app, json!({"op": "info"})).await
}
#[tauri::command]
async fn translate_text(
    app: tauri::AppHandle,
    text: String,
    source: String,
    target: String,
) -> Result<Value, String> {
    call(
        app,
        json!({"op":"text", "text":text, "source":source, "target":target}),
    )
    .await
}
const FORMATS: &[&str] = &[
    "txt", "docx", "pptx", "odt", "odp", "epub", "html", "srt", "pdf",
];
#[tauri::command]
async fn pick_document(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("文档", FORMATS)
            .blocking_pick_file()
            .map(|p| {
                p.into_path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn translate_document(
    app: tauri::AppHandle,
    path: String,
    source: String,
    target: String,
) -> Result<Option<Value>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let input = Path::new(&path).canonicalize().map_err(|e| e.to_string())?;
        let extension = input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !FORMATS.contains(&extension.as_str()) {
            return Err("不支持此文档格式".into());
        }
        let filename = format!(
            "{}-{}.{}",
            input.file_stem().unwrap_or_default().to_string_lossy(),
            target,
            extension
        );
        let Some(output) = app
            .dialog()
            .file()
            .add_filter("译文", &[&extension])
            .set_file_name(filename)
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let output = output.into_path().map_err(|e| e.to_string())?;
        let result = app.state::<Engine>().request(json!({
            "op":"file", "path":input, "output":output, "source":source, "target":target
        }))?;
        Ok(Some(result))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn save_text(app: tauri::AppHandle, text: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("UTF-8 文字", &["txt"])
            .set_file_name("译文.txt")
            .blocking_save_file()
        else {
            return Ok(false);
        };
        fs::write(file.into_path().map_err(|e| e.to_string())?, text).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn runtime_location(engine: State<'_, Engine>) -> String {
    engine.root.to_string_lossy().into_owned()
}

#[tauri::command]
fn set_appearance(window: tauri::WebviewWindow, theme: String) -> Result<(), String> {
    let theme = match theme.as_str() {
        "dark" => tauri::Theme::Dark,
        "light" => tauri::Theme::Light,
        _ => return Err("不支持的主题".into()),
    };
    window
        .set_theme(Some(theme))
        .map_err(|_| "无法设置窗口主题".into())
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_path_buf();
            #[cfg(not(debug_assertions))]
            let root = std::env::current_exe()?.parent().unwrap().to_path_buf();
            fs::create_dir_all(root.join("data/webview"))?;
            app.manage(Engine {
                root: root.clone(),
                child: Mutex::new(None),
                io: Mutex::new(None),
            });
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("静译 · 离线翻译")
            .inner_size(1180.0, 800.0)
            .min_inner_size(900.0, 650.0)
            .data_directory(root.join("data/webview"))
            .additional_browser_args("--disable-background-networking --disable-component-update")
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_info,
            translate_text,
            pick_document,
            translate_document,
            save_text,
            runtime_location,
            set_appearance
        ])
        .build(tauri::generate_context!())
        .expect("无法启动静译");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<Engine>().stop();
        }
    });
}
