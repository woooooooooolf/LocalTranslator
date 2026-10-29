# 静译 · Jingyi Translate

0.3.0 是 Windows x64 原生 CPU 离线翻译工具。Tauri 界面、Rust 文档处理、CTranslate2 和 ONNX Runtime 均编入 EXE；外置文件仅为模型，不包含 Python、程序 DLL 或需解压运行的程序附件。

## 运行

打开 `release/Jingyi-Native/静译.exe`，保留旁边的 `models`：

```text
Jingyi-Native/
├─ 静译.exe
└─ models/
   ├─ packages/    # 翻译权重、SentencePiece 模型、配置与模型 README
   └─ minisbd/     # en.onnx、zh-hans.onnx
```

应用仍需要系统 **WebView2 Runtime**。当前构建也使用 Windows 的 DXCore 系统 API，目标为 Windows 11 x64 或 Windows 10 2004+；[Microsoft 的 API 要求](https://learn.microsoft.com/en-us/windows/win32/api/dxcore/nf-dxcore-dxcorecreateadapterfactory) 为 Build 18936+。没有附带 WebView2 安装器，完全离线机器需预先准备系统组件。运行不需要 Python、Node.js、Rust、CUDA 或 VC++ 附带 DLL。

程序目录需可写。运行时会创建 `data/sessions/session-*` 作为临时 WebView2 会话目录，正常退出清理；仅在父目录为空时移除 `data`。不要放入只读位置。第三方说明内置于“关于”，无需旁挂说明文件。

分发或更新时只取 EXE 与模型，不把运行中的 `data` 目录打包。旧版的 `runtime`、`backend` 不需要放在原生版旁边。

## 使用

- 当前模型为 Argos 中英双向 1.9。输入最多 20,000 字符，Ctrl+Enter 翻译，支持交换语言、手动复制和保存 UTF-8 译文。
- 侧栏切换深浅主题，启动时跟随系统，不记录主题偏好；输入输出区域随窗口扩展。
- 页面文本框保留右键编辑菜单，其他页面区域不显示右键菜单。
- 文档支持 TXT、SRT、HTML、DOCX、PPTX、ODT、ODP、EPUB，单文件最多 50 MB。用户明确选择译文位置后才输出，不能覆盖原稿。
- **PDF 翻译已按用户要求移除**，不包含 PDF、OCR 或文档字体组件。

TXT 支持 UTF-8 和 GB18030 输入，译文使用 UTF-8。容器文档保留未翻译资源、段落结构与主要标记；同段落跨 run 的译文合入首个文本 run，因此局部字符样式、排版与字段效果需检查。该工具不作高保真文档排版承诺。

模型权重未改变，但 CPU 后端从原 Python/MKL 改为 Ruy，译文可能出现措辞、虚词、标点与个别质量差异。当前测试中分词和分句与原版一致，译文不是逐字兼容。没有宣称提升模型质量。

## 隐私行为

- 原文、译文和所选路径只用于当前操作，不写草稿、翻译历史、浏览器存储或内容日志；原生层只缓存模型，不缓存翻译文本。只有主动复制或保存才产生对应输出。
- WebView2 使用独立临时目录与 InPrivate，原生关闭常规自动填充、密码保存，并读回验证；文本框关闭 autocomplete、自动纠错与拼写检查。
- 文件对话框设置 `FOS_DONTADDTORECENT`，用本应用专有 GUID 隔离状态，并在操作后清理该状态；不清除其他软件的记录或改变系统全局偏好。
- 文档原稿直接在内存中读取和转换。文档译文全部生成后，在用户选择的目录内通过临时文件完成原子保存；该显式导出流程会写入译文，不属于自动保存历史。
- 不启动 HTTP 服务，不监听翻译端口，不引用 CDN，不自动下载模型。CPU 原生推理路径不调用联网翻译服务；ONNX Runtime 构建关闭遥测，运行时也禁用遥测。
- 退出清理属于本应用的临时会话；文件锁保护其他活动实例。异常中断或句柄暂未释放时，残留会在下次启动重试清理，不将删除称为安全擦除。

这些措施约束应用自动保存行为，不表示能阻止 Windows 换页、系统转储、系统组件自身行为或用户主动导出/复制。旧 0.2.0 的 `release/Jingyi` 保留为对照，不再作为原生版交付目录。

## 构建与验证

开发构建需要 Node.js、Rust MSVC、Visual Studio C++ Build Tools、Windows SDK、CMake 3.28+、Ninja 和 Python 3。Python 仅是构建/对照测试工具，不是应用运行依赖。

```powershell
npm ci
npm run native:core
npm run native:onnx
npm run prepare:models
npm run portable
```

`prepare:models` 从当前工程保留的旧 `runtime` 复制模型；不会改动原始工具目录。构建源码固定到上游提交，静态使用 VC++ 运行库，不下载运行时 DLL 作为旁挂文件。`native:onnx` 的首次构建耗时较长。

```powershell
cargo test --manifest-path crates/engine/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --bin local-translate
runtime/py/python.exe -B -X utf8 scripts/check-native-engine.py --exe release/Jingyi-Native/静译.exe
```

最后一项使用保留的 Python 环境进行合成样本对照；实际被测程序是原生 EXE。`--stdio` 是显式的开发验证入口，通过 stdin/stdout JSONL 工作，不创建翻译记录。

源码与锁文件纳入 Git，模型、旧运行时、依赖缓存和编译产物忽略。每次验证后的功能批次建立本地提交，不自动推送、打标签或发布。详见 [原生迁移记录](docs/NATIVE_MIGRATION.md)、[验证记录](VALIDATION.md)、[模型下载评估](docs/MODEL_DEPLOYMENT.md) 和 [第三方组件](THIRD_PARTY_NOTICES.md)。在线模型下载仍是独立的后续功能，当前没有下载按钮。
