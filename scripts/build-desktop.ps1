# Portable release, including the self-contained native OCR directory.
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
& (Join-Path $PSScriptRoot 'build-ocr.ps1')
Push-Location $repo
try {
    cargo build --release --locked -p echo_calculator_app
    if ($LASTEXITCODE -ne 0) { throw '桌面程序构建失败' }
    $destination = Join-Path $repo 'out\EchoStudio'
    New-Item -ItemType Directory -Force $destination | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo 'target\release\echo_calculator_app.exe') -Destination (Join-Path $destination 'Echo Studio.exe')
    $ocrDestination = Join-Path $destination 'ocr'
    New-Item -ItemType Directory -Force $ocrDestination | Out-Null
    Get-ChildItem -LiteralPath (Join-Path $repo 'apps\ocr\runtime') -File | Copy-Item -Destination $ocrDestination -Force
    Write-Host "便携版已生成：$destination"
} finally { Pop-Location }
