# Hematite Version Bumper
# Usage: pwsh ./bump-version.ps1 -Version X.Y.Z

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
$content = $content -replace "(?m)^version = `"$([regex]::Escape($old))`"\r?$", "version = `"$Version`""
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

# installer/hematite.iss fallback version
(Get-Content "installer\hematite.iss" -Raw) `
    -replace [regex]::Escape("#define AppVersion `"$old`""), "#define AppVersion `"$Version`"" |
    Set-Content "installer\hematite.iss" -NoNewline

$verifyScript = Join-Path $PSScriptRoot "scripts\verify-version-sync.ps1"
& $verifyScript -Version $Version -PreviousVersion $old

Write-Host "Done. Files updated:" -ForegroundColor Green
Write-Host "  Cargo.toml, README.md, CLAUDE.md, installer\\hematite.iss"
Write-Host "Verified:"
Write-Host "  Cargo.toml, README badge, and installer fallback are in sync"
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. cargo build"
Write-Host "     (regenerates Cargo.lock - must be included in commit)"
Write-Host "  2. pwsh ./scripts/verify-version-sync.ps1 -Version $Version -RequireCargoLock"
Write-Host "  3. git add Cargo.toml Cargo.lock README.md CLAUDE.md installer\hematite.iss"
Write-Host "     git commit -m 'chore: bump version to $Version'"
Write-Host "     (use exactly these files - never git add .)"
Write-Host "  4. git tag -a v$Version -m 'Release v$Version'"
Write-Host "     git push origin main && git push origin v$Version"
