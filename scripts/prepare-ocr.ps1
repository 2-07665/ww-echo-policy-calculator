$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ocr-assets.ps1')
$repo = Split-Path $PSScriptRoot -Parent
$ocrRoot = Join-Path $repo 'apps\ocr'
$deps = Join-Path $ocrRoot '.deps'
$runtime = Join-Path $ocrRoot 'runtime'
New-Item -ItemType Directory -Force $deps, $runtime | Out-Null
$revision = 'b8f84f0b80c529de40b4fbb3544b84fa7233a513'
$modelHash = '5435fd747c9e0efe15a96d0b378d5bd157e9492ed8fd80edf08f30d02fa24634'
$baseUri = "https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx/resolve/$revision"
Get-VerifiedOcrFile -Uri "$baseUri/inference.onnx" -Destination (Join-Path $runtime 'inference.onnx') -Sha256 $modelHash
Get-VerifiedOcrFile -Uri "$baseUri/inference.yml" -Destination (Join-Path $runtime 'inference.yml') -Sha256 'ab078671bb49f06228eadccd34f1bb501e157f7a047095ffb943ba81512c77d1'
$dictionary = Join-Path $ocrRoot 'assets\ppocrv6-small-dictionary.txt'
$dictionaryHash = 'b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d'
if ((Get-FileHash -LiteralPath $dictionary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $dictionaryHash) { throw 'OCR 源码字典摘要不匹配' }
Copy-Item -LiteralPath $dictionary -Destination (Join-Path $runtime 'dictionary.txt')
$ortVersion = '1.31.0'
$archive = Join-Path $deps "onnxruntime-win-x64-$ortVersion.zip"
$archiveHash = '3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195'
Get-VerifiedOcrFile -Uri "https://github.com/microsoft/onnxruntime/releases/download/v$ortVersion/onnxruntime-win-x64-$ortVersion.zip" -Destination $archive -Sha256 $archiveHash
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
