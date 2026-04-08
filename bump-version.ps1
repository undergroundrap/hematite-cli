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

# release.ps1 default param
(Get-Content "release.ps1" -Raw) `
    -replace [regex]::Escape("`"$old`""), "`"$Version`"" |
    Set-Content "release.ps1" -NoNewline

Write-Host "Done. Files updated:" -ForegroundColor Green
Write-Host "  Cargo.toml, README.md, CLAUDE.md, release.ps1"
Write-Host ""
Write-Host "Next: cargo build to verify, then git commit and git tag v$Version"
