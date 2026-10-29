//! Windows shell dialogs with app-scoped history disabled. These functions do
//! not change global Explorer settings or erase another application's history.
use std::path::PathBuf;
use windows::{
    core::{GUID, HSTRING, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::Shell::{
            Common::COMDLG_FILTERSPEC, FileOpenDialog, FileSaveDialog, IFileDialog,
            FOS_DONTADDTORECENT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT,
            FOS_PATHMUSTEXIST, SIGDN_FILESYSPATH,
        },
    },
};

const CLIENT: GUID = GUID::from_u128(0xc2cf6e83_6983_465e_9a67_0fdc1398a984);
struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct PrivateDialog(IFileDialog);
impl Drop for PrivateDialog {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ClearClientData();
        }
    }
}

fn show(parent: usize, save: Option<(String, String)>) -> Result<Option<PathBuf>, String> {
    std::thread::spawn(move || unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|_| "无法初始化文件对话框")?;
        let _com = Com;
        let class = if save.is_some() {
            &FileSaveDialog
        } else {
            &FileOpenDialog
        };
        let dialog: IFileDialog = CoCreateInstance(class, None, CLSCTX_INPROC_SERVER)
            .map_err(|_| "无法创建文件对话框")?;
        dialog
            .SetClientGuid(&CLIENT)
            .map_err(|_| "无法隔离文件对话框状态")?;
        let dialog = PrivateDialog(dialog);
        dialog
            .0
            .ClearClientData()
            .map_err(|_| "无法清理应用文件对话框状态")?;
        let flags = dialog
            .0
            .GetOptions()
            .map_err(|_| "无法读取文件对话框设置")?;
        let extra = FOS_DONTADDTORECENT
            | FOS_FORCEFILESYSTEM
            | FOS_PATHMUSTEXIST
            | if save.is_some() {
                FOS_OVERWRITEPROMPT
            } else {
                FOS_FILEMUSTEXIST
            };
        dialog
            .0
            .SetOptions(flags | extra)
            .map_err(|_| "无法关闭文件路径历史")?;
        if !dialog
            .0
            .GetOptions()
            .map_err(|_| "无法核实文件对话框设置")?
            .contains(FOS_DONTADDTORECENT)
        {
            return Err("文件路径历史设置未生效".into());
        }
        let (label, pattern, filename, extension) = if let Some((name, extension)) = save {
            (
                HSTRING::from("译文"),
                HSTRING::from(format!("*.{extension}")),
                Some(HSTRING::from(name)),
                Some(HSTRING::from(extension)),
            )
        } else {
            (
                HSTRING::from("文档"),
                HSTRING::from(
                    jingyi_engine::FORMATS
                        .iter()
                        .map(|e| format!("*.{e}"))
                        .collect::<Vec<_>>()
                        .join(";"),
                ),
                None,
                None,
            )
        };
        dialog
            .0
            .SetFileTypes(&[COMDLG_FILTERSPEC {
                pszName: PCWSTR(label.as_ptr()),
                pszSpec: PCWSTR(pattern.as_ptr()),
            }])
            .map_err(|_| "无法设置文档格式")?;
        if let Some(name) = filename {
            dialog
                .0
                .SetFileName(&name)
                .map_err(|_| "无法设置译文文件名")?;
        }
        if let Some(extension) = extension {
            dialog
                .0
                .SetDefaultExtension(&extension)
                .map_err(|_| "无法设置译文扩展名")?;
        }
        let result = dialog.0.Show(Some(HWND(parent as *mut std::ffi::c_void)));
        let output = match result {
            Ok(()) => {
                let item = dialog.0.GetResult().map_err(|_| "无法读取选择的文件")?;
                let path = item
                    .GetDisplayName(SIGDN_FILESYSPATH)
                    .map_err(|_| "无法读取文件路径")?;
                let value = path.to_string().map_err(|_| "文件路径不是有效 Unicode");
                CoTaskMemFree(Some(path.0.cast()));
                Some(PathBuf::from(value?))
            }
            Err(error) if error.code().0 == 0x800704c7u32 as i32 => None,
            Err(_) => return Err("文件对话框操作失败".into()),
        };
        dialog
            .0
            .ClearClientData()
            .map_err(|_| "无法清理应用文件对话框状态")?;
        Ok(output)
    })
    .join()
    .map_err(|_| "文件对话框线程异常".to_string())?
}
pub fn open(parent: usize) -> Result<Option<PathBuf>, String> {
    show(parent, None)
}
pub fn save(parent: usize, name: String, extension: String) -> Result<Option<PathBuf>, String> {
    show(parent, Some((name, extension)))
}
