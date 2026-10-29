# 静译原生版第三方组件与模型

0.3.1 仅提供原生 CPU 文字翻译。Python、Torch、SpaCy、LibreTranslate、Argos Translate Files 和 PyMuPDF 不随原生便携包分发；旧版本仅保留为开发对照。

应用内“关于 → 第三方说明”包含生成的许可原文与 Rust 构建依赖清单。生成器为 `scripts/generate-native-notices.py`；本工程未配置远程发布或整体开源许可。

## 主要组件

- Tauri 2：MIT / Apache-2.0。https://github.com/tauri-apps/tauri
- Lucide：ISC。https://github.com/lucide-icons/lucide
- CTranslate2 4.8.3：MIT，官方提交 8d91aa08225004755360daa43deb62b35b1ed65d。https://github.com/OpenNMT/CTranslate2
- Ruy、cpu_features：Apache-2.0；cpuinfo/clog：BSD 系列；spdlog：MIT。使用 CTranslate2 固定的子模块版本。
- ONNX Runtime 1.22：MIT，官方提交 f217402897f40ebba457e2421bc0a4702771968e。https://github.com/microsoft/onnxruntime
- Eigen：以 EIGEN_MPL2_ONLY 构建，MPL-2.0，固定提交 1d8b82b0740839c0de7f1242a3585e3390ff5f33。原始源码：https://gitlab.com/libeigen/eigen/-/tree/1d8b82b0740839c0de7f1242a3585e3390ff5f33 。包含许可原文，源码未修改。
- ONNX Runtime 的其他组成依赖及许可见其原始 ThirdPartyNotices.txt，随内置说明一并保留。
- sentencepiece-rs 0.2.2：Apache-2.0，SentencePiece 的纯 Rust 运行时实现。https://github.com/Rayato159/sentencepiece-rs
- MiniSBD 的预处理、词典特征与边界解码逻辑参照原运行时迁移为 Rust；模型未改变。保留来自 Stanza 的 Apache-2.0 署名和 credits。https://github.com/LibreTranslate/MiniSBD
- 原生文档处理代码及其专用依赖已移除；其他构建和运行依赖的版本及 SPDX 表达式见内置清单。

## 外置模型

保留模型原 README，位置为 `models/packages/translate-en_zh-1_9/README.md` 和 `models/packages/translate-zh_en-1_9/README.md`。

这两份 README 声明，原始 OPUS 模型许可为 CC BY 4.0，作者为 Jörg Tiedemann 与 Santhosh Thottingal，出处为 OPUS-MT — Building open translation services for the World，EAMT 2020，Lisbon, Portugal。本工程未修改翻译权重。https://creativecommons.org/licenses/by/4.0/

`models/minisbd/en.onnx` 与 `zh-hans.onnx` 由原工具的 MiniSBD 本地模型复制，保留其发布来源：https://github.com/LibreTranslate/MiniSBD/releases/tag/v0.0.1 。
