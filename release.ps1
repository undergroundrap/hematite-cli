# Hematite Release Builder
# Usage: pwsh ./release.ps1
# Builds a release binary and updates the portable distribution in dist/windows/.

param(
    [string]$Version = "0.1.0"
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
