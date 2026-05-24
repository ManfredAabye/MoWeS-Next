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

:: Weitere Komponenten (teils Source-Archive, ggf. mit zusaetzlichem Setup)
call :download "strapi.zip" "https://github.com/strapi/strapi/archive/refs/heads/main.zip"
call :download "directus.zip" "https://github.com/directus/directus/archive/refs/heads/main.zip"
call :download "ghost.zip" "https://github.com/TryGhost/Ghost/archive/refs/heads/main.zip"

call :download "joomla.zip" "https://github.com/joomla/joomla-cms/releases/download/6.1.1-rc1/Joomla_6.1.1-rc1-Release_Candidate-Full_Package.zip"
call :download "drupal.zip" "https://ftp.drupal.org/files/projects/drupal-11.2.0.zip"
call :download "typo3.zip" "https://github.com/TYPO3/typo3/archive/refs/heads/main.zip"

call :download "phpbb.zip" "https://download.phpbb.com/pub/release/3.3/3.3.15/phpBB-3.3.15.zip"
call :download "mybb.zip" "https://resources.mybb.com/downloads/mybb_1839.zip"
call :download "discourse.zip" "https://github.com/discourse/discourse/archive/refs/heads/main.zip"
call :download "flarum.zip" "https://github.com/flarum/flarum/archive/refs/tags/v2.0.0-rc.1.zip"

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