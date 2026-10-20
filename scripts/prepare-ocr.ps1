param([string]$UmaOCR = 'D:\Games\uma\UmaOCR')
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$ocrRoot = Join-Path $repo 'apps\ocr'
$deps = Join-Path $ocrRoot '.deps'
$runtime = Join-Path $ocrRoot 'runtime'
New-Item -ItemType Directory -Force $deps, $runtime | Out-Null
$revision = 'b8f84f0b80c529de40b4fbb3544b84fa7233a513'
$source = Join-Path $UmaOCR ".models\ocr\v6_small\$revision\recognition"
$modelHash = '5435fd747c9e0efe15a96d0b378d5bd157e9492ed8fd80edf08f30d02fa24634'
if (!(Test-Path -LiteralPath (Join-Path $source 'inference.onnx'))) { throw "找不到 v6 small 模型：$source" }
if ((Get-FileHash -LiteralPath (Join-Path $source 'inference.onnx') -Algorithm SHA256).Hash.ToLowerInvariant() -ne $modelHash) { throw '来源模型摘要不匹配' }
foreach ($name in @('inference.onnx','inference.yml','dictionary.txt')) {
    $expected = @{
        'inference.onnx' = $modelHash
        'inference.yml' = 'ab078671bb49f06228eadccd34f1bb501e157f7a047095ffb943ba81512c77d1'
        'dictionary.txt' = 'b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d'
    }[$name]
    if ((Get-FileHash -LiteralPath (Join-Path $source $name) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw "来源文件摘要不匹配：$name" }
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination (Join-Path $runtime $name)
}
$ortVersion = '1.31.0'
$archive = Join-Path $deps "onnxruntime-win-x64-$ortVersion.zip"
$archiveHash = '3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195'
if (!(Test-Path $archive)) {
    Invoke-WebRequest "https://github.com/microsoft/onnxruntime/releases/download/v$ortVersion/onnxruntime-win-x64-$ortVersion.zip" -OutFile "$archive.partial"
    if ((Get-FileHash "$archive.partial" -Algorithm SHA256).Hash.ToLowerInvariant() -ne $archiveHash) { throw 'ONNX Runtime 下载摘要不匹配' }
    Move-Item -LiteralPath "$archive.partial" -Destination $archive
}
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $archiveHash) { throw 'ONNX Runtime 缓存摘要不匹配' }
$sdk = Join-Path $deps "onnxruntime-win-x64-$ortVersion"
if (!(Test-Path (Join-Path $sdk 'include\onnxruntime_cxx_api.h'))) { Expand-Archive -LiteralPath $archive -DestinationPath $deps -Force }
Copy-Item -LiteralPath (Join-Path $sdk 'lib\onnxruntime.dll') -Destination $runtime
foreach ($name in @('LICENSE','ThirdPartyNotices.txt')) {
    if (Test-Path (Join-Path $sdk $name)) { Copy-Item -LiteralPath (Join-Path $sdk $name) -Destination (Join-Path $runtime "ONNXRuntime-$name") }
}
Copy-Item -LiteralPath (Join-Path $ocrRoot 'native\PaddleOCR-LICENSE.txt') -Destination $runtime
$files = @('inference.onnx','inference.yml','dictionary.txt','onnxruntime.dll') | ForEach-Object {
    $p = Join-Path $runtime $_
    @{ file = $_; bytes = (Get-Item -LiteralPath $p).Length; sha256 = (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() }
}
@{ model = 'PP-OCRv6_small_rec'; revision = $revision; source = 'PaddlePaddle/PP-OCRv6_small_rec_onnx'; onnxruntime = $ortVersion; files = $files } | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $runtime 'manifest.json')
Write-Host "OCR 运行文件已准备：$runtime"
