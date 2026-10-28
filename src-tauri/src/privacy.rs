use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};

/// One lock-protected scratch directory per application process. No document or
/// browser state is stored in the persistent model directories.
pub struct Session {
    directory: Option<tempfile::TempDir>,
    lock: Option<File>,
}
impl Session {
    pub fn new(parent: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(parent)?;
        // Recover only directories owned by this app, never a live instance.
        for entry in fs::read_dir(parent)?.flatten() {
            let path = entry.path();
            if !entry.file_type()?.is_dir()
                || !entry.file_name().to_string_lossy().starts_with("session-")
            {
                continue;
            }
            if let Ok(lock) = OpenOptions::new()
                .read(true)
                .write(true)
                .open(path.join(".jingyi-session"))
            {
                if lock.try_lock_exclusive().is_ok() {
                    drop(lock);
                    if fs::remove_dir_all(&path).is_err() && path.exists() {
                        let _ = fs::write(path.join(".jingyi-session"), []);
                    }
                }
            }
        }
        let directory = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(parent)?;
        let lock = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(directory.path().join(".jingyi-session"))?;
        lock.try_lock_exclusive()?;
        Ok(Self {
            directory: Some(directory),
            lock: Some(lock),
        })
    }
    pub fn path(&self) -> PathBuf {
        self.directory.as_ref().unwrap().path().to_path_buf()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        drop(self.lock.take());
        if let Some(directory) = self.directory.take() {
            let path = directory.keep();
            // WebView2 processes can release profile handles shortly after Close.
            for _ in 0..30 {
                if fs::remove_dir_all(&path).is_ok() || !path.exists() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            // A surviving folder has the ownership marker and is retried on startup.
            let _ = fs::write(path.join(".jingyi-session"), []);
        }
    }
}

/// Configure and read back the actual WebView2 settings. Initialization fails
/// closed if the installed runtime cannot provide the requested privacy mode.
#[cfg(windows)]
pub fn configure(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Settings4, ICoreWebView2_13};
    use windows::core::{Interface, BOOL};
    let (sender, receiver) = std::sync::mpsc::channel();
    window.with_webview(move |platform| {
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let webview = platform.controller().CoreWebView2()?;
                let settings = webview.Settings()?.cast::<ICoreWebView2Settings4>()?;
                settings.SetIsGeneralAutofillEnabled(false)?;
                settings.SetIsPasswordAutosaveEnabled(false)?;
                let (mut autofill, mut passwords, mut private) =
                    (BOOL::default(), BOOL::default(), BOOL::default());
                settings.IsGeneralAutofillEnabled(&mut autofill)?;
                settings.IsPasswordAutosaveEnabled(&mut passwords)?;
                webview
                    .cast::<ICoreWebView2_13>()?
                    .Profile()?
                    .IsInPrivateModeEnabled(&mut private)?;
                if autofill.as_bool() || passwords.as_bool() || !private.as_bool() {
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        0x80004005u32 as i32,
                    )));
                }
            }
            Ok(())
        })();
        let _ = sender.send(result.map_err(|_| "WebView2 隐私设置未生效。"));
    })?;
    receiver.recv_timeout(Duration::from_secs(10))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_sessions_survive_and_finished_sessions_are_removed() {
        let parent = tempfile::tempdir().unwrap();
        let first = Session::new(parent.path()).unwrap();
        let path = first.path();
        fs::write(path.join("sensitive.txt"), b"private test marker").unwrap();
        let second = Session::new(parent.path()).unwrap();
        assert!(path.join("sensitive.txt").exists());
        drop(first);
        assert!(!path.exists());
        drop(second);
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }
    #[test]
    fn abandoned_owned_sessions_are_removed_without_touching_other_files() {
        let parent = tempfile::tempdir().unwrap();
        let stale = parent.path().join("session-abandoned");
        let unrelated = parent.path().join("session-user-folder");
        fs::create_dir_all(&stale).unwrap();
        fs::create_dir_all(&unrelated).unwrap();
        fs::write(stale.join(".jingyi-session"), []).unwrap();
        fs::write(stale.join("document.txt"), b"private test marker").unwrap();
        let _live = Session::new(parent.path()).unwrap();
        assert!(!stale.exists());
        assert!(unrelated.exists());
    }
}
