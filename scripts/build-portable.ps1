param([string]$OutputName = '', [switch]$IncludeModels)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $project
$version = (Get-Content -LiteralPath 'package.json' -Raw | ConvertFrom-Json).version
if (!$OutputName) {
    $OutputName = 'LocalTranslator-V' + ($version -replace '[^A-Za-z0-9._-]', '-')
}
if ($OutputName -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$' -or $OutputName -in @('.', '..')) { throw 'Invalid output folder name' }
if ($IncludeModels -and !(Test-Path models/packages)) { throw 'Run npm run prepare:models first' }
if (!(Test-Path .cache/native/build/lib/jingyi_native.lib)) { throw 'Run npm run native:core first' }
if (!(Test-Path .cache/onnx/lib/onnxruntime.lib)) { throw 'Run npm run native:onnx first' }
& python scripts/generate-native-notices.py
if ($LASTEXITCODE) { throw 'Notice generation failed' }
& npm.cmd run tauri -- build --no-bundle
if ($LASTEXITCODE) { throw 'Tauri 构建失败' }
$out = Join-Path (Join-Path $project 'release') $OutputName
New-Item -ItemType Directory -Force -Path $out | Out-Null
$exeName = "LocalTranslator_V${version}_Windows_x64.exe"
Copy-Item -LiteralPath 'src-tauri/target/release/local-translate.exe' -Destination (Join-Path $out $exeName)
if ($IncludeModels) {
    & robocopy (Join-Path $project 'models') (Join-Path $out 'models') /E /R:1 /W:1 /NFL /NDL /NJH /NJS /NP /XD __pycache__ .manager | Out-Null
    if ($LASTEXITCODE -ge 8) { throw '模型复制失败' }
}
if ((Test-Path (Join-Path $out 'runtime')) -or (Test-Path (Join-Path $out 'backend'))) { throw 'Unexpected legacy runtime in native output directory' }
$attachments = if (Test-Path (Join-Path $out 'models')) {
    Get-ChildItem -LiteralPath (Join-Path $out 'models') -Recurse -File |
        Where-Object { $_.Extension -in @('.exe','.dll','.pyd','.py') }
}
if ($attachments) { throw 'Program/runtime attachments were found under models' }
Write-Host "便携包: $out"
