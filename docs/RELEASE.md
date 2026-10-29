# Release procedure / 发布流程

Current supported artifact: `LocalTranslator_Vx.y.z_Windows_x64.exe`.

1. Update `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, both Cargo manifests/lockfiles and the About version in `src/main.ts`. Update release-specific paths in the READMEs.
2. Run the UI, native privacy/model tests and portable build. Record relevant native desktop checks separately from browser tests. Check the final EXE with `scripts/verify-release.py`.
3. Commit the reviewed changes and push `main`. Wait for the exact-head **Windows build and release** workflow to pass. The first build compiles static CTranslate2 and ONNX Runtime; later builds reuse pinned caches.
4. Add Chinese release notes at `docs/releases/Vx.y.z.md`. READMEs remain bilingual. Create and push the matching `Vx.y.z` Git tag only when release publication is explicitly authorized; if a notes file is absent, the workflow uses a short Chinese description.
5. The tag workflow repeats the gates, builds the EXE on GitHub, validates its dependencies and standalone behavior, uploads the EXE and `SHA256SUMS.txt`, then creates the Release. Only the tag publishing job has `contents: write`.
6. Verify the release tag SHA and downloaded asset checksum. Keep models outside the release assets. No Linux/macOS artifact should be added without platform-specific build, privacy and runtime validation.

中文要点：先完成并提交版本、文档与验证，推送 main 并确认对应提交构建通过；得到发布授权后再推送同版本的 `Vx.y.z` 标签。Actions 从源码构建 Windows x64 EXE，校验后发布，模型不随包附带。不要将本地临时文件、旧 Python 运行时或开发缓存上传为附件。
