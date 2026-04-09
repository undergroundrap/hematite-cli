[CmdletBinding()]
param(
    [switch]$Installer,  # Build the Inno Setup installer in addition to the portable zip
    [switch]$AddToPath   # Register the portable dir in the user PATH after building
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$cargoToml = Join-Path $repoRoot "Cargo.toml"
$cargoText = Get-Content -LiteralPath $cargoToml -Raw
$versionMatch = [regex]::Match($cargoText, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    throw "Could not determine package version from Cargo.toml."
}
$version = $versionMatch.Groups[1].Value

$distRoot  = Join-Path $repoRoot "dist\windows"
$bundleDir = Join-Path $distRoot "Hematite-$version-portable"
$releaseDir = Join-Path $repoRoot "target\release"
$readmeOut = Join-Path $bundleDir "README.txt"
$zipPath   = Join-Path $distRoot "Hematite-$version-portable.zip"
$issPath   = Join-Path $repoRoot "installer\hematite.iss"

function Resolve-Iscc {
    $command = Get-Command iscc -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }

    $candidates = @(
        (Join-Path ${env:LOCALAPPDATA} "Programs\Inno Setup 6\ISCC.exe"),
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe"
    )

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return $candidate
        }
    }

    return $null
}

# ── Build ─────────────────────────────────────────────────────────────────────

Write-Host "Building release binary (v$version)..." -ForegroundColor Cyan

$previousOffline = $env:CARGO_NET_OFFLINE
if (Test-Path Env:CARGO_NET_OFFLINE) { Remove-Item Env:CARGO_NET_OFFLINE }

try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed." }
} finally {
    if ($null -ne $previousOffline) { $env:CARGO_NET_OFFLINE = $previousOffline }
}

# ── Assemble portable bundle ──────────────────────────────────────────────────

New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
if (Test-Path -LiteralPath $bundleDir) { Remove-Item -LiteralPath $bundleDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null

$requiredFiles = @(
    (Join-Path $releaseDir "hematite.exe"),
    (Join-Path $releaseDir "DirectML.dll")
)

foreach ($file in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $file)) {
        throw "Required release artifact missing: $file"
    }
    Copy-Item -LiteralPath $file -Destination $bundleDir -Force
}

$readme = @"
Hematite $version
=================

What this is:
- Hematite is a local AI coding harness and terminal CLI for LM Studio.
- Built for single-GPU consumer hardware (tested on RTX 4070, 12 GB VRAM).
- No cloud. No API key. No per-token billing.

Before running:
1. Install LM Studio (https://lmstudio.ai).
2. Download and load a coding model. Recommended: Qwen/Qwen3.5-9B Q4_K_M (~6 GB VRAM).
3. Optionally load nomic-embed-text-v2 Q8_0 alongside it (~512 MB VRAM).
   This enables The Vein's semantic search. Both models fit on a 12 GB card.
4. Start LM Studio's local server on port 1234.

How to use:
- Open a terminal inside your project folder.
- Run: hematite

Status bar guide:
- LM:LIVE (green)  = LM Studio connected, coding model loaded and live
- LM:NONE (red)    = LM Studio running but no coding model loaded
- LM:BOOT (grey)   = Hematite starting up, detecting model
- LM:STALE (yellow)= Model detected but connection went quiet
- VN:SEM (green)   = Vein semantic search active (nomic loaded alongside coding model)
- VN:FTS (yellow)  = Vein keyword search only (load nomic-embed-text-v2 to upgrade)
- VN:--  (grey)    = Vein not yet indexed (will populate on first turn)
- BUD / CMP        = prompt budget and compaction pressure

More info: https://github.com/undergroundrap/hematite-cli
"@
Set-Content -LiteralPath $readmeOut -Value $readme -Encoding ASCII

# ── Zip ───────────────────────────────────────────────────────────────────────

if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
Compress-Archive -Path (Join-Path $bundleDir '*') -DestinationPath $zipPath

$zipMB = [math]::Round((Get-Item $zipPath).Length / 1MB)
$exeMB = [math]::Round((Get-Item (Join-Path $bundleDir "hematite.exe")).Length / 1MB)
$dllMB = [math]::Round((Get-Item (Join-Path $bundleDir "DirectML.dll")).Length / 1MB)

Write-Host ""
Write-Host "Portable bundle ready: $bundleDir" -ForegroundColor Green
Write-Host "  hematite.exe  — ${exeMB}MB"
Write-Host "  DirectML.dll  — ${dllMB}MB"
Write-Host "Portable zip:    $zipPath (${zipMB}MB)" -ForegroundColor Green

# ── Installer (optional) ──────────────────────────────────────────────────────

if ($Installer) {
    $iscc = Resolve-Iscc
    if (-not $iscc) {
        throw "Inno Setup compiler (iscc) is not installed. Install Inno Setup, then rerun with -Installer."
    }

    & $iscc "/DAppVersion=$version" "/DBundleDir=$bundleDir" "/DOutputDir=$distRoot" $issPath
    if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed." }

    Write-Host "Installer output ready in: $distRoot" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Tip: add -Installer to also build the Windows Setup.exe (requires Inno Setup)"
}

# ── PATH registration (optional) ──────────────────────────────────────────────

if ($AddToPath) {
    $absBundle = (Resolve-Path $bundleDir).Path
    $userPath  = [Environment]::GetEnvironmentVariable("Path", "User")
    # Remove any stale Hematite portable paths before adding the new one
    $cleaned = ($userPath -split ';' | Where-Object { $_ -notlike '*Hematite-*-portable*' }) -join ';'
    if ($cleaned -notlike "*$absBundle*") {
        [Environment]::SetEnvironmentVariable("Path", "$cleaned;$absBundle", "User")
        Write-Host ""
        Write-Host "Added to user PATH: $absBundle" -ForegroundColor Cyan
        Write-Host "Restart your terminal (or IDE) for 'hematite' to be available everywhere."
    } else {
        Write-Host ""
        Write-Host "Already on PATH: $absBundle"
    }
} else {
    Write-Host "Tip: add -AddToPath to register 'hematite' in your user PATH"
    Write-Host "     pwsh ./scripts/package-windows.ps1 -AddToPath"
}
