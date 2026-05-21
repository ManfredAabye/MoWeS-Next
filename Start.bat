@echo off
setlocal
cd /d "%~dp0"

if exist "target\release\mowes-ui.exe" (
    echo Starte MoWeS UI ^(Release-EXE^)...
    start "" "target\release\mowes-ui.exe"
    exit /b 0
)

echo Keine Release-EXE gefunden, starte ueber Cargo...
where cargo >nul 2>nul
if errorlevel 1 (
    echo Fehler: Cargo wurde nicht gefunden. Bitte Rust/Cargo installieren.
    pause
    exit /b 1
)

cargo run --bin mowes-ui
set "EC=%ERRORLEVEL%"
if not "%EC%"=="0" (
    echo Start fehlgeschlagen. Exit-Code: %EC%
    pause
)

exit /b %EC%
