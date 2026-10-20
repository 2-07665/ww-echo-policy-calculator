$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
& (Join-Path $repo 'apps\ocr\native\build.cmd')
if ($LASTEXITCODE -ne 0) { throw "原生 OCR 构建失败：$LASTEXITCODE" }
$runtime = Join-Path $repo 'apps\ocr\runtime'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$redistRoot = Join-Path $installation 'VC\Redist\MSVC'
$redist = Get-ChildItem -LiteralPath $redistRoot -Force | Where-Object { $_.Name -match '^\d+\.' } | Sort-Object Name -Descending | Select-Object -First 1
if (!$redist) { throw '找不到可分发的 Visual C++ 运行库' }
$crt = Join-Path $redist.FullName 'x64\Microsoft.VC143.CRT'
foreach ($dll in @('msvcp140.dll','msvcp140_1.dll','vcruntime140.dll','vcruntime140_1.dll')) {
    Copy-Item -LiteralPath (Join-Path $crt $dll) -Destination $runtime
}
$bytes = (Get-ChildItem -LiteralPath $runtime -File | Measure-Object Length -Sum).Sum
Write-Host ('OCR 独立运行目录：{0:N2} MiB ({1})' -f ($bytes / 1MB), $runtime)
