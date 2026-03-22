param(
    [switch]$NoPause
)

$ErrorActionPreference = "Stop"

function Pause-IfNeeded {
    if (-not $NoPause) {
        Read-Host "按回车键退出"
    }
}

try {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $sourceTaskPath = Join-Path $scriptDir "EchoOCRTask.py"
    $workingDir = Join-Path $env:LOCALAPPDATA "ok-ww\data\apps\ok-ww\working"

    if (-not (Test-Path -Path $workingDir -PathType Container)) {
        throw "未找到 ok-ww 工作目录：$workingDir"
    }

    if (-not (Test-Path -Path $sourceTaskPath -PathType Leaf)) {
        throw "未找到源任务文件：$sourceTaskPath"
    }

    $targetTaskDir = Join-Path $workingDir "custom\task"
    New-Item -Path $targetTaskDir -ItemType Directory -Force | Out-Null
    Copy-Item -Path $sourceTaskPath -Destination (Join-Path $targetTaskDir "EchoOCRTask.py") -Force

    $configPath = Join-Path $workingDir "config.py"
    if (-not (Test-Path -Path $configPath -PathType Leaf)) {
        throw "未找到 config.py：$configPath"
    }

    $configContent = Get-Content -Path $configPath -Raw
    $entryRegex = "\[`"custom\.task\.EchoOCRTask`"\s*,\s*`"EchoOCRTask`"\s*\],?"
    if ($configContent -match $entryRegex) {
        Write-Host "onetime_tasks 中已存在 EchoOCRTask，未重复添加。"
        Write-Host "注入完成。"
        Pause-IfNeeded
        exit 0
    }

    $assignMatch = [regex]::Match($configContent, "'onetime_tasks'\s*:\s*\[")
    if (-not $assignMatch.Success) {
        throw "在 config.py 中未找到 'onetime_tasks': [ 配置项"
    }

    $lineBreak = if ($configContent.Contains("`r`n")) { "`r`n" } else { "`n" }
    $openBracketIndex = $assignMatch.Index + $assignMatch.Length - 1
    $depth = 0
    $closeBracketIndex = -1
    for ($i = $openBracketIndex; $i -lt $configContent.Length; $i++) {
        $ch = $configContent[$i]
        if ($ch -eq "[") {
            $depth++
        }
        elseif ($ch -eq "]") {
            $depth--
            if ($depth -eq 0) {
                $closeBracketIndex = $i
                break
            }
        }
    }

    if ($closeBracketIndex -lt 0) {
        throw "config.py 中的 onetime_tasks 列表格式不正确"
    }

    $listBody = $configContent.Substring($openBracketIndex + 1, $closeBracketIndex - $openBracketIndex - 1)
    $entryLine = "[`"custom.task.EchoOCRTask`", `"EchoOCRTask`"],"

    $indent = "    "
    $indentMatch = [regex]::Match($listBody, "(?m)^([ \t]+)\[")
    if ($indentMatch.Success) {
        $indent = $indentMatch.Groups[1].Value
    }

    $newBody = $listBody.TrimEnd()
    if ([string]::IsNullOrWhiteSpace($newBody)) {
        $newBody = "$lineBreak$indent$entryLine$lineBreak"
    }
    else {
        if ($newBody -notmatch ",\s*$") {
            $newBody = "$newBody,"
        }
        $newBody = "$newBody$lineBreak$indent$entryLine$lineBreak"
    }

    $newConfigContent =
        $configContent.Substring(0, $openBracketIndex + 1) +
        $newBody +
        $configContent.Substring($closeBracketIndex)

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($configPath, $newConfigContent, $utf8NoBom)
    Write-Host "注入完成。"
    Pause-IfNeeded
    exit 0
}
catch {
    $message = $_.Exception.Message
    if ([string]::IsNullOrWhiteSpace($message)) {
        $message = $_.ToString()
    }
    Write-Host $message -ForegroundColor Red
    Pause-IfNeeded
    exit 1
}
