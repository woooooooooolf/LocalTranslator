$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vsRoot) { throw 'MSVC Build Tools not found' }
$devCommand = '"' + (Join-Path $vsRoot 'Common7/Tools/VsDevCmd.bat') + '" -arch=x64 -host_arch=x64 >nul && set'
$environmentLines = & cmd.exe /d /s /c $devCommand
if ($LASTEXITCODE) { throw 'MSVC environment initialization failed' }
foreach ($entry in $environmentLines) {
    if ($entry -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') }
}
$env:PATH = (Join-Path $vsRoot 'Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja') + ';' + $env:PATH
