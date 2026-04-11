[CmdletBinding(DefaultParameterSetName = "ByBump")]
param(
    [Parameter(Mandatory, ParameterSetName = "ByVersion")]
    [string]$Version,

    [Parameter(Mandatory, ParameterSetName = "ByBump")]
    [ValidateSet("patch", "minor", "major")]
    [string]$Bump,

    [switch]$Push,
    [switch]$AddToPath,
    [switch]$SkipInstaller
)

$ErrorActionPreference = "Stop"

$repoRoot = $PSScriptRoot
Set-Location $repoRoot

function Invoke-Step([string]$Label, [scriptblock]$Action) {
    Write-Host ""
    Write-Host "==> $Label" -ForegroundColor Cyan
    & $Action
}

function Get-CurrentVersion {
    $cargoToml = Get-Content -LiteralPath "Cargo.toml" -Raw
    $match = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Could not determine package version from Cargo.toml."
    }
    $match.Groups[1].Value
}

function Get-BumpedVersion([string]$CurrentVersion, [string]$BumpKind) {
    $parts = $CurrentVersion.Split('.')
    if ($parts.Length -ne 3) {
        throw "Unsupported semantic version format: $CurrentVersion"
    }

    $major = [int]$parts[0]
    $minor = [int]$parts[1]
    $patch = [int]$parts[2]

    switch ($BumpKind) {
        "patch" { $patch += 1 }
        "minor" { $minor += 1; $patch = 0 }
        "major" { $major += 1; $minor = 0; $patch = 0 }
        default { throw "Unknown bump kind: $BumpKind" }
    }

    "$major.$minor.$patch"
}

function Ensure-CleanWorktree {
    $status = git status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "git status failed."
    }
    if ($status) {
        throw "Release flow requires a clean git worktree. Commit or stash existing changes first."
    }
}

function Ensure-TagDoesNotExist([string]$TagName) {
    $existing = git tag --list $TagName
    if ($LASTEXITCODE -ne 0) {
        throw "git tag --list failed."
    }
    if ($existing) {
        throw "Tag $TagName already exists."
    }
}

function Invoke-CargoBuild {
    $previousOffline = $env:CARGO_NET_OFFLINE
    if (Test-Path Env:CARGO_NET_OFFLINE) {
        Remove-Item Env:CARGO_NET_OFFLINE
    }

    try {
        & cargo build
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed."
        }
    } finally {
        if ($null -ne $previousOffline) {
            $env:CARGO_NET_OFFLINE = $previousOffline
        }
    }
}

function Invoke-WindowsPackage([bool]$IncludeInstaller, [bool]$RegisterPath) {
    $args = @("-ExecutionPolicy", "Bypass", "-File", ".\scripts\package-windows.ps1")
    if ($IncludeInstaller) {
        $args += "-Installer"
    }
    if ($RegisterPath) {
        $args += "-AddToPath"
    }

    & powershell @args
    if ($LASTEXITCODE -ne 0) {
        throw "Windows packaging failed."
    }
}

function Invoke-UnixPackage {
    & bash ./scripts/package-unix.sh
    if ($LASTEXITCODE -ne 0) {
        throw "Unix packaging failed."
    }
}

$currentVersion = Get-CurrentVersion
if ($PSCmdlet.ParameterSetName -eq "ByBump") {
    $Version = Get-BumpedVersion -CurrentVersion $currentVersion -BumpKind $Bump
}

$tagName = "v$Version"

Write-Host "Preparing release $Version from $currentVersion" -ForegroundColor Yellow
if ($Version -eq $currentVersion) {
    throw "Target version matches the current version. Pick a new version or bump kind."
}

Invoke-Step "Checking worktree state" {
    Ensure-CleanWorktree
    Ensure-TagDoesNotExist $tagName
}

Invoke-Step "Bumping version metadata" {
    & powershell -ExecutionPolicy Bypass -File .\bump-version.ps1 -Version $Version
    if ($LASTEXITCODE -ne 0) {
        throw "Version bump failed."
    }
}

Invoke-Step "Rebuilding debug artifacts and Cargo.lock" {
    Invoke-CargoBuild
}

Invoke-Step "Verifying version sync" {
    & powershell -ExecutionPolicy Bypass -File .\scripts\verify-version-sync.ps1 -Version $Version -RequireCargoLock
    if ($LASTEXITCODE -ne 0) {
        throw "Version sync verification failed."
    }
}

Invoke-Step "Creating version bump commit" {
    git add Cargo.toml Cargo.lock README.md CLAUDE.md installer/hematite.iss
    if ($LASTEXITCODE -ne 0) {
        throw "git add failed."
    }

    git commit -m "chore: bump version to $Version"
    if ($LASTEXITCODE -ne 0) {
        throw "git commit failed."
    }
}

Invoke-Step "Creating release tag" {
    git tag -a $tagName -m "Release $tagName"
    if ($LASTEXITCODE -ne 0) {
        throw "git tag failed."
    }
}

Invoke-Step "Building release artifacts" {
    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        Invoke-WindowsPackage -IncludeInstaller:(-not $SkipInstaller) -RegisterPath:$AddToPath
    } else {
        Invoke-UnixPackage
    }
}

if ($Push) {
    Invoke-Step "Pushing commit and tag" {
        git push origin main
        if ($LASTEXITCODE -ne 0) {
            throw "git push origin main failed."
        }

        git push origin $tagName
        if ($LASTEXITCODE -ne 0) {
            throw "git push origin $tagName failed."
        }
    }
} else {
    Write-Host ""
    Write-Host "Release $Version is ready locally." -ForegroundColor Green
    Write-Host "Push when ready:" -ForegroundColor Green
    Write-Host "  git push origin main"
    Write-Host "  git push origin $tagName"
}
