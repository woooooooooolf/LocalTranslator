# 模型下载与单 EXE 评估

核实日期：2026-10-28。本轮按用户确认，完成界面及隐私调整；原生化留作下一轮专项。此文为部署设计，当前 0.2.0 **尚未增加联网下载按钮**，正常启动和翻译仍不联网。

## 1. 官方来源与可行性

可以增加按需下载、自动校验和部署。现有模型来自 [Argos 官方模型索引](https://github.com/argosopentech/argospm-index)，[原始 JSON](https://raw.githubusercontent.com/argosopentech/argospm-index/main/index.json) 中当前两项的版本均为 1.9：

| 方向 | 索引给出的官方下载地址 | 本次 HEAD 检查 |
| --- | --- | --- |
| 英 → 中 | https://argos-net.com/v1/translate-en_zh-1_9.argosmodel | 200；70,743,021 字节 |
| 中 → 英 | https://argos-net.com/v1/translate-zh_en-1_9.argosmodel | 200；74,481,402 字节 |

还需提供英文 `en.onnx` 和中文 `zh-hans.onnx` 分句模型。原安装的 MiniSBD 库明确使用 [LibreTranslate/MiniSBD 官方发布](https://github.com/LibreTranslate/MiniSBD/releases/tag/v0.0.1)，例如 `https://github.com/LibreTranslate/MiniSBD/releases/download/v0.0.1/en.onnx`；本次该地址 HEAD 返回 200。

本次核实到的两项 Argos 索引都只提供 `argos-net.com` 链接，没有提供 Hugging Face / ModelScope 镜像地址。未将社区上传或来源不明的同名权重当成官方镜像。若将来采用镜像，应校验与受信任清单内的同一 SHA-256 完全一致。Hugging Face 上其他架构的翻译权重不能仅改路径就替代当前 CTranslate2 模型。

当前索引没有公布这两个压缩包的 SHA-256。HTTP 成功和 ETag 只能用于可用性判断，不能冒充受信任的文件校验值；本次未下载完整归档，因此也未验证归档完整性或新下载后的安装流程。

## 2. 建议的程序内流程

1. 本地检查模型。缺失时在“本地模型”页提供“下载中英模型”，显示来源、大小和目标目录；只在用户点击后联网。
2. Rust 下载组件仅允许清单内的 HTTPS 端点，检查重定向目标、连接/读取超时及最大下载大小，支持取消与失败重试。翻译进程继续禁止网络连接，不传输输入、译文、文件路径。
3. 发布时维护带版本、大小、受信任 SHA-256 的小型清单；模型先下载到 `.part`，完整校验后再解压。
4. 在临时目录解包，拒绝绝对路径、`..`、链接、异常文件数量和膨胀体积；校验 metadata 的语言/版本及 CTranslate2、SentencePiece、MiniSBD 所需文件。
5. 保留模型 README/署名；等待正在进行的翻译结束，完成原子安装或回滚，再重启引擎并刷新语言列表。
6. 下载完成后断网验证。下载中断、磁盘不足、校验失败和重启也不能留下“半安装”的可用模型标记。另保留导入离线模型包的途径。

这里的“自动部署”指点击下载后的流程自动完成，不是后台自动联网。

## 3. 当前体积分布

本机当前副本中：

| 内容 | 字节数 | 说明 |
| --- | ---: | --- |
| 中英两个翻译模型，解包后 | 173,817,314 | 约 174 MB |
| 两个 MiniSBD 模型 | 804,485 | 约 0.8 MB |
| Python 与依赖 | 1,012,973,041 | 约 1.01 GB；含尚未裁剪的 Torch/SpaCy 等 |

所以模型按需下载能使模型与程序更新分离，但当前架构即使不附带翻译模型，仍有约 1 GB 的运行时，不能达到仅分发小 EXE 的目标。

## 4. “一个 EXE，只有模型外置”

| 依赖 | 当前实现 | 下一轮目标与限制 |
| --- | --- | --- |
| 页面 HTML/CSS/JS | 已编入 EXE | 保持 |
| Rust/Tauri 主程序 | 单个 EXE | 保持 |
| Python / `.pyd` / 第三方 DLL | 外置 `runtime/py` | 用原生推理实现替代，不能只靠 Tauri 打包消失 |
| `backend/worker.py` | 外置 | 迁移为 EXE 内的原生模块 |
| 翻译及分句权重 | 外置 | 统一到 `models`，支持按需下载 |
| WebView2 | 系统运行时 | Tauri 在 Windows 上仍需要它，属于目标电脑的系统前提 |
| VC++ / 数学库运行时 | Python 包含多项动态依赖 | 原生构建需逐项静态链接或核实系统依赖，不能预先保证全都消除 |

建议下一轮保留现有 Argos 模型，直接使用 [CTranslate2 官方 C++ 库](https://github.com/OpenNMT/CTranslate2)、[SentencePiece](https://github.com/google/sentencepiece) 和所需的本地分句能力，评估静态链接至 Tauri EXE。官方提供 [C++ 源码构建及后端选项](https://opennmt.net/CTranslate2/installation.html)。先用现有模型对比中英、长段落和换行，再迁移文档处理；PDF/Word 等目前也依赖 Python，不能只替换文字推理就删除整个 Python。

该方案可以朝“应用文件只有 EXE 与模型”迁移，但需要实际构建、DLL 依赖检查与行为对比后才能确认最终文件布局。若将 Python 压入 EXE 后启动时解压，则只是单文件分发，运行时仍产生 Python/DLL 文件，不符合这里的原生化目标。

如果连系统 WebView2 也不能依赖，则需要评估原生 GUI 框架；这将改变当前 Tauri 技术路线。[Tauri 的 WebView 依赖说明](https://v2.tauri.app/reference/webview-versions/)。
