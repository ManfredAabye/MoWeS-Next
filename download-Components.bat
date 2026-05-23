@echo off
setlocal EnableExtensions

set "SCRIPT_DIR=%~dp0"
set "COMPONENTS_DIR=%SCRIPT_DIR%Components"

where curl.exe >nul 2>&1
if errorlevel 1 (
	echo [ERROR] curl.exe wurde nicht gefunden. Bitte curl installieren oder den PATH pruefen.
	pause
	exit /b 1
)

if not exist "%COMPONENTS_DIR%" (
	mkdir "%COMPONENTS_DIR%"
	if errorlevel 1 (
		echo [ERROR] Konnte Components-Ordner nicht anlegen: %COMPONENTS_DIR%
		pause
		exit /b 1
	)
)

echo.
echo Lade Components nach:
echo %COMPONENTS_DIR%
echo.

:: Systemkomponenten nicht in den Data Bereich integrieren.
call :download "apache.zip" "https://www.apachelounge.com/download/VS18/binaries/httpd-2.4.67-260504-Win64-VS18.zip"
call :download "mariadb.zip" "https://downloads.mariadb.com/MariaDB/mariadb-10.11.3/winx64-packages/mariadb-10.11.3-winx64.zip"
call :download "php.zip" "https://windows.php.net/downloads/releases/php-8.5.6-nts-Win32-vs17-x64.zip"
:: Webseiten fuer den Data Bereich. Diese koennen auch in den Data Bereich integriert werden, da sie nicht direkt von MoWeS genutzt werden.
call :download "phpMyAdmin.zip" "https://files.phpmyadmin.net/phpMyAdmin/5.2.3/phpMyAdmin-5.2.3-all-languages.zip"
call :download "oswebinterface.zip" "https://github.com/ManfredAabye/oswebinterface/archive/refs/heads/main.zip"
call :download "wordpress.zip" "https://de.wordpress.org/latest-de_DE.zip"

if defined DOWNLOAD_FAILED (
	echo.
	echo [WARN] Mindestens ein Download ist fehlgeschlagen.
	pause
	exit /b 1
)

echo.
echo [OK] Alle Downloads abgeschlossen.
pause
exit /b 0

:download
set "FILE_NAME=%~1"
set "URL=%~2"
set "TARGET=%COMPONENTS_DIR%\%FILE_NAME%"

echo [INFO] %FILE_NAME%
curl.exe -fL --retry 3 --retry-delay 2 -o "%TARGET%" "%URL%"
if errorlevel 1 (
	echo [ERROR] Download fehlgeschlagen: %URL%
	set "DOWNLOAD_FAILED=1"
) else (
	echo [OK] Gespeichert: %TARGET%
)
echo.
goto :eof