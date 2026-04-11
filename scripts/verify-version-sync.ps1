# Verifies that Hematite's tracked release-version surfaces are in sync.
# Usage:
#   pwsh ./scripts/verify-version-sync.ps1
#   pwsh ./scripts/verify-version-sync.ps1 -Version X.Y.Z -PreviousVersion A.B.C

param(
    [string]$Version,
    [string]$PreviousVersion,
    [switch]$RequireCargoLock
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

function Get-FileText([string]$Path) {
    Get-Content -LiteralPath $Path -Raw
}

function Require-Match([string]$Path, [string]$Pattern, [string]$Description) {
    $text = Get-FileText $Path
    if ($text -notmatch $Pattern) {
        throw "$Path is out of sync: expected $Description"
    }
}

function Reject-Match([string]$Path, [string]$Pattern, [string]$Description) {
    $text = Get-FileText $Path
    if ($text -match $Pattern) {
        throw "$Path still contains $Description"
    }
}

if (-not $Version) {
    $cargoToml = Get-FileText "Cargo.toml"
    $versionMatch = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $versionMatch.Success) {
        throw "Could not determine package version from Cargo.toml."
    }
    $Version = $versionMatch.Groups[1].Value
}
else {
    $cargoToml = Get-FileText "Cargo.toml"
}

$packageNameMatch = [regex]::Match($cargoToml, '(?m)^name\s*=\s*"([^"]+)"')
if (-not $packageNameMatch.Success) {
    throw "Could not determine package name from Cargo.toml."
}
$packageName = $packageNameMatch.Groups[1].Value

$escapedVersion = [regex]::Escape($Version)
$escapedPackageName = [regex]::Escape($packageName)
$installerPattern = "(?m)^\s*#define AppVersion\s+`"$escapedVersion`"\r?$"
$readmeBadgePattern = [regex]::Escape("version-$Version")
$cargoLockPattern = "(?ms)\[\[package\]\]\s*name = `"$escapedPackageName`"\s*version = `"$escapedVersion`""

Require-Match "Cargo.toml" "(?m)^version\s*=\s*`"$escapedVersion`"$" "Cargo.toml package version $Version"
Require-Match "installer\hematite.iss" $installerPattern "installer AppVersion $Version"
Require-Match "README.md" $readmeBadgePattern "README version badge for $Version"

if ($RequireCargoLock) {
    Require-Match "Cargo.lock" $cargoLockPattern "Cargo.lock $packageName package version $Version"
}

if ($PreviousVersion) {
    $escapedPrevious = [regex]::Escape($PreviousVersion)
    Reject-Match "Cargo.toml" "\b$escapedPrevious\b" "previous version $PreviousVersion"
    Reject-Match "README.md" "\b$escapedPrevious\b" "previous version $PreviousVersion"
    Reject-Match "CLAUDE.md" "\b$escapedPrevious\b" "previous version $PreviousVersion"
    Reject-Match "installer\hematite.iss" "\b$escapedPrevious\b" "previous version $PreviousVersion"
}

if ($RequireCargoLock) {
    Write-Host "Version sync verified for $Version (including Cargo.lock)" -ForegroundColor Green
} else {
    Write-Host "Version sync verified for $Version (static release surfaces)" -ForegroundColor Green
}
