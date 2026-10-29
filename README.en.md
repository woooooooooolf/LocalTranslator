# LocalTranslator · 静译

[简体中文](README.md) | English

![LocalTranslator English interface](docs/images/screenshot-en.png)

*Interface preview with synthetic sample text, rendered from the same frontend as the desktop app.*

Translate Chinese ↔ English and Japanese ↔ Chinese text locally on your CPU. The interface and inference engines are compiled into one EXE. Download models in the app, then translate offline.

- **Privacy first:** Input and output are processed in local memory, without sending text to a translation service or saving drafts, history or content logs. Translation works offline after model installation.
- **No application-level content restrictions:** The app adds no content moderation, keyword blocking or topic-based refusal rules. The 20,000-character per-request limit still applies.
- **Quality depends on the model:** Results may be disappointing, with awkward wording, mistranslations or omissions. Accuracy, fluency and completeness are not guaranteed, and performance can vary across topics and contexts.

## Download and run

Download `LocalTranslator_V1.1.0_Windows_x64.exe` from [Releases](https://github.com/woooooooooolf/LocalTranslator/releases/latest). Place it in a writable folder and run it. Python, Node.js, CUDA and accompanying application DLLs are not required.

Supported: **Windows 11 x64 / Windows 10 2004+ x64**, with [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) installed. This release supports Windows only. Linux build/linking and privacy behavior need separate porting and validation; macOS is not offered.

On first launch, open **Models** and download English → Chinese, Chinese → English or the shared Japanese ↔ Chinese model. Only explicit downloads contact the pinned sources. There are no background downloads or model updates. Once installed, translation works offline. Models can be deleted and downloaded again.

```text
LocalTranslator_V1.1.0_Windows_x64.exe
models/
  packages/    # Translation weights, tokenizer, configuration and attribution
  minisbd/     # Sentence-boundary models
```

Distribute the EXE alone, or include prepared `models/packages` and `models/minisbd` for offline machines. Do not distribute the temporary `.manager` or running `data` directories. Deleting a translation model retains small shared sentence-boundary files.

## Use

V1.1.0 adds Japanese ↔ Chinese translation and corrects the default translation directions. The original V1.0.0 assets remain unchanged.

- Enter or paste up to 20,000 characters. Click **Translate** or press `Ctrl+Enter`. Clipboard writes require an explicit copy action.
- The interface starts in Chinese with English → Chinese translation. Select **English** in the sidebar to use English and automatically choose Chinese → English. Switching back restores English → Chinese. Existing input and output stay unchanged. Missing directions prompt you to download the model.
- Select Japanese manually as source or target. The shared M2M100 418M INT8 bundle is about 474.2 MiB and supports both directions with one installation. See the [model evaluation](docs/JAPANESE_MODEL_EVALUATION.md) for measured latency, memory and failure examples. Manga image recognition and OCR are not included.
- Light/dark themes and expanding text editors are included. Interface language and manual theme choices apply only to the current session and are not stored in the browser.
- Text translation and copy only: no document import, translation history or translation file export.

## Local processing and models

Input and output remain in memory. The app does not write drafts, browser storage, content logs or translation caches. WebView2 runs in a separate temporary InPrivate session; autofill and password saving are disabled and verified through native settings. Sessions are cleaned up on exit, with abandoned sessions retried on the next launch. This is not secure erasure of Windows swap files or system crash dumps.

Only model downloads use the network. The module takes fixed model IDs, never translation text or arbitrary URLs. Argos / MiniSBD use official releases; M2M100 uses a documented community INT8 conversion. All assets use pinned versions, embedded SHA-256 checks and loading validation. Archives are extracted with limits, and interrupted or cancelled downloads do not become usable partial models. See [model deployment](docs/MODEL_DEPLOYMENT.md).

Chinese ↔ English uses Argos 1.9; Japanese ↔ Chinese uses M2M100 418M INT8. Both use native CPU inference with CTranslate2, SentencePiece and MiniSBD. No application-level content filters does not mean every text will be correctly understood: the Japanese model still makes errors in kinship terms, tone and other cases. The English/Chinese weights are unchanged, but native Ruy and Python/MKL may produce different wording. Licenses and attribution are in **About** and [third-party notices](THIRD_PARTY_NOTICES.md).

## Build from source

Requires Windows x64, Visual Studio C++ Build Tools / Windows SDK, Rust 1.99.0, Node.js 24, Python 3, CMake 3.28+ and Git. The scripts use Ninja bundled with Visual Studio. Python is build/test tooling only.

```powershell
npm ci
npm run native:core
npm run native:onnx
npm run portable
```

The first static-library build takes time. Upstream source revisions are pinned in the build scripts. The default output is a single named EXE in `release/LocalTranslator-V1.1.0/`. Use `npm run portable -- -IncludeModels` to optionally include models already prepared in the project.

```powershell
npx playwright install chromium
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml --bin local-translate
python scripts/fetch-model-catalog.py --verify
cargo test --locked --manifest-path src-tauri/Cargo.toml --bin local-translate cached_install_translate_remove -- --ignored --nocapture
```

UI regression tests use an isolated browser with mocked IPC. Model lifecycle tests load real weights and execute CPU translation in both directions. These checks do not replace native desktop acceptance. See [validation records](VALIDATION.md).

## Automated builds and releases

Code pushes to `main` and pull requests run Windows builds, UI/privacy checks and real-model tests. A `Vx.y.z` tag matching the project version publishes `LocalTranslator_Vx.y.z_Windows_x64.exe` and `SHA256SUMS.txt`. Workflow permissions are read-only by default; only the publishing job receives content-write permission. Releases are created only from tags. See the [release procedure](docs/RELEASE.md).
