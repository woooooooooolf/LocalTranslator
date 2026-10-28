# 验证记录 · 2026-10-28

## 已通过

- Node 24.14.1、Rust 1.99.1 / MSVC；TypeScript 与 Vite 正式构建成功，Cargo check 成功。
- Tauri Windows x64 release 构建成功，桌面壳约 9.2 MB；运行时、模型独立附带。
- 从当前工程复制后的便携 Python 运行真实 Argos 模型，中译英、英译中、换行文字均产出译文。
- TXT、SRT、DOCX、文字层 PDF 实际翻译成功；检查源文件 SHA-256 不变、字幕时间轴保留、DOCX 内未修改的二进制条目保留、PDF 含中文文字。
- 空文字、超长文字、同语言、覆盖源文件被拒绝；没有文字层的 PDF 提示先做 OCR。
- 运行审计钩子，验证 DNS 解析与 socket 连接在网络 I/O 前被拒绝。未通过修改系统防火墙或断开系统网络测试。
- stdin EOF 后引擎退出；模拟桌面父进程消失且保留 stdin 时，翻译进程仍自动退出。
- 正式便携包在 Windows 正常桌面成功启动，可访问性树显示“离线引擎已就绪”。在实际 EXE 输入“你好，世界！这是一个离线翻译工具。”并使用 Ctrl+Enter，约 1 秒产出 `Hello, world! This is an offline translation tool.`。
- 用户确认原生桌面窗口可见、显示正常。同套前端在浏览器预览中也已视觉检查双栏、侧栏和控件排版。

## 验证边界

- 自动化截图服务未能捕获原生窗口（FrameArrived / window capture timeout）；应用可访问性读取、输入和快捷键翻译成功。原生控件点击也受到自动化几何信息不可用的限制，因此未完成系统保存对话框的完整自动化回归。
- 文档集成检查使用小型合成样本，不代表所有复杂文档的排版质量或全部九种格式均已逐项验收。
- 未在另一台电脑、无 WebView2 的机器、Windows 10 或不同 CPU 型号上实际运行；未进行代码签名、安装器发布或全系统文件/网络审计。
- 原运行环境的 Requests 在导入时会产生依赖版本组合警告；本应用未使用 Requests 发起翻译请求，真实翻译与文档检查已通过。首版保留原依赖，未作未经验证的升级或裁剪。

可重复运行：`scripts/smoke-test.py`、`scripts/check-process.py`；需使用随包 `runtime/py/python.exe -B -X utf8`。
