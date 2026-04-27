@echo off
chcp 65001 >nul 2>&1
echo.
echo ========================================
echo   Iris Runtime - Vue Demo
echo ========================================
echo.

echo Starting Iris Runtime with Vue Demo...
echo.

cd /d "%~dp0"

if "%1"=="build" (
    echo Running build command...
    ..\..\target\release\iris-runtime.exe build
) else if "%1"=="info" (
    echo Running info command...
    ..\..\target\release\iris-runtime.exe info
) else (
    echo Starting development server...
    echo.
    ..\..\target\release\iris-runtime.exe dev --port 3000
)

echo.
pause
