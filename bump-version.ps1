# Hematite Version Bumper
# Usage: pwsh ./bump-version.ps1 -Version 0.2.0

param(
    [Parameter(Mandatory)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$old = (Select-String -Path "Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value

if ($old -eq $Version) {
    Write-Host "Already at $Version, nothing to do." -ForegroundColor Yellow
    exit 0
}

Write-Host "Bumping $old -> $Version" -ForegroundColor Cyan

# Cargo.toml (first occurrence = package version, not deps)
$content = Get-Content "Cargo.toml" -Raw
$content = $content -replace "^version = `"$old`"", "version = `"$Version`""
Set-Content "Cargo.toml" $content -NoNewline

# README.md
(Get-Content "README.md" -Raw) `
    -replace [regex]::Escape("version-$old"), "version-$Version" `
    -replace [regex]::Escape("Hematite-$old"), "Hematite-$Version" `
    -replace [regex]::Escape("v$old"), "v$Version" |
    Set-Content "README.md" -NoNewline

# CLAUDE.md
(Get-Content "CLAUDE.md" -Raw) `
    -replace [regex]::Escape("Hematite-$old"), "Hematite-$Version" |
    Set-Content "CLAUDE.md" -NoNewline

Write-Host "Done. Files updated:" -ForegroundColor Green
Write-Host "  Cargo.toml, README.md, CLAUDE.md"
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. cargo build          - verify it compiles (also regenerates Cargo.lock)"
Write-Host "  2. git add Cargo.toml Cargo.lock README.md CLAUDE.md"
Write-Host "     git commit -m 'chore: bump version to $Version'"
Write-Host "  3. pwsh ./scripts/package-windows.ps1 -AddToPath"
