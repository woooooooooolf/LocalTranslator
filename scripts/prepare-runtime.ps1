param([string]$Source = 'D:\AIWorkSpace_ZCode\翻译')
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$destination = Join-Path $project 'runtime'
if (!(Test-Path -LiteralPath (Join-Path $Source 'py/python.exe'))) { throw '找不到便携 Python。请使用 -Source 指定原工具目录。' }
New-Item -ItemType Directory -Force -Path $destination | Out-Null
function Copy-Tree($from, $to) {
    & robocopy $from $to /E /R:1 /W:1 /NFL /NDL /NJH /NJS /NP /XD __pycache__ /XF *.pyc | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "复制失败: $from" }
}
Copy-Tree (Join-Path $Source 'py') (Join-Path $destination 'py')
Copy-Tree (Join-Path $Source 'data/packages') (Join-Path $destination 'packages')
Copy-Tree (Join-Path $Source 'data/xdg/data/argos-translate/minisbd') (Join-Path $destination 'assets/argos-translate/minisbd')
Write-Host '便携运行时和模型已复制到 runtime，原目录未修改。'
