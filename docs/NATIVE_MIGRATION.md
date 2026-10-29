# 原生迁移记录

目标：应用程序仅一个 EXE，CTranslate2 翻译模型和 MiniSBD 分句模型置于 `models`；不嵌入或解压 Python/DLL。Tauri 仍依赖系统 WebView2。

用户确认：移除 PDF 翻译，定位为隐私翻译工具；不实现 PDF 重排或引入字体组件。其他现有格式继续迁移。0.2.0 的源码、Python 对照与 `release/Jingyi` 保留；新产物使用独立的 `release/Jingyi-Native`。

开发分支：`codex/native-single-exe`。按已验证的功能批次提交，不推送或发布。

## 依赖与验证

- CTranslate2 4.8.3：官方提交 `8d91aa08225004755360daa43deb62b35b1ed65d`，CPU/Ruy、静态 CRT、不使用 CUDA/MKL/OpenMP DLL；窄 C ABI 桥接，Unicode 文件流读取模型。
- C++ 静态核心已构建，16 个中英样本全部成功推理，12/16 的输出 token 与原 Python/MKL 完全一致。其余是措辞、虚词和标点差异，不能称为逐字兼容；详见可重复执行的 `scripts/check-native-core.py`，`--strict` 会在有任何差异时失败。
- SentencePiece 使用纯 Rust 运行时，后续将逐项比较 token 与解码结果。
- ONNX Runtime 1.22 官方提交 `f217402897f40ebba457e2421bc0a4702771968e`：需要静态 CRT 源码构建，保留原始量化 MiniSBD 模型及中文词典特征。
- 官方源码归档在末尾未使用的 WinML 测试资源处传输中断；本地提取的 4,319 个构建相关源码文件已逐个计算 Git blob SHA-1，与官方提交树一致，0 个不匹配。归档本身不作为完整发行证据。可重复构建脚本使用官方 Git 稀疏检出。

- 静态 ONNX Runtime 已完成编译：所有组件统一静态 CRT，关闭遥测；构建相关 LoRA 目录额外 7 个文件也逐一对照官方提交。Eigen 归档未通过原固定哈希，改为 Google 镜像中同一 Git 提交的源码。中文路径响应文件由仅用于构建的 UTF-16 BOM 启动器处理。
- Rust 原生文档模块的 4 项结构测试已通过，覆盖 Word 跨 run 合并及转义、字幕时间轴和标签、HTML 脚本/代码保留、ZIP 资源与原稿不变。

最终仍需完成并记录：完整链路对照、全部保留格式的合成样本测试、原生窗口翻译/隐私回归、只含 EXE 和模型的隔离目录启动、PE 导入检查及最终体积。
