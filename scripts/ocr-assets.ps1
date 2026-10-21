# Download only through the explicit preparation entry point, never during OCR.
function Get-VerifiedOcrFile {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$Destination,
        [Parameter(Mandatory = $true)][string]$Sha256
    )
    if ((Test-Path -LiteralPath $Destination -PathType Leaf) -and
        (Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash.ToLowerInvariant() -eq $Sha256) { return }

    $parent = Split-Path -Parent ([System.IO.Path]::GetFullPath($Destination))
    New-Item -ItemType Directory -Force $parent | Out-Null
    $temporary = "$Destination.$([guid]::NewGuid().ToString('N')).partial"
    try {
        Invoke-WebRequest -Uri $Uri -OutFile $temporary -UseBasicParsing -ErrorAction Stop
        if ((Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Sha256) {
            throw "OCR 下载文件摘要不匹配：$Uri"
        }
        Move-Item -LiteralPath $temporary -Destination $Destination -Force
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force }
    }
}
