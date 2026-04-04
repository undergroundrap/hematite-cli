[CmdletBinding()]
param(
    [switch]$Installer
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

$distRoot = Join-Path $repoRoot "dist\windows"
$bundleDir = Join-Path $distRoot "Hematite-$version-portable"
$releaseDir = Join-Path $repoRoot "target\release"
$readmeOut = Join-Path $bundleDir "README.txt"
$zipPath = Join-Path $distRoot "Hematite-$version-portable.zip"
$issPath = Join-Path $repoRoot "installer\hematite.iss"

function Resolve-Iscc {
    $command = Get-Command iscc -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

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

New-Item -ItemType Directory -Force -Path $distRoot | Out-Null
if (Test-Path -LiteralPath $bundleDir) {
    Remove-Item -LiteralPath $bundleDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $bundleDir | Out-Null

$previousOffline = $env:CARGO_NET_OFFLINE
if (Test-Path Env:CARGO_NET_OFFLINE) {
    Remove-Item Env:CARGO_NET_OFFLINE
}

try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed."
    }
}
finally {
    if ($null -ne $previousOffline) {
        $env:CARGO_NET_OFFLINE = $previousOffline
    }
}

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
- Hematite is a local GPU coding harness and terminal CLI for LM Studio.

Before running:
1. Install LM Studio.
2. Load a compatible Gemma-family model.
3. Start LM Studio's local server on port 1234.

How to use:
- Open a terminal inside your project folder.
- Run: hematite

Installer note:
- If you used the Windows installer and selected the PATH option, open a fresh terminal after installation.
"@
Set-Content -LiteralPath $readmeOut -Value $readme -Encoding ASCII

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -Path (Join-Path $bundleDir '*') -DestinationPath $zipPath

if ($Installer) {
    $iscc = Resolve-Iscc
    if (-not $iscc) {
        throw "Inno Setup compiler (iscc) is not installed. Install Inno Setup, then rerun with -Installer."
    }

    & $iscc "/DAppVersion=$version" "/DBundleDir=$bundleDir" "/DOutputDir=$distRoot" $issPath
    if ($LASTEXITCODE -ne 0) {
        throw "Installer compilation failed."
    }
}

Write-Host "Portable bundle ready: $bundleDir"
Write-Host "Portable zip ready: $zipPath"
if ($Installer) {
    Write-Host "Installer output ready in: $distRoot"
} else {
    Write-Host "Installer not built. Re-run with -Installer after installing Inno Setup."
}
