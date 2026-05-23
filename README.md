# MoWeS-Next / MoWeS-Builder

MoWeS-Next ist ein portabler lokaler Server-Stack (Apache + MariaDB), der mit dem Rust-basierten MoWeS-Builder aus ZIP-Komponenten aufgebaut wird.

Diese Datei beschreibt Setup, Start, Build, CLI-Befehle, GUI-Nutzung und Wartung.

## Schnell-Checkliste Release

1. `cargo run --bin mowes-next -- build-preset presets/mowes-next-os.json`
1. Paketgroesse pruefen: `dist/mowes-next-os` (Ziel: <= 90 MB)
1. `dist/mowes-next-os/` komplett weitergeben (USB/ZIP); Datadir initialisiert sich beim ersten Start automatisch.

## Inhalt

1. Voraussetzungen
2. Schnellstart
3. Projektstruktur
4. Startvarianten
5. GUI-Anwendungen
6. CLI-Befehle
7. Presets (JSON)
8. Build-Ausgaben
9. Service-Modus
10. Plugin-System
11. Update-System
12. Logs und Diagnose
13. Troubleshooting
14. Release-Workflow (USB/Weitergabe)

## 1) Voraussetzungen

- Windows
- Rust Toolchain (cargo verfügbar)
- Komponenten-ZIP-Dateien für Apache und MariaDB

Empfohlen:

- ZIP-Dateien in `Components/` ablegen
- Preset-Datei unter `presets/default.json` pflegen

## 2) Schnellstart

### Variante A: Start per Batch

- Doppelklick auf `Start.bat`

Verhalten:

- Startet zuerst `target/release/mowes-ui.exe`, falls vorhanden
- Fällt sonst auf `cargo run --bin mowes-ui` zurück

### Variante B: Direkt im Terminal

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-ui
```

### Erster kompletter Lauf

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-next -- build
cargo run --bin mowes-next -- doctor
cargo run --bin mowes-next -- start
```

Hinweis:

- Das Control Center zeigt nach Build/Start die exakte Website-URL und die MariaDB-Zugangsdaten an.
- Bei Port-Konflikten bevorzugt lokal `http://localhost:<port>/` statt `http://127.0.0.1:<port>/` testen.

## 3) Projektstruktur

Wichtige Verzeichnisse:

- `Components/` ZIP-Quellen für Apache/MariaDB
- `presets/` wiederverwendbare Build-Konfigurationen als JSON
- `dist/` erzeugte Pakete
- `logs/` Builder-Logs (JSONL)
- `temp/pids/` PID-Dateien für Prozesssteuerung
- `plugins/` Plugin-Deskriptoren (`*.json`)
- `updates/` Update-Manifest und angewendete Bundles
- `target/` Rust-Build-Artefakte (debug/release)

## 4) Startvarianten

### GUI Control Center

```powershell
cargo run --bin mowes-ui
```

### Builder-GUI

```powershell
cargo run --bin mowes-builder
```

### CLI

```powershell
cargo run --bin mowes-next -- status
```

## 5) GUI-Anwendungen

### MoWeS-Next Control Center (`mowes-ui`)

Start:

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-ui
```

Aktueller Funktionsumfang:

- grosses Startfenster fuer bessere Sichtbarkeit
- Statuskarten fuer Apache, MariaDB, Service, Plugins und Update
- Status-Detailzeile fuer PHP (php-cgi/php.ini/mysqli)
- Uebersicht mit Prozessstatus, Website-URL und MariaDB-Zugangsdaten
- Anzeige konfigurierter Datenbanken aus der Paketkonfiguration

Tab `Build`:

- Preset auswaehlen
- HTTP-Port und DB-Port direkt eingeben
- Live-Validierung fuer freie/belegte/ungueltige Ports
- Exportverzeichnis fuer `dist` frei waehlen
- `Build from Components`
- `Build from Preset`
- `Build Installable`
- `Build Installable from Preset`

Tab `Betrieb`:

- `Start`, `Stop`, `Restart`
- `Refresh Overview`
- `Index im Browser oeffnen`
- `MariaDB Verbindung testen`
- Bereich `Zugriffsdaten` mit:
- Website-URL
- direkter `index.html`-URL
- MariaDB Host, Port, User, Passwort, Datenbankname
- konfigurierter Datenbankliste (z. B. `opensim`, `robust`)

Tab `Wartung`:

- Service-Modus vorbereiten
- Update-Manifest schreiben
- Update-Bundle anwenden

Wichtige Verhaltensweisen:

- Beim ersten Start initialisiert das Control Center das MariaDB-Datadir automatisch, falls es noch nicht existiert.
- Beim Start werden die PHP-Tempordner (`runtime/php/tmp/sessions`, `runtime/php/tmp/upload`) automatisch angelegt.
- Beim Start werden die tatsaechlich verwendeten Apache- und MariaDB-Versionen aus den laufenden Binaries ermittelt und als `versions.json` in die Web-Root geschrieben.
- Beim Start wird zusaetzlich ein DB-Report ausgegeben: `erstellt (...)`, `bereits vorhanden (...)` oder gemischt.
- Das paketinterne `ControlGUI.ps1` zeigt Statusmeldungen farblich (OK/INFO/ERR) an.

### MoWeS Builder (`mowes-builder`)

Start:

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-builder
```

Fokus:

- ZIP-Auswahl fuer Apache und MariaDB
- Paketbau

## 6) CLI-Befehle

Alle Befehle im Projektroot ausführen:

### Wichtige Befehle

```powershell
Set-Location D:\MoWeS-Next

# Paket bauen
cargo run --bin mowes-next -- build

# Server starten/stoppen
cargo run --bin mowes-next -- start
cargo run --bin mowes-next -- stop

# Status und Diagnose
cargo run --bin mowes-next -- status
cargo run --bin mowes-next -- doctor
```

### Verfuegbare Commands

- `build` oder `build-package`: Baut ein portables Paket aus `Components/`.
- `build-installable`: Baut zusaetzlich eine installierbare Variante.
- `build-preset <file>`: Baut aus einer Preset-JSON.
- `build-installable-preset <file>`: Baut eine installierbare Variante aus einer Preset-JSON.
- `start`: Startet Apache und MariaDB.
- `stop`: Stoppt Apache und MariaDB.
- `restart`: Neustart von Apache und MariaDB.
- `status`: Zeigt Paket-, Log- und Prozessstatus.
- `doctor`: Fuehrt Diagnosechecks fuer Build und Runtime aus.
- `service-prepare`: Legt Service-Konfigurationsdateien an.
- `service-status`: Zeigt den Service-Konfigurationsstatus an.
- `plugins`: Listet Plugins aus `plugins/*.json`.
- `update-check`: Prueft das lokale Update-Manifest.
- `update-prepare [version]`: Schreibt oder aktualisiert `updates/update.manifest.json`.
- `update-apply <dir>`: Wendet ein Update-Bundle aus einem Verzeichnis an.
- `--headless` oder `headless`: Zeigt Status und Doctor ohne GUI an.

### Haeufige Probleme

### GUI startet nicht ueber `Start.bat`

- Prüfen: `cargo --version`
- Falls keine Release-EXE vorhanden ist, startet das Script automatisch über Cargo

### `start` findet Apache/MariaDB nicht

- Erst Build ausführen (`build` oder GUI-Build)
- ZIP-Dateien prüfen (Apache/MariaDB, gültige Archive)

### Build-Fehler

```powershell
Set-Location D:\MoWeS-Next
cargo check
```

## 7) Presets (JSON)

Beispiel: `presets/default.json`

```json
{
  "package_name": "mowes-next-package",
  "output_base_dir": "dist",
  "apache_zip": "Components/apache.zip",
  "mariadb_zip": "Components/mariadb.zip",
  "http_port": 8080,
  "db_port": 3306,
  "php_version": "8.3",
  "php_extensions": ["mysqli", "gd", "curl"],
  "database_name": "mowes",
  "database_names": ["mowes", "robust"],
  "database_connections": {
    "robust": "Data Source=localhost;Database=robust;User ID=opensim;Password=***",
    "opensim": "Data Source=localhost;Database=opensim;User ID=opensim;Password=***"
  },
  "web_root": "./Data/http",
  "extra_components": [
    {
      "name": "WordPress",
      "zip_path": "Components/latest-de_DE.zip",
      "target_subdir": "wordpress"
    },
    {
      "name": "phpMyAdmin",
      "zip_path": "Components/phpMyAdmin-5.2.3-all-languages.zip",
      "target_subdir": "phpmyadmin"
    },
    {
      "name": "oswebinterface",
      "zip_path": "Components/oswebinterface-main.zip",
      "target_subdir": "oswebinterface"
    }
  ],
  "root_password": "root",
  "portable": true,
  "service": false
}
```

Hinweis:

- Wenn `apache_zip`/`mariadb_zip` fehlen, versucht der Builder automatische Erkennung in `Components/`.
- Das Control Center erkennt zusaetzliche `*.zip` in `Components/` (ausser Apache/MariaDB) automatisch als auswaehlbare Zusatz-Components.
- Ausgewaehlte Zusatz-Components werden beim Build in den Web-Root integriert (`<web_root>/<target_subdir>`).
- Zusatz-Components werden in den konfigurierten Web-Root entpackt (`<web_root>/<target_subdir>`, standardmaessig `Data/http/<target_subdir>`).
- Komponentenarchive duerfen nur aus `Components/` geladen werden; Pfade ausserhalb werden abgewiesen.
- Wenn eine ausgewaehlte Zusatz-Component als WordPress erkannt wird, ergaenzt das Control Center automatisch die Datenbank `wordpress` in der effektiven Build-Konfiguration.
- Wenn dein Apache-ZIP **kein** `mod_php*.so` mitbringt, wird zusaetzlich ein PHP-ZIP mit `php-cgi.exe` in `Components/` benoetigt (Dateiname mit `php`, aber nicht `phpmyadmin`).
- Das Apache-ZIP muss PHP auch ausfuehren koennen: entweder `mod_php*.so` oder die Kombination `mod_actions.so` + `mod_cgi.so`.
- Beim Entpacken werden nur fuer den Betrieb notwendige Runtime-Pfade aus Apache/MariaDB uebernommen (kein Voll-Export des gesamten Archivs).
- Apache wird auf HTML/PHP-Betrieb reduziert; nicht benoetigte Module und Zusatzverzeichnisse werden entfernt.
- Die Runtime-`php.ini` wird automatisch erzeugt (inkl. `php_extensions`) und auf notwendige Erweiterungen reduziert.
- PHP-Sessions/Temp verwenden standardmaessig `runtime/php/tmp` statt `temp/`, damit Session-Dateien nicht durch Release-Cleanup verloren gehen.
- MariaDB wird ohne Backup/Restore-Werkzeuge ausgeliefert; in `Data/SQL` bleiben nur erforderliche Datenbankstrukturen (z. B. ohne `test`).
- Nach jedem Build werden `Data/SQL`, `logs/` und `temp/` automatisch bereinigt (Release-Cleanup fuer kleine Paketgroesse).
- Das SQL-Datadir ist im ausgelieferten Paket absichtlich leer und wird beim ersten Start automatisch neu initialisiert.
- `output_base_dir` ist optional. Ohne Angabe wird nach `./dist` gebaut.
- Das Control Center kann HTTP-Port, DB-Port und Exportverzeichnis auch ohne manuelle Preset-Aenderung ueberschreiben.
- Die Versionsnummer für die Beispiel-Startseite wird aus der Datei `version` im Projekt-Root gelesen.
- Die Datei `version` wird automatisch beim Cargo-Build mit der Paketversion aus `Cargo.toml` synchronisiert (`build.rs`).

## 8) Build-Ausgaben

### Portables Paket

- `dist/mowes-next-package/`

Oder bei abweichender Konfiguration:

- `<output_base_dir>/<package_name>/`

Enthält u. a.:

- `runtime/` (Apache, MariaDB, generated configs)
- `Data/http/versions.json`
- `runtime/php/php.ini`
- `runtime/php/tmp/sessions/`
- `Data/http/index.html`
- `config/default.config.json`
- `Start.bat`, `StartHeadless.bat`
- `builder.manifest.json`
- `THIRD_PARTY_NOTICES.txt`

Hinweis:

- Die ausgelieferte Website wird standardmaessig aus `Data/http/` bedient (DocumentRoot).
- `versions.json` enthaelt die tatsaechlich beim Start erkannten Apache- und MariaDB-Versionen.
- `index.html` zeigt zusaetzlich die konfigurierten Datenbanken aus dem Preset/Config an.
- Das Paket wird nach dem Build fuer den Versand verkleinert; Laufzeitdaten werden erst beim ersten Start erzeugt.

### Installierbare Variante

- `dist/mowes-next-installable/`

Zusätzlich:

- `install/install.bat`
- `install/uninstall.bat`
- `INSTALLABLE.txt`

## 9) Service-Modus

Vorbereitung:

```powershell
cargo run --bin mowes-next -- service-prepare
```

Erzeugt:

- `service/service-mode.json`
- `service/install-service.bat`
- `service/uninstall-service.bat`

Status prüfen:

```powershell
cargo run --bin mowes-next -- service-status
```

## 10) Plugin-System

Plugin-Deskriptoren liegen unter:

- `plugins/*.json`

Liste anzeigen:

```powershell
cargo run --bin mowes-next -- plugins
```

## 11) Update-System

### Manifest schreiben

```powershell
cargo run --bin mowes-next -- update-prepare 26.05.1
```

### Update prüfen

```powershell
cargo run --bin mowes-next -- update-check
```

### Update-Bundle anwenden

```powershell
cargo run --bin mowes-next -- update-apply D:\Pfad\zum\Bundle
```

Ergebnis wird nach `updates/applied/<version>/` kopiert.

## 12) Logs und Diagnose

- Builder-Log: `logs/builder.jsonl`
- Diagnose:

```powershell
cargo run --bin mowes-next -- doctor
```

## 13) Troubleshooting

### `Start.bat` startet nicht

- Prüfen, ob Rust/Cargo installiert ist (`cargo --version`)
- Prüfen, ob `target/release/mowes-ui.exe` existiert
- Notfalls direkt starten:

```powershell
cargo run --bin mowes-ui
```

### `start` meldet fehlende Executables

- Erst Build ausführen (`build` oder `build-preset`)
- Sicherstellen, dass Apache/MariaDB-ZIPs korrekt sind
- `doctor` ausführen und Fehlerdetails prüfen

### `127.0.0.1` liefert 404, `localhost` funktioniert aber

- Es kann ein lokaler Port-Konflikt mit einer anderen Anwendung vorliegen.
- Im Zweifel `http://localhost:<port>/` testen.
- HTTP-Port im Control Center auf einen freien Port aendern und Paket neu bauen.

### MariaDB startet nicht manuell aus dem Projektroot

- `mariadbd.exe --defaults-file=...` direkt aus dem falschen Arbeitsverzeichnis kann wegen relativer Pfade fehlschlagen.
- Stattdessen Start ueber `cargo run --bin mowes-next -- start` oder ueber das Control Center verwenden.
- Das Projekt initialisiert das Datadir bei Bedarf automatisch.

### MariaDB erscheint nicht im Task-Manager

- Ein leeres `dist/.../Data/SQL/` direkt nach dem Build ist normal (Release-Cleanup).
- `cargo run --bin mowes-next -- start` erneut ausfuehren.
- Im Control Center `MariaDB Verbindung testen` verwenden.

### phpMyAdmin meldet Session-Fehler (`session_start`)

- Sicherstellen, dass die Runtime-Pfade existieren:
  - `dist/<package>/runtime/php/tmp/sessions`
  - `dist/<package>/runtime/php/tmp/upload`
- In `dist/<package>/runtime/php/php.ini` pruefen, ob gesetzt ist:
  - `session.save_path=.../runtime/php/tmp/sessions`
  - `sys_temp_dir=.../runtime/php/tmp`
- Server ueber Control Center oder `cargo run --bin mowes-next -- start` neu starten (legt fehlende Ordner automatisch an).

### Build schlägt fehl

- `cargo check` ausführen
- Fehlermeldungen in der Konsole lesen
- Bei ZIP-Problemen Dateinamen und Inhalt prüfen (`httpd.exe`, `mariadbd.exe`/`mysqld.exe`)

---

Maintainer-Hinweis:

- Das Verzeichnis `target/` ist ein Build-Cache von Cargo und kann bei Bedarf gelöscht werden.
- Für reproduzierbare Releases vor dem Start eine Release-Build erstellen:

```powershell
cargo build --release --bin mowes-ui
```

## 14) Release-Workflow (USB/Weitergabe)

Ziel: Ein schlankes, portables Paket fuer Weitergabe an Dritte (z. B. USB-Stick), ohne Laufzeitreste aus deiner lokalen Session.

1. Preset bauen:

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-next -- build-preset presets/mowes-next-os.json
```

1. Groesse pruefen (Ziel z. B. <= 90 MB):

```powershell
$p='D:\MoWeS-Next\dist\mowes-next-os'
$size=(Get-ChildItem $p -Recurse -File | Measure-Object Length -Sum).Sum
[math]::Round($size/1MB,2)
```

1. Ergebnis verifizieren:

- `Data/SQL`, `logs/` und `temp/` sind direkt nach dem Build leer (automatischer Release-Cleanup).
- `runtime/` und `config/default.config.json` sind enthalten.

1. Optionaler Smoke-Test vor Versand:

```powershell
cargo run --bin mowes-next -- start
cargo run --bin mowes-next -- stop
```

1. Weitergabe:

- Verzeichnis `dist/mowes-next-os/` komplett auf den USB-Stick oder in ein ZIP fuer Dritte kopieren.
- Beim ersten Start initialisiert MariaDB das leere Datadir automatisch und legt konfigurierte Datenbanken an.
