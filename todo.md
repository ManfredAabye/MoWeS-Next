# Projektbeschreibung – MoWeS-Builder

Ich benötige einen **MoWeS-Builder**, entwickelt in der Programmiersprache Rust für Windows.

Der Builder soll aus bereitgestellten ZIP-Archiven im Verzeichnis `Components` automatisch einen betriebsfertigen lokalen Server erstellen. Die ZIP-Archive enthalten die benötigten Komponenten wie Apache und MariaDB.

## Komponenten

## MoWeS-Builder

GUI-basierte Anwendung zur Erstellung und Konfiguration eines **MoWeS-Next Servers**.

## MoWeS-Next

Vorkonfigurierter, portabler lokaler Server, der einer Software direkt beigefügt werden kann.
Der Server soll:

* portable ausführbar sein
* über CLI von externer Software gestartet werden können
* optional über eine GUI gestartet und verwaltet werden können

## Enthaltene Server-Komponenten

* Apache Webserver
* MariaDB Datenbankserver
* vorkonfigurierte Datenbanken
* Rust-basierte CLI-Verwaltung
* GUI-Starter für einfache Bedienung
* automatische Konfiguration aller benötigten Komponenten
* Erstellung einer vollständig einsatzbereiten Serverumgebung
* automatische Port-Erkennung
* PHP-Unterstützung
* Logging
* Service-Modus
* Plugin-System
* Update-System
* portable vs. installierbare Version

## Ziel

Das Ziel ist ein vollständig vorkonfiguriertes, portables Entwicklungs- und Serverpaket mit dem Namen **MoWeS-Next**.

Der Builder selbst trägt den Namen **MoWeS-Builder**.

## Technologien

* Programmiersprache: Rust
* Betriebssystem: Windows
* Komponentenbereitstellung über ZIP-Archive

## Projekt-Status (TODO-Liste)

### Benoetigt (Anforderungen laut Zielbild)

* [x] MoWeS-Builder als GUI zur Paket-Erstellung
* [x] Automatische Verarbeitung von ZIP-Archiven aus Components
* [x] Portabler MoWeS-Next Server (Start per GUI und CLI)
* [x] Apache + MariaDB Runtime inklusive vorkonfigurierter Struktur
* [x] Automatische Konfiguration (Ports, Pfade, Startskripte)
* [x] Logging und Health-Status
* [x] Service-Modus
* [x] Plugin-System
* [x] Update-System
* [x] Portable und installierbare Auslieferungsvariante

### Bereits eingefuegt (neues Projekt im Workspace-Root)

* [x] Neues Rust-Projekt im Root angelegt (ohne Nutzung von AlteVersion)
* [x] Cargo-Konfiguration mit Binaries fuer mowes-next und mowes-builder
* [x] Builder-Module fuer ZIP-Erkennung und Paketaufbau erstellt
* [x] Automatische Component-Erkennung fuer Apache- und MariaDB-ZIP in Components
* [x] Paketstruktur unter dist mit runtime, config, projects, data, logs, temp
* [x] Sicheres ZIP-Entpacken mit Pfadschutz (enclosed_name)
* [x] Generierung von default.config.json, Start.bat, StartHeadless.bat
* [x] Builder-Manifest wird erzeugt
* [x] Kompilierbar: cargo check erfolgreich
* [x] Runtime-Validierung auf erforderliche Binaries in ZIP und Extrakt
* [x] Automatische Port-Ermittlung fuer HTTP und MariaDB
* [x] Generierung produktiver Runtime-Templates (Apache, MariaDB, PHP)
* [x] Strukturierte Builder-Logs in logs/builder.jsonl
* [x] CLI-Kommandos: build, status, doctor, headless
* [x] GUI-Diagnose via Doctor-Report integriert
* [x] GUI fuer MoWeS-Next Verwaltung (Build/Start/Stop/Restart/Service)
* [x] Service-Modus vorbereitet (service-mode.json + Skripte)
* [x] Plugin-Discovery fuer plugins/*.json
* [x] Lokaler Update-Check gegen updates/update.manifest.json
* [x] Third-Party-Notices im Build-Output
* [x] Installierbare Build-Variante (dist/mowes-next-installable)
* [x] Update-Manifest schreiben und Update-Bundle anwenden

### Noch zu machen (naechste Umsetzungsschritte)

* [x] Server-Orchestrator implementieren (Start/Stop/Restart Apache und MariaDB)
* [x] Prozess- und Health-Management fuer beide Dienste (laufende Prozesse)
* [x] CLI-Befehle erweitern um start, stop, restart mit Prozesssteuerung

## Versionsschema

Format:

```text
JJ.MM.PATCH
```

Beispiel:

```text
26.05.0
```

---

Lizens die gleiche wie Apache und MariaDB, damit die Komponenten rechtlich abgesichert sind.
