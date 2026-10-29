use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
    ptr,
};
unsafe extern "C" {
    fn jy_model_new(path: *const c_char, handle: *mut *mut c_void, error: *mut *mut c_char) -> i32;
    fn jy_translate(
        handle: *mut c_void,
        input: *const c_char,
        output: *mut *mut c_char,
        error: *mut *mut c_char,
    ) -> i32;
    fn jy_model_delete(handle: *mut c_void);
    fn jy_free(text: *mut c_char);
}
pub struct Model(*mut c_void);
// A model is exclusively owned and only used through &mut self, under Engine's mutex.
unsafe impl Send for Model {}
unsafe fn take_string(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let text = CStr::from_ptr(raw).to_string_lossy().into_owned();
    jy_free(raw);
    text
}
impl Model {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "模型路径无效")?;
        let (mut handle, mut error) = (ptr::null_mut(), ptr::null_mut());
        let status = unsafe { jy_model_new(path.as_ptr(), &mut handle, &mut error) };
        if status != 0 {
            return Err(unsafe { take_string(error) });
        }
        if handle.is_null() {
            return Err("原生模型句柄为空".into());
        }
        Ok(Self(handle))
    }
    pub fn translate(&mut self, tokens: &[Vec<String>]) -> Result<Vec<String>, String> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let json = CString::new(serde_json::to_vec(tokens).map_err(|_| "无法编码翻译请求")?)
            .map_err(|_| "翻译请求无效")?;
        let (mut output, mut error) = (ptr::null_mut(), ptr::null_mut());
        let status = unsafe { jy_translate(self.0, json.as_ptr(), &mut output, &mut error) };
        if status != 0 {
            return Err(unsafe { take_string(error) });
        }
        let json = unsafe { take_string(output) };
        serde_json::from_str(&json).map_err(|_| "原生翻译返回无效结果".into())
    }
}
impl Drop for Model {
    fn drop(&mut self) {
        unsafe { jy_model_delete(self.0) }
    }
}
