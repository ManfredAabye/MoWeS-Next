use std::fs;
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use mowes_next::builder::package::{build_from_preset, build_installable_from_preset};
use mowes_next::core::paths::resolve_package_dir;
use mowes_next::orchestrator::health::check_health;
use mowes_next::orchestrator::manager::{start_process, status_process, stop_process};
use mowes_next::orchestrator::service::{prepare_service_mode, service_status};
use mowes_next::plugins::discover_plugins;
use mowes_next::update::{apply_update_bundle, check_local_update, UpdateManifest, write_update_manifest};
use rfd::FileDialog;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTab {
    Build,
    Runtime,
    Maintenance,
}

struct ServerUiApp {
    project_root: PathBuf,
    preset_path: Option<PathBuf>,
    http_port_input: String,
    db_port_input: String,
    output_base_dir: PathBuf,
    status_message: String,
    details: Vec<String>,
    selected_tab: UiTab,
}

#[derive(Debug, Clone)]
struct AccessInfo {
    web_url: String,
    index_url: String,
    db_host: String,
    db_port: u16,
    db_user: String,
    db_password: String,
    db_name: String,
}

impl ServerUiApp {
    fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            preset_path: Some(PathBuf::from("presets/default.json")),
            http_port_input: "8080".to_string(),
            db_port_input: "3306".to_string(),
            output_base_dir: PathBuf::from("dist"),
            status_message: "Ready".to_string(),
            details: Vec::new(),
            selected_tab: UiTab::Build,
        }
    }

    fn pid_dir(&self) -> PathBuf {
        self.project_root.join("temp").join("pids")
    }

    fn refresh_overview(&mut self) {
        self.details.clear();

        if let Ok(health) = check_health(&self.pid_dir()) {
            self.details.push(format!("Apache: {} ({})", health.apache.running, health.apache.details));
            self.details.push(format!("MariaDB: {} ({})", health.mariadb.running, health.mariadb.details));
        }

        if let Ok(apache) = status_process("apache", &self.pid_dir()) {
            self.details.push(format!("Apache PID: {:?}", apache.pid));
        }

        if let Ok(mariadb) = status_process("mariadb", &self.pid_dir()) {
            self.details.push(format!("MariaDB PID: {:?}", mariadb.pid));
        }

        if let Ok(service) = service_status(&self.project_root) {
            self.details.push(format!("Service mode: {} ({})", service.configured, service.details));
        }

        if let Ok(plugins) = discover_plugins(&self.project_root) {
            self.details.push(format!("Plugins: {} found in {}", plugins.plugins.len(), plugins.plugin_dir.display()));
        }

        if let Ok(update) = check_local_update(&self.project_root, "26.05.0") {
            self.details.push(format!("Update: {}", update.details));
        }

        if let Some(access) = self.load_access_info() {
            self.details.push(format!("Webseite: {}", access.index_url));
            self.details.push(format!("MariaDB: {}:{} user={} db={}", access.db_host, access.db_port, access.db_user, access.db_name));
        }
    }

    fn active_package_dir(&self) -> PathBuf {
        resolve_package_dir(&self.project_root)
            .unwrap_or_else(|_| self.project_root.join("dist").join("mowes-next-package"))
    }

    fn load_access_info(&self) -> Option<AccessInfo> {
        let package = self.active_package_dir();
        let cfg_path = package.join("config").join("default.config.json");
        let raw = fs::read_to_string(cfg_path).ok()?;
        let cfg: Value = serde_json::from_str(&raw).ok()?;

        let host = cfg
            .get("server")
            .and_then(|v| v.get("host"))
            .and_then(Value::as_str)
            .unwrap_or("127.0.0.1");

        let http_port = cfg
            .get("server")
            .and_then(|v| v.get("port"))
            .and_then(Value::as_u64)
            .and_then(|v| u16::try_from(v).ok())
            .unwrap_or(8080);

        let db_port = cfg
            .get("mariadb")
            .and_then(|v| v.get("port"))
            .and_then(Value::as_u64)
            .and_then(|v| u16::try_from(v).ok())
            .unwrap_or(3306);

        let db_password = cfg
            .get("mariadb")
            .and_then(|v| v.get("root_password"))
            .and_then(Value::as_str)
            .unwrap_or("root")
            .to_string();

        let db_name = cfg
            .get("mariadb")
            .and_then(|v| v.get("database_name"))
            .and_then(Value::as_str)
            .unwrap_or("(nicht gesetzt)")
            .to_string();

        let web_url = format!("http://{host}:{http_port}/");
        let index_url = format!("http://{host}:{http_port}/index.html");

        Some(AccessInfo {
            web_url,
            index_url,
            db_host: host.to_string(),
            db_port,
            db_user: "root".to_string(),
            db_password,
            db_name,
        })
    }

    fn render_access_info(&self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading("Zugriffsdaten");
                if let Some(access) = self.load_access_info() {
                    ui.label(format!("Webseite: {}", access.web_url));
                    ui.label(format!("Index: {}", access.index_url));
                    ui.add_space(4.0);
                    ui.label(format!("MariaDB Host: {}", access.db_host));
                    ui.label(format!("MariaDB Port: {}", access.db_port));
                    ui.label(format!("MariaDB User: {}", access.db_user));
                    ui.label(format!("MariaDB Passwort: {}", access.db_password));
                    ui.label(format!("MariaDB Datenbank: {}", access.db_name));
                } else {
                    ui.label("Keine Zugriffsdaten verfuegbar. Bitte zuerst Build ausfuehren.");
                }
            });
        });
    }

    fn open_index_in_browser(&mut self) {
        let Some(access) = self.load_access_info() else {
            self.status_message = "Keine Zugriffsdaten verfuegbar".to_string();
            return;
        };

        let result = Command::new("cmd")
            .args(["/C", "start", "", &access.index_url])
            .spawn();

        match result {
            Ok(_) => self.status_message = format!("Browser geoeffnet: {}", access.index_url),
            Err(err) => self.status_message = format!("Browser konnte nicht geoeffnet werden: {err}"),
        }
    }

    fn test_mariadb_connection(&mut self) {
        let package = self.active_package_dir();
        let Some(access) = self.load_access_info() else {
            self.status_message = "Keine MariaDB-Zugangsdaten verfuegbar".to_string();
            return;
        };

        let candidates = [
            package.join("runtime").join("mariadb").join("bin").join("mariadb-admin.exe"),
            package.join("runtime").join("mariadb").join("bin").join("mysqladmin.exe"),
        ];

        let Some(admin_exe) = candidates.iter().find(|p| p.exists()) else {
            self.status_message = "mariadb-admin.exe nicht gefunden".to_string();
            return;
        };

        let output = Command::new(admin_exe)
            .args([
                "ping",
                "--connect-timeout=3",
                "-h",
                &access.db_host,
                "-P",
                &access.db_port.to_string(),
                "-u",
                &access.db_user,
                &format!("--password={}", access.db_password),
            ])
            .current_dir(&package)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                self.status_message = "MariaDB Verbindung OK".to_string();
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                self.status_message = format!(
                    "MariaDB Verbindung fehlgeschlagen: {} {}",
                    stdout.trim(),
                    stderr.trim()
                );
            }
            Err(err) => {
                self.status_message = format!("MariaDB-Test konnte nicht ausgefuehrt werden: {err}");
            }
        }
    }

    fn find_first_existing(base: &std::path::Path, candidates: &[&str]) -> Option<PathBuf> {
        for rel in candidates {
            let p = base.join(rel);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    fn detect_binary_version(exe: &std::path::Path, args: &[&str]) -> String {
        let output = Command::new(exe).args(args).output();
        let Ok(output) = output else {
            return "unbekannt".to_string();
        };

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return stdout.lines().next().unwrap_or("unbekannt").to_string();
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            return stderr.lines().next().unwrap_or("unbekannt").to_string();
        }

        "unbekannt".to_string()
    }

    fn write_runtime_versions_file(package: &std::path::Path, apache_version: &str, mariadb_version: &str) -> Result<(), String> {
        let htdocs = package.join("runtime").join("apache").join("htdocs");
        fs::create_dir_all(&htdocs).map_err(|e| format!("htdocs create failed: {e}"))?;

        let payload = json!({
            "apache": apache_version,
            "mariadb": mariadb_version,
        });

        let body = serde_json::to_string_pretty(&payload)
            .map_err(|e| format!("serialize versions failed: {e}"))?;
        fs::write(htdocs.join("versions.json"), body)
            .map_err(|e| format!("write versions.json failed: {e}"))?;

        Ok(())
    }

    fn status_chip(ui: &mut egui::Ui, label: &str, ok: bool, details: &str) {
        let color = if ok {
            egui::Color32::from_rgb(34, 197, 94)
        } else {
            egui::Color32::from_rgb(239, 68, 68)
        };
        let text = egui::RichText::new(format!("{}: {}", label, details)).color(color).strong();
        ui.label(text);
    }

    fn render_overview_cards(&mut self, ui: &mut egui::Ui) {
        let health = check_health(&self.pid_dir()).ok();
        let service = service_status(&self.project_root).ok();
        let plugins = discover_plugins(&self.project_root).ok();
        let update = check_local_update(&self.project_root, "26.05.0").ok();

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading("Status Overview");
                if let Some(health) = health {
                    Self::status_chip(ui, "Apache", health.apache.running, &health.apache.details);
                    Self::status_chip(ui, "MariaDB", health.mariadb.running, &health.mariadb.details);
                } else {
                    Self::status_chip(ui, "Apache", false, "health unavailable");
                    Self::status_chip(ui, "MariaDB", false, "health unavailable");
                }

                if let Some(service) = service {
                    Self::status_chip(ui, "Service", service.configured, &service.details);
                }

                if let Some(plugins) = plugins {
                    let details = format!("{} plugins found", plugins.plugins.len());
                    Self::status_chip(ui, "Plugins", !plugins.plugins.is_empty(), &details);
                }

                if let Some(update) = update {
                    Self::status_chip(ui, "Update", update.update_available, &update.details);
                }
            });
        });
    }

    fn parse_port_input(value: &str, label: &str) -> Result<u16, String> {
        let trimmed = value.trim();
        trimmed
            .parse::<u16>()
            .map_err(|_| format!("Invalid {label} port: {trimmed}"))
    }

    fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    fn port_status(value: &str) -> (String, egui::Color32) {
        match value.trim().parse::<u16>() {
            Ok(port) => {
                if Self::is_port_available(port) {
                    ("frei".to_string(), egui::Color32::from_rgb(34, 197, 94))
                } else {
                    ("belegt".to_string(), egui::Color32::from_rgb(239, 68, 68))
                }
            }
            Err(_) => ("ungueltig".to_string(), egui::Color32::from_rgb(245, 158, 11)),
        }
    }

    fn create_effective_preset(&self, use_selected_preset: bool) -> Result<PathBuf, String> {
        let mut preset = if use_selected_preset {
            let Some(path) = &self.preset_path else {
                return Err("No preset selected".to_string());
            };

            let raw = fs::read_to_string(path).map_err(|e| format!("Cannot read preset: {e}"))?;
            serde_json::from_str::<Value>(&raw).map_err(|e| format!("Invalid preset JSON: {e}"))?
        } else {
            json!({})
        };

        let obj = preset
            .as_object_mut()
            .ok_or_else(|| "Preset root must be a JSON object".to_string())?;

        let http_port = Self::parse_port_input(&self.http_port_input, "HTTP")?;
        let db_port = Self::parse_port_input(&self.db_port_input, "DB")?;

        if http_port == db_port {
            return Err("HTTP und DB Port muessen unterschiedlich sein".to_string());
        }

        if !Self::is_port_available(http_port) {
            return Err(format!("HTTP-Port {http_port} ist bereits belegt"));
        }

        if !Self::is_port_available(db_port) {
            return Err(format!("DB-Port {db_port} ist bereits belegt"));
        }

        obj.insert("http_port".to_string(), json!(http_port));
        obj.insert("db_port".to_string(), json!(db_port));

        let output_base = if self.output_base_dir.is_absolute() {
            self.output_base_dir.clone()
        } else {
            self.project_root.join(&self.output_base_dir)
        };
        obj.insert(
            "output_base_dir".to_string(),
            json!(output_base.display().to_string()),
        );

        let temp_dir = self.project_root.join("temp").join("control-center");
        fs::create_dir_all(&temp_dir).map_err(|e| format!("Cannot create temp dir: {e}"))?;
        let temp_preset_path = temp_dir.join("effective-build-preset.json");

        let payload = serde_json::to_string_pretty(&preset)
            .map_err(|e| format!("Cannot serialize effective preset: {e}"))?;
        fs::write(&temp_preset_path, payload)
            .map_err(|e| format!("Cannot write effective preset: {e}"))?;

        Ok(temp_preset_path)
    }

    fn run_build(&mut self, use_selected_preset: bool, installable: bool) {
        let effective_preset = match self.create_effective_preset(use_selected_preset) {
            Ok(path) => path,
            Err(err) => {
                self.status_message = err;
                return;
            }
        };

        if installable {
            match build_installable_from_preset(&self.project_root, &effective_preset) {
                Ok(summary) => {
                    self.status_message = format!("Installable built: {}", summary.installable_dir.display());
                    self.refresh_overview();
                }
                Err(err) => self.status_message = format!("Installable build failed: {err}"),
            }
        } else {
            match build_from_preset(&self.project_root, &effective_preset) {
                Ok(summary) => {
                    self.status_message = format!("Built package: {}", summary.output_dir.display());
                    self.refresh_overview();
                }
                Err(err) => self.status_message = format!("Build failed: {err}"),
            }
        }
    }

    fn ensure_mariadb_initialized(package: &std::path::Path) -> Result<bool, String> {
        let data_dir = package.join("data").join("mariadb");
        let logs_dir = package.join("logs");
        fs::create_dir_all(&data_dir).map_err(|e| format!("datadir create failed: {e}"))?;
        fs::create_dir_all(&logs_dir).map_err(|e| format!("logs dir create failed: {e}"))?;

        if data_dir.join("mysql").exists() {
            return Ok(false);
        }

        let installer_candidates = [
            package.join("runtime").join("mariadb").join("bin").join("mariadb-install-db.exe"),
            package.join("runtime").join("mariadb").join("bin").join("mysql_install_db.exe"),
        ];

        let installer = installer_candidates
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| "mariadb-install-db.exe not found".to_string())?;

        let output = Command::new(installer)
            .arg("-d")
            .arg(data_dir.display().to_string())
            .arg("-p")
            .arg("root")
            .current_dir(package)
            .output()
            .map_err(|e| format!("installer execution failed: {e}"))?;

        if output.status.success() {
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!("installer failed: {} {}", stdout.trim(), stderr.trim()))
        }
    }

    fn start_server(&mut self) {
        let package = self.active_package_dir();
        let pids = self.pid_dir();
        let mariadb_ini = package.join("runtime").join("generated").join("my.generated.ini");
        let mariadb_data = package.join("data").join("mariadb");
        let mariadb_log = package.join("logs").join("mariadb-error.log");
        let apache_args: Vec<String> = Vec::new();
        let mariadb_args = vec![
            format!("--defaults-file={}", mariadb_ini.display()),
            format!("--datadir={}", mariadb_data.display()),
            format!("--log-error={}", mariadb_log.display()),
        ];

        let apache_exe = Self::find_first_existing(
            &package.join("runtime").join("apache"),
            &["bin/httpd.exe", "httpd.exe"],
        );
        let mariadb_exe = Self::find_first_existing(
            &package,
            &[
                "runtime/mariadb/bin/mariadbd.exe",
                "runtime/mariadb/mariadbd.exe",
                "runtime/mariadb/bin/mysqld.exe",
                "runtime/mariadb/mysqld.exe",
            ],
        );

        let apache_version = apache_exe
            .as_deref()
            .map(|exe| Self::detect_binary_version(exe, &["-v"]))
            .unwrap_or_else(|| "unbekannt".to_string());
        let mariadb_version = mariadb_exe
            .as_deref()
            .map(|exe| Self::detect_binary_version(exe, &["--version"]))
            .unwrap_or_else(|| "unbekannt".to_string());

        let _ = Self::write_runtime_versions_file(&package, &apache_version, &mariadb_version);

        let initialized_now = match Self::ensure_mariadb_initialized(&package) {
            Ok(v) => v,
            Err(err) => {
                self.status_message = format!("MariaDB init failed: {err}");
                return;
            }
        };

        let apache = start_process(
            "apache",
            &package.join("runtime").join("apache"),
            &pids,
            &["bin/httpd.exe", "httpd.exe"],
            &apache_args,
        );
        let mariadb = start_process(
            "mariadb",
            &package,
            &pids,
            &["runtime/mariadb/bin/mariadbd.exe", "runtime/mariadb/mariadbd.exe", "runtime/mariadb/bin/mysqld.exe", "runtime/mariadb/mysqld.exe"],
            &mariadb_args,
        );

        match (apache, mariadb) {
            (Ok(a), Ok(m)) => {
                self.status_message = if initialized_now {
                    format!("Started: {} / {} | MariaDB-Datadir wurde initialisiert", a.details, m.details)
                } else {
                    format!("Started: {} / {}", a.details, m.details)
                }
            }
            (Err(err), _) | (_, Err(err)) => self.status_message = format!("Start failed: {err}"),
        }
        self.refresh_overview();
    }

    fn stop_server(&mut self) {
        let pids = self.pid_dir();
        let mut messages = Vec::new();
        if let Ok(status) = stop_process("mariadb", &pids) {
            messages.push(status.details);
        }
        if let Ok(status) = stop_process("apache", &pids) {
            messages.push(status.details);
        }
        self.status_message = if messages.is_empty() { "Nothing stopped".to_string() } else { messages.join(" | ") };
        self.refresh_overview();
    }

    fn restart_server(&mut self) {
        self.stop_server();
        self.start_server();
    }

    fn prepare_service_mode(&mut self) {
        match prepare_service_mode(&self.project_root) {
            Ok(status) => self.status_message = format!("Service mode prepared: {}", status.config_path.display()),
            Err(err) => self.status_message = format!("Service prep failed: {err}"),
        }
        self.refresh_overview();
    }

    fn prepare_update_manifest(&mut self) {
        let manifest = UpdateManifest {
            version: "26.05.0".to_string(),
            download_url: None,
            notes: Some("Prepared from UI".to_string()),
        };

        match write_update_manifest(&self.project_root, &manifest) {
            Ok(path) => self.status_message = format!("Update manifest written: {}", path.display()),
            Err(err) => self.status_message = format!("Update manifest failed: {err}"),
        }
        self.refresh_overview();
    }

    fn apply_update_from_dir(&mut self) {
        let Some(bundle) = FileDialog::new().pick_folder() else {
            self.status_message = "No update bundle selected".to_string();
            return;
        };

        match apply_update_bundle(&self.project_root, &bundle) {
            Ok(report) => self.status_message = format!("Update applied: {} files", report.files_copied),
            Err(err) => self.status_message = format!("Update apply failed: {err}"),
        }
        self.refresh_overview();
    }

    fn select_tab(&mut self, tab: UiTab) {
        self.selected_tab = tab;
    }

    fn render_build_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Select preset").clicked() {
                if let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    self.preset_path = Some(path);
                }
            }
            if let Some(path) = &self.preset_path {
                ui.label(format!("Preset: {}", path.display()));
            } else {
                ui.label("Preset: none");
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("HTTP Port:");
            ui.text_edit_singleline(&mut self.http_port_input);
            let (http_status, http_color) = Self::port_status(&self.http_port_input);
            ui.colored_label(http_color, format!("{http_status}"));
            ui.label("DB Port:");
            ui.text_edit_singleline(&mut self.db_port_input);
            let (db_status, db_color) = Self::port_status(&self.db_port_input);
            ui.colored_label(db_color, format!("{db_status}"));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Exportverzeichnis wählen").clicked() {
                if let Some(path) = FileDialog::new().pick_folder() {
                    self.output_base_dir = path;
                }
            }
            ui.label(format!("Dist-Export: {}", self.output_base_dir.display()));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Build from Components").clicked() {
                self.run_build(false, false);
            }
            if ui.button("Build from Preset").clicked() {
                self.run_build(true, false);
            }
            if ui.button("Build Installable").clicked() {
                self.run_build(false, true);
            }
            if ui.button("Build Installable from Preset").clicked() {
                self.run_build(true, true);
            }
        });
    }

    fn render_runtime_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.start_server();
            }
            if ui.button("Stop").clicked() {
                self.stop_server();
            }
            if ui.button("Restart").clicked() {
                self.restart_server();
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Refresh Overview").clicked() {
                self.refresh_overview();
            }
            if ui.button("Index im Browser oeffnen").clicked() {
                self.open_index_in_browser();
            }
            if ui.button("MariaDB Verbindung testen").clicked() {
                self.test_mariadb_connection();
            }
        });

        ui.add_space(10.0);
        self.render_access_info(ui);
    }

    fn render_maintenance_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Prepare Service Mode").clicked() {
                self.prepare_service_mode();
            }
            if ui.button("Prepare Update Manifest").clicked() {
                self.prepare_update_manifest();
            }
            if ui.button("Apply Update Bundle").clicked() {
                self.apply_update_from_dir();
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Refresh Overview").clicked() {
                self.refresh_overview();
            }
        });
    }
}

impl eframe::App for ServerUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MoWeS-Next Control Center");
            ui.label(format!("Project root: {}", self.project_root.display()));
            ui.separator();
            self.render_overview_cards(ui);
            ui.separator();

            ui.horizontal(|ui| {
                let build_selected = self.selected_tab == UiTab::Build;
                let runtime_selected = self.selected_tab == UiTab::Runtime;
                let maintenance_selected = self.selected_tab == UiTab::Maintenance;

                if ui.selectable_label(build_selected, "Build").clicked() {
                    self.select_tab(UiTab::Build);
                }
                if ui.selectable_label(runtime_selected, "Betrieb").clicked() {
                    self.select_tab(UiTab::Runtime);
                }
                if ui.selectable_label(maintenance_selected, "Wartung").clicked() {
                    self.select_tab(UiTab::Maintenance);
                }
            });

            ui.separator();

            match self.selected_tab {
                UiTab::Build => self.render_build_tab(ui),
                UiTab::Runtime => self.render_runtime_tab(ui),
                UiTab::Maintenance => self.render_maintenance_tab(ui),
            }

            ui.separator();
            ui.label(&self.status_message);

            if !self.details.is_empty() {
                ui.add_space(8.0);
                ui.label("Overview:");
                for line in &self.details {
                    ui.label(line);
                }
            }
        });
    }
}

fn main() {
    let project_root = match env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to determine project root: {err}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1600.0, 1000.0]),
        ..Default::default()
    };
    let app = ServerUiApp::new(project_root);

    if let Err(err) = eframe::run_native("MoWeS Next Control Center", options, Box::new(|_cc| Ok(Box::new(app)))) {
        eprintln!("failed to run control center ui: {err}");
        std::process::exit(1);
    }
}
