# 第三方组件与模型

这是本地集成工程的组件说明，不代表已完成公开发行所需的完整许可证清单或源代码交付。现阶段产物用于本地评估；未配置项目整体开源许可证，未进行公开发布。

## 直接使用

- **Tauri 2**：MIT / Apache-2.0。https://github.com/tauri-apps/tauri
- **Lucide**：ISC。https://github.com/lucide-icons/lucide 。许可证位于 npm 包 `node_modules/lucide/LICENSE`，打包脚本同时附带该许可。
- **Python 3.11.9**：Python Software Foundation License 等。许可原文在 `runtime/py/LICENSE.txt`。
- **Argos Translate / argos-translate-lt**：MIT。原环境同时存在 1.11.0 与 1.12.1 的 distribution metadata，二者提供同名 `argostranslate` 模块；运行使用复制环境实际安装的实现，不重新覆盖安装。原有 metadata 和许可证保留在 `runtime/py/Lib/site-packages`。https://github.com/argosopentech/argos-translate
- **CTranslate2 4.8.3**：MIT。https://github.com/OpenNMT/CTranslate2
- **Argos Translate Files 1.4.4**：沿用原始安装；包 metadata 未清楚标注整体许可，其 PDF 实现注明衍生自 GPLv3 的 PolyglotPDF。https://github.com/LibreTranslate/argos-translate-files
- **PyMuPDF**：原环境自带的 PDF 处理组件，其许可信息保留于对应 `.dist-info` 目录。https://github.com/pymupdf/PyMuPDF
- **MiniSBD、SentencePiece、Stanza、SpaCy、Torch 等**：保留原始 Python 依赖及各自许可，详情见对应 `.dist-info` 内的 `METADATA`、`LICENSE` 或 `licenses`。

## 随包模型署名

保留两份模型的原始 README，均为 Argos v1.9：

- English → Chinese：`runtime/packages/translate-en_zh-1_9/README.md`
- Chinese → English：`runtime/packages/translate-zh_en-1_9/README.md`

模型 README 声明原始 OPUS 模型为 **CC BY 4.0**，作者 **Jörg Tiedemann and Santhosh Thottingal**，论文 **OPUS-MT — Building open translation services for the World**，EAMT 2020，Lisbon, Portugal。本工程未修改模型权重。https://creativecommons.org/licenses/by/4.0/

中英文 MiniSBD 模型由原工具的本地缓存复制，未修改权重。

## 保留但未调用的原始组件

首版完整复制 Python 运行时，因此还含 LibreTranslate（AGPL 系列）等不被桌面翻译路径调用的组件。现有许可证文件保持原样。日后若公开分发，应整理精简运行环境、准确的各组件版本与许可证、必要的源代码和署名，再制定发布方式；不应仅给整个便携目录标注 MIT。
