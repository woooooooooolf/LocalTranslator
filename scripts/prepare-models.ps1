param([string]$Source = '')
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
if (!$Source) { $Source = Join-Path $project 'runtime' }
$packages = Join-Path $Source 'packages'
$segmentation = Join-Path $Source 'assets/argos-translate/minisbd'
if (!(Test-Path -LiteralPath $packages)) { throw 'Source runtime/packages not found' }
$destination = Join-Path $project 'models'
New-Item -ItemType Directory -Force -Path (Join-Path $destination 'packages'),(Join-Path $destination 'minisbd') | Out-Null
foreach ($package in Get-ChildItem -LiteralPath $packages -Directory) {
    if (!(Test-Path -LiteralPath (Join-Path $package.FullName 'metadata.json'))) { continue }
    $target = Join-Path $destination ('packages/' + $package.Name)
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    foreach ($file in @('metadata.json','sentencepiece.model','README.md')) {
        $path = Join-Path $package.FullName $file
        if (Test-Path -LiteralPath $path) { Copy-Item -LiteralPath $path -Destination $target }
    }
    & robocopy (Join-Path $package.FullName 'model') (Join-Path $target 'model') /E /R:1 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { throw 'Model copy failed' }
}
Get-ChildItem -LiteralPath $segmentation -Filter '*.onnx' -File | Copy-Item -Destination (Join-Path $destination 'minisbd')
Write-Host "External models prepared: $destination"
