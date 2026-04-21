# Hematite SearXNG Scaffolder
# Automates the creation of a local SearXNG environment for Windows + Docker.

$ErrorActionPreference = "Stop"

# 1. Target Directory (Desktop by default)
$desktop = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::Desktop)
$targetRoot = Join-Path $desktop "searxng-local"
$searxConfigDir = Join-Path $targetRoot "searxng"

Write-Host "Scaffolding SearXNG at: $targetRoot" -ForegroundColor Cyan

if (-not (Test-Path $targetRoot)) {
    New-Item -ItemType Directory -Path $targetRoot | Out-Null
}
if (-not (Test-Path $searxConfigDir)) {
    New-Item -ItemType Directory -Path $searxConfigDir | Out-Null
}

# 2. Generate Random Secret Key (64-char hex)
$bytes = [byte[]]::new(32)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$rng.GetBytes($bytes)
$secretKey = [System.BitConverter]::ToString($bytes).Replace("-", "").ToLower()

# 3. Create docker-compose.yaml
$composeContent = @"
services:
  searxng:
    container_name: searxng
    image: docker.io/searxng/searxng:latest
    restart: always
    networks:
      - searxng
    ports:
      - "8080:8080"
    volumes:
      - ./searxng:/etc/searxng:rw
    environment:
      - SEARXNG_BASE_URL=http://localhost:8080/
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - SETGID
      - SETUID
    logging:
      driver: "json-file"
      options:
        max-size: "1m"
        max-file: "1"

networks:
  searxng:
    ipam:
      driver: default
"@

Set-Content -Path (Join-Path $targetRoot "docker-compose.yaml") -Value $composeContent -Encoding UTF8

# 4. Create settings.yml
$settingsContent = @"
use_default_settings: true

server:
  secret_key: "$secretKey"
  limiter: false
  image_proxy: true

search:
  safe_search: 0
  autocomplete: ""
  formats:
    - html
    - json

engines:
  # Tier 1: Primary general-purpose (high quality, may rate-limit under heavy use)
  - name: google
    engine: google
    shortcut: g
    use_official_api: false
  - name: duckduckgo
    engine: duckduckgo
    shortcut: ddg
  - name: bing
    engine: bing
    shortcut: b

  # Tier 2: Privacy-first alternatives (rarely rate-limit, good fallbacks)
  - name: brave
    engine: brave
    shortcut: br
  - name: qwant
    engine: qwant
    shortcut: qw
  - name: startpage
    engine: startpage
    shortcut: sp
  - name: mojeek
    engine: mojeek
    shortcut: mj

  # Tier 3: Developer-focused (great for technical queries)
  - name: wikipedia
    engine: wikipedia
    shortcut: wp
  - name: github
    engine: github
    shortcut: gh
  - name: stackoverflow
    engine: stackexchange
    shortcut: so
  - name: npm
    engine: npm
    shortcut: npm
  - name: crates.io
    engine: crates
    shortcut: crio

ui:
  static_use_hash: true
  query_in_title: true
"@

Set-Content -Path (Join-Path $searxConfigDir "settings.yml") -Value $settingsContent -Encoding UTF8

# 5. Create start_searx.bat
$batContent = @"
@echo off
echo Starting SearXNG with 12-engine configuration...
docker compose up -d
echo.
echo SearXNG is now running on port 8080!
timeout /t 5
"@

Set-Content -Path (Join-Path $targetRoot "start_searx.bat") -Value $batContent -Encoding Ascii

Write-Host "`nSUCCESS: SearXNG environment scaffolded!" -ForegroundColor Green
Write-Host "Location: $targetRoot"
Write-Host "`nNext Steps:" -ForegroundColor White
Write-Host "1. Open a terminal in that folder: cd \"$targetRoot\""
Write-Host "2. Start the service: docker compose up -d"
Write-Host "3. Hematite will now auto-detect SearXNG on port 8080!"
Write-Host "`nNote: You can move this folder anywhere on your machine." -ForegroundColor Gray
