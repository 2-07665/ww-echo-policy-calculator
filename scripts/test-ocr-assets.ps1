# Offline checks for verified downloads; no model files, network or compiler needed.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ocr-assets.ps1')
$testDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "echo-ocr-assets-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $testDirectory | Out-Null
$destination = Join-Path $testDirectory 'model.bin'
$good = [System.Text.Encoding]::UTF8.GetBytes('verified fixture')
$fixture = Join-Path $testDirectory 'fixture.bin'
[System.IO.File]::WriteAllBytes($fixture, $good)
$hash = (Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant()
$script:downloadMode = 'good'
$script:downloads = 0
function Invoke-WebRequest {
    param($Uri, $OutFile, [switch]$UseBasicParsing, $ErrorAction)
    $script:downloads++
    if ($script:downloadMode -eq 'good') { [System.IO.File]::WriteAllBytes($OutFile, $good) }
    else {
        [System.IO.File]::WriteAllText($OutFile, 'bad download')
        if ($script:downloadMode -eq 'interrupted') { throw 'simulated interruption' }
    }
}
try {
    Get-VerifiedOcrFile -Uri 'https://example.invalid/model' -Destination $destination -Sha256 $hash
    if ((Get-FileHash -LiteralPath $destination).Hash.ToLowerInvariant() -ne $hash) { throw 'valid download failed' }
    Get-VerifiedOcrFile -Uri 'https://example.invalid/model' -Destination $destination -Sha256 $hash
    if ($script:downloads -ne 1) { throw 'valid cache downloaded again' }
    # A corrupt download must preserve the previous file and remove its partial.
    foreach ($mode in @('bad-hash','interrupted')) {
        $script:downloadMode = $mode
        $failed = $false
        try { Get-VerifiedOcrFile -Uri 'https://example.invalid/model' -Destination $destination -Sha256 ('0' * 64) }
        catch { $failed = $true }
        if (!$failed) { throw "expected failure: $mode" }
        if ((Get-FileHash -LiteralPath $destination).Hash.ToLowerInvariant() -ne $hash) { throw 'previous file was damaged' }
        if (Get-ChildItem -LiteralPath $testDirectory -Filter '*.partial') { throw 'partial file leaked' }
    }
    # Replace a corrupt cache only after receiving a verified file.
    [System.IO.File]::WriteAllText($destination, 'corrupt cache')
    $script:downloadMode = 'good'
    Get-VerifiedOcrFile -Uri 'https://example.invalid/model' -Destination $destination -Sha256 $hash
    if ((Get-FileHash -LiteralPath $destination).Hash.ToLowerInvariant() -ne $hash) { throw 'cache recovery failed' }
    Write-Host 'OCR asset checks passed: download, cache reuse, SHA-256 rejection, interrupted download and cache recovery.'
} finally {
    # Remove only the two files and partials created inside this unique test folder.
    Get-ChildItem -LiteralPath $testDirectory -File | Remove-Item -Force
    Remove-Item -LiteralPath $testDirectory
}
