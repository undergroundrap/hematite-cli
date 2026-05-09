# Installs the Hematite git hooks into .git/hooks/.
# Run once after cloning: pwsh ./scripts/install-hooks.ps1

$hooksDir = Join-Path $PSScriptRoot "..\\.git\\hooks"
$sourceDir = Join-Path $PSScriptRoot "..\\scripts\\hooks"

if (-not (Test-Path $sourceDir)) {
    Write-Host "No hooks source directory found at $sourceDir" -ForegroundColor Yellow
    exit 0
}

foreach ($hook in Get-ChildItem $sourceDir) {
    $dest = Join-Path $hooksDir $hook.Name
    Copy-Item $hook.FullName $dest -Force
    Write-Host "Installed hook: $($hook.Name)" -ForegroundColor Green
}

Write-Host "Git hooks installed." -ForegroundColor Green
