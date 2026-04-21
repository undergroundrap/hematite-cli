@echo off
set "TARGET_DIR=%HEMATITE_SEARX_ROOT%"
if "%TARGET_DIR%"=="" set "TARGET_DIR=%USERPROFILE%\.hematite\searxng-local"
if not exist "%TARGET_DIR%" if exist "%USERPROFILE%\Desktop\searxng-local" set "TARGET_DIR=%USERPROFILE%\Desktop\searxng-local"
if not exist "%TARGET_DIR%" (
    echo [ERROR] SearXNG directory not found at: %TARGET_DIR%
    echo Please run scripts\setup-searxng.ps1 first, or set HEMATITE_SEARX_ROOT.
    pause
    exit /b 1
)

cd /d "%TARGET_DIR%"
echo Starting SearXNG with safer technical search profile...
docker compose up -d
echo.
echo SearXNG is now running on port 8080!
timeout /t 5
