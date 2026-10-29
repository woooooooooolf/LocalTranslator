# 静译 · Jingyi Translate

0.4.0 是 Windows x64 原生 CPU 离线文字翻译工具。Tauri 界面、CTranslate2 和 ONNX Runtime 均编入 EXE；只需分发 EXE，首次运行可按需下载模型，不包含 Python、程序 DLL 或需解压运行的程序附件。

## 运行

本次更新位于 `release/Jingyi-Native-0.4.0/静译.exe`。将 EXE 放到可写目录，联网运行，在“本地模型”下载需要的方向，完成后可断网翻译。也可保留旧版 EXE 旁的 `models` 直接使用。

下载后目录结构：

```text
Jingyi-Native/
├─ 静译.exe
└─ models/
   ├─ packages/    # 翻译权重、SentencePiece 模型、配置与模型 README
   └─ minisbd/     # en.onnx、zh-hans.onnx
```

应用仍需要系统 **WebView2 Runtime**。当前构建也使用 Windows 的 DXCore 系统 API，目标为 Windows 11 x64 或 Windows 10 2004+；[Microsoft 的 API 要求](https://learn.microsoft.com/en-us/windows/win32/api/dxcore/nf-dxcore-dxcorecreateadapterfactory) 为 Build 18936+。没有附带 WebView2 安装器，完全离线机器需预先准备系统组件。运行不需要 Python、Node.js、Rust、CUDA 或 VC++ 附带 DLL。

程序目录需可写。运行时会创建 `data/sessions/session-*` 作为临时 WebView2 会话目录，正常退出清理；仅在父目录为空时移除 `data`。不要放入只读位置。第三方说明内置于“关于”，无需旁挂说明文件。

分发只需 EXE；为离线电脑准备时可另附 `models/packages` 与 `models/minisbd`。不要把运行中的 `data` 或 `models/.manager` 临时管理目录打包。旧版的 `runtime`、`backend` 不需要放在原生版旁边。

## 使用

- 当前模型为 Argos 中英双向 1.9。输入最多 20,000 字符，Ctrl+Enter 翻译，支持交换语言和手动复制。文字译文的“保存为文本”功能已移除。
- “本地模型”分别提供英→中（约 67.6 MiB）和中→英（约 71.6 MiB）的下载、进度、取消、刷新与删除；不后台联网。删除需要一次应用内确认，删除后可重新下载，无需重启。少于 1 MB 的共享 MiniSBD 数据保留，避免影响手动安装的同源语言模型。
- 侧栏切换深浅主题，启动时跟随系统，不记录主题偏好；输入输出区域随窗口扩展。
- 页面文本框保留右键编辑菜单，其他页面区域不显示右键菜单。
- 文档翻译、文件选择和文件保存已按用户要求全部移除；不包含 PDF、OCR 或文档字体组件。

工具只处理输入框中的文字。需要带走译文时，可主动复制，或选中后使用 Ctrl+C。

模型权重未改变，但 CPU 后端从原 Python/MKL 改为 Ruy，译文可能出现措辞、虚词、标点与个别质量差异。当前测试中分词和分句与原版一致，译文不是逐字兼容。没有宣称提升模型质量。

## 隐私行为

- 原文和译文只用于当前操作，不写草稿、翻译历史、浏览器存储或内容日志；原生层只缓存模型，不缓存翻译文本。只有主动复制才写入剪贴板。
- WebView2 使用独立临时目录与 InPrivate，原生关闭常规自动填充、密码保存，并读回验证；文本框关闭 autocomplete、自动纠错与拼写检查。
- 没有文件对话框或译文文件输出命令；CLI 文件翻译请求也被拒绝。不存在清理对话框状态失败的路径，不更改其他软件的最近文件记录或系统全局偏好。
- 不启动 HTTP 服务，不监听翻译端口，不引用 CDN，不自动下载模型。只有主动点击模型下载会通过原生 HTTP 客户端访问官方模型源，发送固定权重请求，不传入原文、译文或本地路径；页面 CSP 不开放互联网请求。CPU 原生推理路径不调用联网翻译服务；ONNX Runtime 构建关闭遥测，运行时也禁用遥测。
- 模型完整包与所选文件均校验内置 SHA-256；临时下载、校验、受限解包和实际加载验证成功后才发布安装目录。失败或取消不启用半成品，异常退出的模型临时文件在下次模型操作时清理。`models/.manager` 只存锁、版本戳和临时权重，不存翻译内容。模型管理期间暂停翻译，多实例使用文件锁协调。
- 退出清理属于本应用的临时会话；文件锁保护其他活动实例。异常中断或句柄暂未释放时，残留会在下次启动重试清理，不将删除称为安全擦除。

这些措施约束应用自动保存行为，不表示能阻止 Windows 换页、系统转储、系统组件自身行为或用户主动导出/复制。旧 0.2.0 的 `release/Jingyi` 保留为对照，不再作为原生版交付目录。

## 构建与验证

开发构建需要 Node.js、Rust MSVC、Visual Studio C++ Build Tools、Windows SDK、CMake 3.28+、Ninja 和 Python 3。Python 仅是构建/对照测试工具，不是应用运行依赖。

```powershell
npm ci
npm run native:core
npm run native:onnx
npm run portable
```

`prepare:models` 从当前工程保留的旧 `runtime` 复制模型；不会改动原始工具目录。构建源码固定到上游提交，静态使用 VC++ 运行库，不下载运行时 DLL 作为旁挂文件。`native:onnx` 的首次构建耗时较长。

打包默认输出 `release/Jingyi-Native-<版本>` 内的单一 EXE，避免覆盖正在使用的旧版。需要预附离线模型时，先 `npm run prepare:models`，再执行 `npm run portable -- -IncludeModels`。HTTP/TLS 使用 Windows 系统组件，不附带 OpenSSL 等程序 DLL。

```powershell
cargo test --manifest-path crates/engine/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --bin local-translate
cargo test --manifest-path src-tauri/Cargo.toml --bin local-translate cached_install_translate_remove -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --bin local-translate official_download_install_translate_remove -- --ignored --nocapture
runtime/py/python.exe -B -X utf8 scripts/check-native-engine.py --exe <已安装双向模型的EXE路径>
```

最后一项使用保留的 Python 环境进行合成样本对照；实际被测程序是原生 EXE。`--stdio` 是显式的开发验证入口，通过 stdin/stdout JSONL 工作，不创建翻译记录。

缓存测试需要先由维护脚本 `python scripts/fetch-model-catalog.py` 获取官方权重；真实联网测试会下载约 139 MiB 数据，使用独立测试目录，不改已有模型。维护脚本会重建内置清单，模型版本更新时应人工审查提交，应用本身不下载远程清单。

源码与锁文件纳入 Git，模型、旧运行时、依赖缓存和编译产物忽略。每次验证后的功能批次建立本地提交，不自动推送、打标签或发布。详见 [原生迁移记录](docs/NATIVE_MIGRATION.md)、[验证记录](VALIDATION.md)、[模型下载实现](docs/MODEL_DEPLOYMENT.md) 和 [第三方组件](THIRD_PARTY_NOTICES.md)。
