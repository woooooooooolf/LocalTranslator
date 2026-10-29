param([int]$Jobs = 6)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $project
. (Join-Path $PSScriptRoot 'msvc-env.ps1')
$source = Join-Path $project '.cache/native-src/ort-source'
if (!(Test-Path -LiteralPath (Join-Path $source '.git'))) {
    & git -c http.sslBackend=openssl -c http.version=HTTP/1.1 clone --filter=blob:none --no-checkout --branch v1.22.0 --depth 1 https://github.com/microsoft/onnxruntime.git $source
    if ($LASTEXITCODE) { throw 'ONNX Runtime clone failed' }
    & git -c http.sslBackend=openssl -c http.version=HTTP/1.1 -C $source sparse-checkout set cmake include onnxruntime/core onnxruntime/contrib_ops onnxruntime/lora tools
    if ($LASTEXITCODE) { throw 'ONNX Runtime sparse checkout failed' }
    & git -c http.sslBackend=openssl -C $source checkout
    if ($LASTEXITCODE) { throw 'ONNX Runtime checkout failed' }
}
$revision = & git -C $source rev-parse HEAD
if ($revision -ne 'f217402897f40ebba457e2421bc0a4702771968e') { throw 'Unexpected ONNX Runtime source revision' }
$eigen = Join-Path $project '.cache/native-src/eigen'
if (!(Test-Path -LiteralPath (Join-Path $eigen '.git'))) {
    & git init $eigen
    & git -c http.sslBackend=openssl -c http.version=HTTP/1.1 -C $eigen fetch --depth 1 https://eigen.googlesource.com/mirror 1d8b82b0740839c0de7f1242a3585e3390ff5f33
    if ($LASTEXITCODE) { throw 'Pinned Eigen fetch failed' }
    & git -C $eigen checkout --detach FETCH_HEAD
    if ($LASTEXITCODE) { throw 'Pinned Eigen checkout failed' }
}
if ((& git -C $eigen rev-parse HEAD) -ne '1d8b82b0740839c0de7f1242a3585e3390ff5f33') { throw 'Unexpected Eigen source revision' }
& cmake --compile-no-warning-as-error -S (Join-Path $source 'cmake') -B .cache/onnx/build -G Ninja `
    '-DCMAKE_BUILD_TYPE=Release' '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded' `
    "-DCMAKE_PROJECT_TOP_LEVEL_INCLUDES=$project/native/msvc_response.cmake" `
    "-DFETCHCONTENT_SOURCE_DIR_EIGEN3=$eigen" '-DBUILD_TESTING=OFF' `
    '-DONNX_USE_MSVC_STATIC_RUNTIME=ON' '-Dprotobuf_MSVC_STATIC_RUNTIME=ON' '-DABSL_MSVC_STATIC_RUNTIME=ON' `
    '-Donnxruntime_BUILD_SHARED_LIB=OFF' '-Donnxruntime_BUILD_UNIT_TESTS=OFF' `
    '-Donnxruntime_USE_TELEMETRY=OFF' '-Donnxruntime_DISABLE_ML_OPS=ON' `
    '-Donnxruntime_ENABLE_CPUINFO=OFF' '-Donnxruntime_BUILD_BENCHMARKS=OFF' `
    '-DCMAKE_POLICY_VERSION_MINIMUM=3.5'
if ($LASTEXITCODE) { throw 'ONNX Runtime configure failed' }
& cmake --build .cache/onnx/build --config Release --parallel $Jobs
if ($LASTEXITCODE) { throw 'ONNX Runtime build failed' }
& cmake --build .cache/onnx/build --config Release --target re2 --parallel $Jobs
if ($LASTEXITCODE) { throw 'ONNX Runtime RE2 dependency build failed' }
$out = Join-Path $project '.cache/onnx/lib'
New-Item -ItemType Directory -Force -Path $out | Out-Null
# Combine built CPU component archives, so ort-sys links one controlled static library.
$libraries = Get-ChildItem (Join-Path $project '.cache/onnx/build') -Recurse -Filter '*.lib' -File |
    Where-Object { $_.FullName -notmatch '[\\/]CMakeFiles[\\/]|[\\/]try_compile[\\/]' -and $_.Name -notin @('libprotoc.lib','libprotobuf.lib','onnxruntime_providers_shared.lib') }
if (!$libraries) { throw 'No ONNX Runtime static archives found' }
$response = Join-Path $project '.cache/onnx/libraries.rsp'
$arguments = @('/NOLOGO',('/OUT:"' + (Join-Path $out 'onnxruntime.lib') + '"'))
$arguments += $libraries | ForEach-Object { '"' + $_.FullName + '"' }
$arguments | Set-Content -LiteralPath $response -Encoding Unicode
& lib.exe ('@' + $response)
if ($LASTEXITCODE) { throw 'ONNX Runtime archive bundling failed' }
Write-Host "Static ONNX Runtime ready: $out"
