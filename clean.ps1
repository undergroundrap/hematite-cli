[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$Deep
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $repoRoot

$removed = New-Object System.Collections.Generic.List[string]

function Remove-TreeContents {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LiteralPath
    )

    if (-not (Test-Path -LiteralPath $LiteralPath)) {
        return
    }

    Get-ChildItem -LiteralPath $LiteralPath -Force -ErrorAction SilentlyContinue | ForEach-Object {
        if ($PSCmdlet.ShouldProcess($_.FullName, "Remove")) {
            Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
            $removed.Add($_.FullName) | Out-Null
        }
    }
}

function Remove-IfExists {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LiteralPath
    )

    if (-not (Test-Path -LiteralPath $LiteralPath)) {
        return
    }

    $resolved = Resolve-Path -LiteralPath $LiteralPath -ErrorAction SilentlyContinue
    $display = if ($resolved) { $resolved.Path } else { (Join-Path $repoRoot $LiteralPath) }

    if ($PSCmdlet.ShouldProcess($display, "Remove")) {
        Remove-Item -LiteralPath $LiteralPath -Recurse -Force -ErrorAction SilentlyContinue
        $removed.Add($display) | Out-Null
    }
}

function Remove-Matches {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Pattern
    )

    Get-ChildItem -Path $Pattern -Force -ErrorAction SilentlyContinue | ForEach-Object {
        if ($PSCmdlet.ShouldProcess($_.FullName, "Remove")) {
            Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
            $removed.Add($_.FullName) | Out-Null
        }
    }
}

$contentDirs = @(
    ".hematite\ghost",
    ".hematite\scratch",
    ".hematite_logs",
    ".hematite_scratch",
    "tmp"
)

foreach ($dir in $contentDirs) {
    Remove-TreeContents -LiteralPath $dir
}

$runtimeFiles = @(
    ".hematite\session.json",
    ".hematite\last_request.json",
    ".hematite\vein.db-shm",
    ".hematite\vein.db-wal",
    "hematite_memory.db-shm",
    "hematite_memory.db-wal",
    "error.log",
    "error_log.txt",
    "our_errors.txt",
    "error_lines.txt",
    "build_errors.txt",
    "build_errors.txt.txt",
    "build_errors.txt.json",
    "errors.txt",
    "errors.txt.json",
    "errors.json",
    "errors.json.txt"
)

foreach ($file in $runtimeFiles) {
    Remove-IfExists -LiteralPath $file
}

$runtimeGlobs = @(
    "cargo_errors*.txt"
)

foreach ($pattern in $runtimeGlobs) {
    Remove-Matches -Pattern $pattern
}

Remove-TreeContents -LiteralPath ".hematite\reports"

if ($Deep) {
    $deepTargets = @(
        "target",
        "onnx_lib",
        ".hematite\vein.db"
    )

    foreach ($target in $deepTargets) {
        Remove-IfExists -LiteralPath $target
    }
}

$summary = if ($WhatIfPreference) {
    "Hematite cleanup dry run complete."
} elseif ($removed.Count -eq 0) {
    "Hematite cleanup: nothing to remove."
} else {
    "Hematite cleanup removed $($removed.Count) item(s)."
}

Write-Host $summary
if ($removed.Count -gt 0) {
    $removed | Sort-Object | ForEach-Object { Write-Host " - $_" }
}
