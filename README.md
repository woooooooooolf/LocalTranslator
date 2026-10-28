# 静译 · Jingyi Translate

Windows x64 便携式 CPU 离线翻译工具。Tauri 2 桌面界面直接调用随包 Python/Argos 翻译进程，保留原工具的模型和文档处理能力。原始工具目录只作为复制来源，不作修改。

## 使用便携版

打开 `release/Jingyi/静译.exe`。整个 `Jingyi` 文件夹可以复制到其他位置；请保留 `runtime` 和 `backend`，不要单独移动 EXE。程序目录需要可写，避免放入 Program Files。目标平台为 Windows 10/11 x64，需已安装 Microsoft Edge WebView2 Runtime；本包不附带 WebView2 安装器，缺少该运行时的完全离线电脑需预先离线部署。

0.2.0 支持中英双向文字翻译、交换语言、复制、保存 UTF-8 文本与 Ctrl+Enter 快捷键。文字上限 20,000 字符。深浅主题可从侧栏切换，启动时跟随系统、不记录主题偏好；编辑框随窗口尺寸扩展。文本框保留右键编辑菜单，其他页面区域不显示右键菜单。翻译质量沿用 Argos 1.9 模型。

文档支持 TXT / DOCX / PPTX / ODT / ODP / EPUB / HTML / SRT / PDF，单文件上限 50 MB。选择文档后点击“翻译文档”，在系统对话框选择译文位置。程序先处理临时副本，成功后保存译文；禁止覆盖源文件。TXT 支持 UTF-8 与 GB18030 输入，输出 UTF-8。PDF 仅处理文字层，不含 OCR；文档复杂排版、图表、字体及分页可能变化，需自行检查成品。

## 本地数据与隐私边界

- Python 进程的网络连接及 DNS 解析由审计钩子拒绝；不启动 HTTP 服务、不监听端口、不自动下载或更新模型、不使用外部 CDN。
- 前端通过 Tauri IPC 调用固定命令；不授予前端通用 shell、文件系统或网络权限。
- 模型在 `runtime/packages`，中英文 MiniSBD 分句模型在 `runtime/assets`，便携 Python 在 `runtime/py`。
- 文字原文、译文、所选文档路径只保留在内存中；不使用 localStorage、sessionStorage、IndexedDB、Cookie 或草稿/历史文件存储这些内容。没有自动剪贴板写入，只有点击复制或保存时才执行对应操作。
- WebView2 使用 InPrivate 与每次启动独立的 `data/sessions/session-*` 目录。关闭常规自动填充和密码保存，并在启动时通过原生接口读回验证；任一设置不生效就不继续启动。文本框同时关闭 autocomplete、自动纠错和拼写检查。
- 引擎 stderr 不写文件，错误不记录到 `engine.log`。关闭页面时清空表单与引用；正常退出先结束引擎，再释放 WebView 并删除会话目录。
- 用户主动进行文档翻译时，第三方格式库需要会话内的临时原稿/译文副本；成功后仅将译文写入用户明确选择的位置。正常退出清理临时文件，异常中断残留会在下次启动检查后清理；不将删除称为安全擦除。
- 多实例通过文件锁保护各自会话，不清理其他活动实例；未带本应用会话标记的文件夹不会被删除。旧版持久化 WebView 目录不再复用，本次本机旧便携包的 WebView 缓存和引擎日志已清理。
- 正常退出时终止翻译进程并释放模型；翻译进程监测桌面父进程退出。单次任务最长等待 10 分钟，超时后可重试。
- 没有安装器、注册表设置或全局 PATH 修改。以上措施约束应用自动保存行为，不代表可阻止 Windows 内存换页、系统崩溃转储或用户主动导出/复制。WebView2 是系统组件，其自身更新和系统行为不由本应用控制。

## 开发

需要 Node.js 22.12+（或 Vite 支持的版本）、Rust stable MSVC 和 Visual Studio C++ Build Tools / Windows SDK。

```powershell
npm ci
npm run prepare:runtime
npm run desktop
```

其他来源路径：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-runtime.ps1 -Source 'D:\你的便携翻译工具'
```

准备脚本复制已有 Python、模型和 MiniSBD 缓存，保留原有第三方文件与许可证，不复制原用户目录或历史缓存，并跳过 `.pyc` 字节码缓存。首版保留完整 Python 依赖以降低兼容性风险，便携包约 1.2 GB；尚未对 Torch、SpaCy、LibreTranslate 等未使用组件做裁剪。

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --bin local-translate
runtime/py/python.exe -B -X utf8 scripts/smoke-test.py
runtime/py/python.exe -B -X utf8 scripts/check-process.py
runtime/py/python.exe -B -X utf8 scripts/check-worker-privacy.py
npm run portable
```

`npm run portable` 生成 `release/Jingyi`，运行时不需要系统 Python、Node.js 或 Rust。源码及锁文件纳入 Git，`runtime`、`release`、`data`、依赖缓存和构建输出被忽略。仓库初始化为 `main`，未配置远程地址。

迭代完成并验证后，按功能粒度创建本地提交；不会自动推送、打标签或发布。当前作者信息仅配置在本仓库。

## 架构与评估

```text
Tauri WebView2 界面 → 固定 Rust commands → stdin/stdout JSONL → Argos / CTranslate2 CPU
                                                      └→ Argos Translate Files
```

采用桌面 UI 加本地子进程的方式可行，适合现有 Python 模型生态。相比嵌入原网页，避免了 HTTP 服务、端口冲突与后台服务器管理，能定制桌面交互。Tauri 只减小桌面壳的开销，不能消除模型和 Python 依赖体积；也不会提升原模型的翻译质量或修复所有第三方文档格式缺陷。

当前实现支持已经安装的翻译方向，不包含自动识别语言、联网语言市场、自动更新、历史记录或 OCR。新增模型可手动放入 `runtime/packages`；需要同时提供该语言的离线 MiniSBD 资源，缺失资源会报错而不会联网补下载。重启后刷新语言列表。

官方模型下载与“单 EXE + 外置模型”可以进一步实现，但当前除模型外仍附带 Python 与 `backend/worker.py`，不能只保留 EXE 运行。本轮按用户决定保留现有推理层；原生推理、文档处理迁移与模型下载作为后续专项。详见 [模型下载与单 EXE 评估](docs/MODEL_DEPLOYMENT.md)。

官方参考：[Tauri Rust commands](https://v2.tauri.app/develop/calling-rust/)、[Windows 与 WebView2 部署](https://v2.tauri.app/distribute/windows-installer/)。第三方组件与模型署名见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
