param([int]$Jobs = 6, [switch]$WithCheck)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $project
function Run-Git([string[]]$Arguments) {
    & git @Arguments
    if ($LASTEXITCODE) { throw "git failed: $Arguments" }
}
$source = Join-Path $project '.cache/native-src/ctranslate2'
if (!(Test-Path -LiteralPath (Join-Path $source '.git'))) {
    Run-Git @('clone','--branch','v4.8.3','--depth','1','https://github.com/OpenNMT/CTranslate2.git',$source)
}
$revision = & git -C $source rev-parse HEAD
if ($revision -ne '8d91aa08225004755360daa43deb62b35b1ed65d') { throw 'Unexpected CTranslate2 source revision' }
Run-Git @('-C',$source,'submodule','update','--init','--depth','1','--jobs','3','third_party/cpu_features','third_party/ruy','third_party/spdlog')
Run-Git @('-C',(Join-Path $source 'third_party/ruy'),'submodule','update','--init','--depth','1','third_party/cpuinfo')
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
& cmake -S native -B .cache/native/build -G Ninja '-DCMAKE_BUILD_TYPE=Release' '-DCMAKE_POLICY_VERSION_MINIMUM=3.5'
if ($LASTEXITCODE) { throw 'Native configure failed' }
$targets = @('jingyi_native')
if ($WithCheck) { $targets += 'jingyi-native-check' }
& cmake --build .cache/native/build --config Release --target $targets --parallel $Jobs
if ($LASTEXITCODE) { throw 'Native build failed' }
