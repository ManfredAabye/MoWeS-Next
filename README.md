# MoWeS-Next / MoWeS-Builder

MoWeS-Next ist ein portabler lokaler Server-Stack (Apache + MariaDB), der mit dem Rust-basierten MoWeS-Builder aus ZIP-Komponenten aufgebaut wird.

Diese Datei beschreibt Setup, Start, Build, CLI-Befehle, GUI-Nutzung und Wartung.

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
- Uebersicht mit Prozessstatus, Website-URL und MariaDB-Zugangsdaten

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

Tab `Wartung`:

- Service-Modus vorbereiten
- Update-Manifest schreiben
- Update-Bundle anwenden

Wichtige Verhaltensweisen:

- Beim ersten Start initialisiert das Control Center das MariaDB-Datadir automatisch, falls es noch nicht existiert.
- Beim Start werden die tatsaechlich verwendeten Apache- und MariaDB-Versionen aus den laufenden Binaries ermittelt und als `versions.json` in die Web-Root geschrieben.

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
  "web_root": "./projects",
  "root_password": "root",
  "portable": true,
  "service": false
}
```

Hinweis:

- Wenn `apache_zip`/`mariadb_zip` fehlen, versucht der Builder automatische Erkennung in `Components/`.
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
- `runtime/apache/htdocs/index.html`
- `runtime/apache/htdocs/versions.json`
- `config/default.config.json`
- `Start.bat`, `StartHeadless.bat`
- `builder.manifest.json`
- `THIRD_PARTY_NOTICES.txt`

Hinweis:

- Die ausgelieferte Website wird standardmaessig aus `runtime/apache/htdocs/` bedient.
- `versions.json` enthaelt die tatsaechlich beim Start erkannten Apache- und MariaDB-Versionen.

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

- Pruefen, ob das Datadir unter `dist/.../data/mariadb/` existiert.
- `cargo run --bin mowes-next -- start` erneut ausfuehren.
- Im Control Center `MariaDB Verbindung testen` verwenden.

### Build schlägt fehl

- `cargo check` ausführen
- Fehlermeldungen in der Konsole lesen
- Bei ZIP-Problemen Dateinamen und Inhalt prüfen (`httpd.exe`, `mariadbd.exe`/`mysqld.exe`)

### Dist startet nicht

- Wenn Start.bat oder StartHeadless.bat nicht funktioniert dann nutzen sie ControlGUI.bat

---

Maintainer-Hinweis:

- Das Verzeichnis `target/` ist ein Build-Cache von Cargo und kann bei Bedarf gelöscht werden.
- Für reproduzierbare Releases vor dem Start eine Release-Build erstellen:

```powershell
cargo build --release --bin mowes-ui
```
