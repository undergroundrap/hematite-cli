[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$Deep,
    [switch]$Reset,      # Full blank-slate: everything Deep does + wipes settings.json, PLAN.md, TASK.md
    [switch]$PruneDist   # Keep only the current Cargo.toml version under dist/
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $repoRoot

$removed = New-Object System.Collections.Generic.List[string]

function Get-CurrentVersion {
    $cargoTomlPath = Join-Path $repoRoot "Cargo.toml"
    if (-not (Test-Path -LiteralPath $cargoTomlPath)) {
        throw "Cargo.toml not found at repo root."
    }

    $cargoToml = Get-Content -LiteralPath $cargoTomlPath -Raw
    $match = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Could not determine package version from Cargo.toml."
    }

    $match.Groups[1].Value
}

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

function Remove-StaleDistArtifacts {
    $distRoot = Join-Path $repoRoot "dist"
    if (-not (Test-Path -LiteralPath $distRoot)) {
        return
    }

    $currentVersion = Get-CurrentVersion
    $keepPrefix = "Hematite-$currentVersion"

    $candidateParents = @($distRoot)
    $candidateParents += Get-ChildItem -LiteralPath $distRoot -Directory -Force -ErrorAction SilentlyContinue | ForEach-Object {
        $_.FullName
    }

    foreach ($parent in $candidateParents | Select-Object -Unique) {
        Get-ChildItem -LiteralPath $parent -Force -ErrorAction SilentlyContinue | ForEach-Object {
            if ($_.Name -like "Hematite-*" -and $_.Name -notlike "$keepPrefix*") {
                if ($PSCmdlet.ShouldProcess($_.FullName, "Remove stale dist artifact")) {
                    Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
                    $removed.Add($_.FullName) | Out-Null
                }
            }
        }
    }
}

$contentDirs = @(
    ".hematite\ghost",
    ".hematite\scratch",
    ".hematite\memories",
    ".hematite\sandbox",
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
    "build_output.txt",
    "build_errors.txt",
    "build_errors_utf8.txt",
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

if ($Deep -or $Reset) {
    $deepTargets = @(
        "target",
        "onnx_lib",
        ".hematite\vein.db"
    )

    foreach ($target in $deepTargets) {
        Remove-IfExists -LiteralPath $target
    }
}

if ($Reset) {
    # Full blank-slate — simulates a new user with no prior state.
    # settings.json and mcp_servers.json are intentionally NOT wiped here;
    # delete them manually if you want to test the absolute default config path.
    $resetTargets = @(
        ".hematite\PLAN.md",
        ".hematite\TASK.md"
    )

    foreach ($target in $resetTargets) {
        Remove-IfExists -LiteralPath $target
    }
}

if ($PruneDist) {
    Remove-StaleDistArtifacts
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
