@echo off
set "TARGET_DIR=%USERPROFILE%\Desktop\searxng-local"
if not exist "%TARGET_DIR%" (
    echo [ERROR] SearXNG directory not found at: %TARGET_DIR%
    echo Please run scripts\setup-searxng.ps1 first.
    pause
    exit /b 1
)

cd /d "%TARGET_DIR%"
echo Starting SearXNG with 12-engine configuration (Universal)...
docker compose up -d
echo.
echo SearXNG is now running on port 8080!
timeout /t 5
