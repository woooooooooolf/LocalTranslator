# 原生迁移记录

当前更新为 V1.0.0：在 0.4.0 的原生文字翻译和按需模型管理基础上，新增默认中文的中英文界面切换及翻译方向联动，使用 LocalTranslator 作为英文名。默认分发只含 EXE；模型由固定官方源按需获取，完整校验、加载验证后部署到旁边的 models 目录。下载模块不接收翻译内容，详见 [模型部署](MODEL_DEPLOYMENT.md)。

用户确认移除全部文档翻译与文件保存。文档模块、文件对话框、文件翻译命令及专用依赖均已删除；zip 依赖现仅用于受限解包模型，不恢复文档功能。下述文档格式验证是 0.3.0 阶段的历史记录，不代表当前仍提供这些功能。

目标：应用程序仅一个 EXE，CTranslate2 翻译模型和 MiniSBD 分句模型置于 `models`；不嵌入或解压 Python/DLL。Tauri 仍依赖系统 WebView2。

用户确认：移除 PDF 翻译，定位为隐私翻译工具；不实现 PDF 重排或引入字体组件。其他现有格式继续迁移。0.2.0 的源码、Python 对照与 `release/Jingyi` 保留；新产物使用独立的 `release/Jingyi-Native`。

原生迁移历史分支：`codex/native-single-exe`；V1.0.0 已快进合入 `main`。旧 0.2.0 基线保留于提交 `66a360b` 和本地对照运行时中。用户授权绑定 LocalTranslator 远端、推送及发布，当前通过匹配版本标签的 GitHub Actions 构建 Windows x64 Release。

## 依赖与验证

- CTranslate2 4.8.3：官方提交 `8d91aa08225004755360daa43deb62b35b1ed65d`，CPU/Ruy、静态 CRT、不使用 CUDA/MKL/OpenMP DLL；窄 C ABI 桥接，Unicode 文件流读取模型。
- C++ 静态核心已构建，16 个中英样本全部成功推理，12/16 的输出 token 与原 Python/MKL 完全一致。其余是措辞、虚词和标点差异，不能称为逐字兼容；详见可重复执行的 `scripts/check-native-core.py`，`--strict` 会在有任何差异时失败。
- SentencePiece 使用纯 Rust 运行时，后续将逐项比较 token 与解码结果。
- ONNX Runtime 1.22 官方提交 `f217402897f40ebba457e2421bc0a4702771968e`：需要静态 CRT 源码构建，保留原始量化 MiniSBD 模型及中文词典特征。
- 官方源码归档在末尾未使用的 WinML 测试资源处传输中断；本地提取的 4,319 个构建相关源码文件已逐个计算 Git blob SHA-1，与官方提交树一致，0 个不匹配。归档本身不作为完整发行证据。可重复构建脚本使用官方 Git 稀疏检出。

- 静态 ONNX Runtime 已完成编译：所有组件统一静态 CRT，关闭遥测；构建相关 LoRA 目录额外 7 个文件也逐一对照官方提交。Eigen 归档未通过原固定哈希，改为 Google 镜像中同一 Git 提交的源码。中文路径响应文件由仅用于构建的 UTF-16 BOM 启动器处理。
- Rust 原生文档模块的 4 项结构测试已通过，覆盖 Word 跨 run 合并及转义、字幕时间轴和标签、HTML 脚本/代码保留、ZIP 资源与原稿不变。
- 完整原生测试程序已运行：22 个分词样本和 24 个分句样本与原运行时一致，22 个翻译样本均产出结果，11/22 逐字一致。其余有措辞、虚词、标点及个别质量差异；保留在开发缓存报告中，不能标为逐字兼容或质量等价。
- TXT、SRT、HTML、DOCX、PPTX、ODT、ODP、EPUB 的小型合成样本路径均通过；原稿哈希、未翻译 ZIP 资源、字幕时间轴、EPUB XML 与首条 mimetype 保持有效。不是复杂真实文档的视觉验收。
- 完整原生测试程序只导入 Windows 系统 DLL，无 Python、ONNX Runtime、CTranslate2 或 VC++ 附带 DLL。测试将 PATH 限制为 System32 并从与 EXE 无关的工作目录启动，模型目录在测试前后逐文件哈希不变，正常 stderr 为空。
- 初次集成中的自定义 Rust 日志回调会在加载 ORT 会话时触发崩溃；去掉该回调后完整测试通过。保留禁遥测与 Fatal 会话日志级别，没有文件日志，也没有保留临时诊断输出。

最终仍需完成并记录：正式 Tauri EXE 的隔离目录启动与完整检查、原生窗口翻译/隐私回归及最终体积。
