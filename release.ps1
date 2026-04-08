# Hematite Release Builder
# Usage: pwsh ./release.ps1
# Builds a release binary, updates the portable distribution, and optionally adds to PATH.

param(
    [string]$Version   = "0.1.0",
    [switch]$AddToPath           # Pass -AddToPath to register hematite in user PATH
)

$ErrorActionPreference = "Stop"
$PortableDir = "dist\windows\Hematite-$Version-portable"
$ZipPath     = "dist\windows\Hematite-$Version-portable.zip"

Write-Host "Building release binary..."
cargo build --release
if ($LASTEXITCODE -ne 0) { Write-Error "cargo build --release failed"; exit 1 }

Write-Host "Copying files to $PortableDir..."
New-Item -ItemType Directory -Path $PortableDir -Force | Out-Null
Copy-Item "target\release\hematite.exe"  "$PortableDir\hematite.exe"  -Force
Copy-Item "target\release\DirectML.dll"  "$PortableDir\DirectML.dll"  -Force

Write-Host "Zipping portable..."
Compress-Archive -Path $PortableDir -DestinationPath $ZipPath -Force

$size = (Get-Item $ZipPath).Length / 1MB
Write-Host ""
Write-Host "Done. $ZipPath ($([math]::Round($size))MB)"
Write-Host "  hematite.exe  — $(([math]::Round((Get-Item "$PortableDir\hematite.exe").Length / 1MB)))MB"
Write-Host "  DirectML.dll  — $(([math]::Round((Get-Item "$PortableDir\DirectML.dll").Length / 1MB)))MB"

# ── PATH registration ────────────────────────────────────────────────────────
$AbsPortableDir = (Resolve-Path $PortableDir).Path

if ($AddToPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$AbsPortableDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$AbsPortableDir", "User")
        Write-Host ""
        Write-Host "Added to user PATH: $AbsPortableDir"
        Write-Host "Restart your terminal (or IDE) for 'hematite' to be available everywhere."
    } else {
        Write-Host ""
        Write-Host "Already on PATH: $AbsPortableDir"
    }
} else {
    Write-Host ""
    Write-Host "Tip: run with -AddToPath to register 'hematite' in your user PATH"
    Write-Host "     pwsh ./release.ps1 -AddToPath"
}
