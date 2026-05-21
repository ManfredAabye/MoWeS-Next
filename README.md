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

## Voraussetzungen

- Windows
- Rust/Cargo installiert
- Apache- und MariaDB-ZIP-Dateien

ZIP-Dateien entweder:

- in `Components/` ablegen
- oder in der GUI direkt auswählen
Tabs:

## Programm starten

### Einfachster Weg

- `Start.bat` per Doppelklick starten

Das Script startet:

- zuerst `target/release/mowes-ui.exe` (falls vorhanden)
- sonst `cargo run --bin mowes-ui`
- `Betrieb`: Start/Stop/Restart
- `Wartung`: Service vorbereiten, Update-Manifest, Update-Bundle anwenden

Zusätzlich:

### Start im Terminal

```powershell
Set-Location D:\MoWeS-Next
cargo run --bin mowes-ui
```text
- Statuskarten (Apache, MariaDB, Service, Plugins, Update)
- Übersichtslinien mit Details

### MoWeS Builder (`mowes-builder`)

- ZIP-Auswahl für Apache/MariaDB
- Paketbau
## Erste Schritte

1. GUI öffnen (`mowes-ui`)
2. Tab `Build`:
  - `Build from Components` oder
  - `Select preset` + `Build from Preset`
3. Tab `Betrieb`:
  - `Start`
4. Bei Problemen:
  - `Refresh Overview` und Statuskarten prüfen
- Diagnose

## 6) CLI-Befehle

Alle Befehle im Projektroot ausführen:
## Wichtige Befehle (CLI)

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

```powershell
Verfügbare Commands:

- `build` oder `build-package`:
### Häufige Probleme

### `Start.bat` startet nicht

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

- Baut portables Paket aus `Components/`
- `build-installable`:
  - Baut installierbare Variante zusätzlich
- `build-preset <file>`:
  - Baut aus Preset-JSON
- `build-installable-preset <file>`:
  - Installable-Build aus Preset-JSON
- `start`:
  - Startet Apache + MariaDB
- `stop`:
  - Stoppt Apache + MariaDB
- `restart`:
  - Restart Apache + MariaDB
- `status`:
  - Zeigt Paket-/Log-/Prozessstatus
- `doctor`:
  - Diagnosechecks für Build/Runtime
- `service-prepare`:
  - Legt Service-Konfigurationsdateien an
- `service-status`:
  - Zeigt Service-Konfigurationsstatus
- `plugins`:
  - Listet Plugins aus `plugins/*.json`
- `update-check`:
  - Prüft lokales Update-Manifest
- `update-prepare [version]`:
  - Schreibt/aktualisiert `updates/update.manifest.json`
- `update-apply <dir>`:
  - Wendet Update-Bundle aus Verzeichnis an
- `--headless` / `headless`:
  - Status + Doctor ohne GUI

## 7) Presets (JSON)

Beispiel: `presets/default.json`

```json
{
  "package_name": "mowes-next-package",
  "apache_zip": "Components/apache.zip",
  "mariadb_zip": "Components/mariadb.zip",
  "http_port": 8080,
  "db_port": 3306,
  "php_version": "8.3",
  "php_extensions": ["mysqli", "gd", "curl"],
  "root_password": "root",
  "portable": true,
  "service": false
}
```

Hinweis:

- Wenn `apache_zip`/`mariadb_zip` fehlen, versucht der Builder automatische Erkennung in `Components/`.

## 8) Build-Ausgaben

### Portables Paket

- `dist/mowes-next-package/`

Enthält u. a.:

- `runtime/` (Apache, MariaDB, generated configs)
- `config/default.config.json`
- `Start.bat`, `StartHeadless.bat`
- `builder.manifest.json`
- `THIRD_PARTY_NOTICES.txt`

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
