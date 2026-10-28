$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $project
if (!(Test-Path runtime/py/python.exe)) { throw '请先执行 npm run prepare:runtime' }
& npm.cmd run tauri -- build --no-bundle
if ($LASTEXITCODE) { throw 'Tauri 构建失败' }
$out = Join-Path $project 'release/Jingyi'
New-Item -ItemType Directory -Force -Path $out | Out-Null
Copy-Item -LiteralPath 'src-tauri/target/release/local-translate.exe' -Destination (Join-Path $out '静译.exe')
Copy-Item -LiteralPath 'README.md','THIRD_PARTY_NOTICES.md','VALIDATION.md' -Destination $out
Copy-Item -LiteralPath 'node_modules/lucide/LICENSE' -Destination (Join-Path $out 'LUCIDE_LICENSE.txt')
foreach ($part in @('runtime','backend')) {
    & robocopy (Join-Path $project $part) (Join-Path $out $part) /E /R:1 /W:1 /NFL /NDL /NJH /NJS /NP /XD __pycache__ | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "复制失败: $part" }
}
Write-Host "便携包: $out"
