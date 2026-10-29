# 中日文字翻译模型评测与选择

2026-10-29，范围为纯文字翻译。没有加入 OCR、图片、文档导入或译文保存。

## 结论

选用 **Meta M2M100 418M 的 CTranslate2 INT8 转换模型**，提供日语↔简体中文两个方向。共用一个约 474.2 MiB 下载包和同一进程内的一份模型权重，保留原有 Argos 中英模型。

这不是高质量漫画翻译方案：它能辅助理解短对白，但人物关系、语气、称呼和省略语仍可能译错。没有通过词语替换或屏蔽错误来修饰评测结果。

## 条件与范围

- CPU：AMD Ryzen 7 5700G，8 核 / 16 线程；系统报告物理内存约 43.9 GiB。
- 每个 CTranslate2 实例使用 4 个 CPU 推理线程，beam size 4。没有使用 GPU。
- 24 条仓库内合成对白，日→中与中→日各 12 条，含否定、亲属称谓、人名、代词、拟声词、两行对话；不是来自用户的漫画或私密输入。
- Python 对照使用现有开发运行时的 CTranslate2 4.8.3 INT8。M2M100 length penalty 1.0；Argos 使用原有 0.2 配置。每行单独翻译，中转执行两步。
- 原生验证使用实际静态 C++/Rust 引擎和 Ruy，包含 MiniSBD 分句；两种 M2M100 均进行 26 项 BPE/分句对照与全部 24 条翻译。它与 Python 按整行输入的计时/结果不是同一条完整处理链。
- 延迟是这些短句暖机后的单条中位值，含测试中的换行处理；内存为独立进程峰值工作集，包含分词器/动态库或原生分句运行时，不是纯权重大小。未做操作系统性能隔离，不作为其他 CPU 或长文本的吞吐保证。

## 测量

| 方案 | Python CT2 单条中位秒 | Python 进程峰值 MiB | 原生 Ruy 单条中位秒 | 原生进程峰值 MiB |
| --- | ---: | ---: | ---: | ---: |
| Argos ja↔en 1.1 + en↔zh 1.9，中转 | 0.070 | 501.9 | 未迁入 | 未迁入 |
| M2M100 418M INT8，直译 | 0.236 | 560.4 | 0.463 | 696.0 |
| M2M100 1.2B INT8，直译 | 0.497 | 1299.7 | 0.968 | 1433.4 |

1.2B 的部分措辞更自然，但没有解决亲属称谓错误；原生中位耗时和峰值工作集约为 418M 的两倍，权重下载约为 2.5 倍。对当前 CPU 隐私翻译工具，选择 418M 的成本更合适。Argos 旧日英模型中转虽快，但这些对白中有明显漏译和无关输出，因此不集成中转方案。Sakura 需要另一套推理适配且模型采用非商业许可，本轮没有实际运行，也没有给它编造性能/质量评分。

## 代表性原始结果

以下为原生 Ruy 的输出，不是人工润色后的译文。

| 输入 | 418M | 1.2B | 观察 |
| --- | --- | --- | --- |
| お前のこと、ずっと好きだった。 | 我一直喜欢你。 | 我一直喜欢你。 | 两者可辨认核心意思 |
| ここで会ったことは、二人だけの秘密ね。 | 在这里见到的只是两个秘密。 | 两人在这里见面是秘密的。 | 大模型有所改善，仍未完整表达“只属于两人的秘密” |
| 妹じゃなくて、姉です。 | 不是妹妹,而是妹妹。 | 不是妹妹,而是妹妹。 | 两者均译错“姐姐”，不能用于确认人物关系 |
| 田中さんは来ないって。先に行こう。 | 田田先生不来。 让我们先去吧。 | 田中先生不会来。 让我们先去吧。 | 418M 出现人名错误；1.2B 本例较好 |
| 才不是特意为你准备的呢！ | あなたのために特別に準備されていません! | 意図的に用意したわけではありません! | 两者语气均不自然，较大模型还丢失“为你” |
| 不是妹妹，是姐姐。 | 妹ではなく、妹です。 | 妹ではなく、妹です。 | 反向同样未正确区分亲属称谓 |

旧中转对照中，“待って！まだ話は終わってない！”得到“抱歉,此条目仅可使用日语.”；“我一直很喜欢你。”得到“お問い合わせ”。这是该样本和配置下的模型输出，不是程序设置的题材审核或拒译规则。

## 来源、许可与固定值

- 原模型：[facebook/m2m100_418M](https://huggingface.co/facebook/m2m100_418M)，MIT。
- 采用的社区 INT8 转换：[auralmira/m2m100-418M-ct2-int8](https://huggingface.co/auralmira/m2m100-418M-ct2-int8/tree/e205afefce2fd6933a1dca3b92b5063894f2f2bb)。固定提交 `e205afefce2fd6933a1dca3b92b5063894f2f2bb`，不是 Meta 官方发布的 CTranslate2 包。
- 主权重 490,667,752 字节，SHA-256：`137949450f9cf691f3c92084f14123e82d05ecb23ac3cd688308ce1a95214572`。
- SentencePiece 文件 SHA-256 为 `d8f7c76ed2a5e0822be39f0a4f95a55eb19c78f4593ce609e2edbc2aea4d380a`，与 Meta 官方仓库的同名文件一致。没有声称已对社区量化权重逐张量核对官方 FP32 原始权重。
- 分句使用 [MiniSBD v0.0.1](https://github.com/LibreTranslate/MiniSBD/releases/tag/v0.0.1) 的 `ja.onnx` 与 `zh-hans.onnx`。全部权重、配置、生成的元数据和署名文件均受内置清单大小/SHA-256 约束。
- 1.2B 对照为 [theDev997/m2m100-1.2B-ct2-int8](https://huggingface.co/theDev997/m2m100-1.2B-ct2-int8/tree/cedd592d23478a20e87645851003025498df60ff)，仅留在忽略的开发缓存中，不加入程序下载清单。

## 复现与集成检查

`scripts/evaluate-japanese-models.py` 包含固定下载版本、24 条样本、参考推理与原生推理，报告写入忽略的 `.cache/japanese-evaluation`，只使用合成测试文字。Python 和旧运行时仅供开发评测；不随应用分发。

```powershell
python scripts/evaluate-japanese-models.py download
python scripts/evaluate-japanese-models.py download --candidate m2m100-1.2b
runtime/py/python.exe -B -X utf8 scripts/evaluate-japanese-models.py benchmark --candidate argos-pivot
runtime/py/python.exe -B -X utf8 scripts/evaluate-japanese-models.py benchmark --candidate m2m100
runtime/py/python.exe -B -X utf8 scripts/evaluate-japanese-models.py benchmark --candidate m2m100-1.2b
cargo build --release --manifest-path crates/engine/Cargo.toml --bin check-engine
runtime/py/python.exe -B -X utf8 scripts/evaluate-japanese-models.py native --candidate m2m100
runtime/py/python.exe -B -X utf8 scripts/evaluate-japanese-models.py native --candidate m2m100-1.2b
```

应用只在主动点击下载时联网，IPC 不接受任意 URL 或文件路径。M2M100 原始大文件按固定清单下载，不通过解压大归档绕过原有 ZIP 上限。元数据声明语言标记，旧版 V1.0.0 会跳过它，避免将多语种权重错误地当作普通 Argos 模型使用。

未发布的实际桌面 EXE 也用同一批模型数据通过 24 条中日推理与 26 项分词/分句对照；短句中位约 0.455 秒，进程峰值约 697.4 MiB。完整权重已下载并逐文件校验；原生 HTTP 链路用有界 Range 核对实际内容，完整部署使用校验后的本地缓存。新增模型页的原生 GUI 自动启动被桌面输入服务的访问拒绝阻断，故没有将浏览器测试当成原生像素验收。
