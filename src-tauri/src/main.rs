#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod privacy;

use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::Mutex, time::Instant};
use tauri::{Manager, State};

struct Engine {
    root: PathBuf,
    inner: Mutex<jingyi_engine::Engine>,
}
impl Engine {
    fn request(&self, payload: Value) -> Result<Value, String> {
        let started = Instant::now();
        let data = self
            .inner
            .lock()
            .map_err(|_| "翻译引擎状态异常")?
            .request(&payload)?;
        Ok(json!({"data":data,"elapsed":started.elapsed().as_secs_f64()}))
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
#[tauri::command]
fn runtime_location(engine: State<'_, Engine>) -> String {
    engine.root.to_string_lossy().into_owned()
}

#[tauri::command]
fn third_party_notices() -> &'static str {
    include_str!("../third-party-notices.txt")
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
    #[cfg(debug_assertions)]
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    #[cfg(not(debug_assertions))]
    let root = std::env::current_exe()
        .expect("无法定位程序")
        .parent()
        .unwrap()
        .to_path_buf();
    if std::env::args().any(|argument| argument == "--stdio") {
        jingyi_engine::run_stdio(root.join("models"));
        return;
    }
    let session = privacy::Session::new(&root.join("data/sessions")).expect("无法创建临时会话");
    let cleanup_root = root.clone();
    let session_path = session.path();
    let app = tauri::Builder::default()
        .setup(move |app| {
            app.manage(Engine {
                root: root.clone(),
                inner: Mutex::new(jingyi_engine::Engine::new(root.join("models"))),
            });
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("静译 · 离线翻译")
            .inner_size(1180.0, 800.0)
            .min_inner_size(900.0, 650.0)
            .data_directory(session_path.join("webview"))
            .incognito(true)
            .general_autofill_enabled(false)
            .additional_browser_args("--disable-background-networking --disable-component-update --disable-breakpad --disable-crash-reporter")
            .build()?;
            #[cfg(windows)]
            privacy::configure(&window)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_info,
            translate_text,
            runtime_location,
            third_party_notices,
            set_appearance
        ])
        .build(tauri::generate_context!())
        .expect("无法启动静译");
    let exit_code = app.run_return(|_, _| {});
    drop(session);
    // Remove only empty parent directories; other running instances or unrelated
    // files make these calls fail harmlessly.
    let _ = fs::remove_dir(cleanup_root.join("data/sessions"));
    let _ = fs::remove_dir(cleanup_root.join("data"));
    std::process::exit(exit_code);
}
