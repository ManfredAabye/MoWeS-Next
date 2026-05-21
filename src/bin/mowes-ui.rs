use std::env;
use std::path::PathBuf;

use eframe::egui;
use mowes_next::builder::package::{build_from_components, build_from_preset, build_installable_from_components, build_installable_from_preset};
use mowes_next::orchestrator::health::check_health;
use mowes_next::orchestrator::manager::{start_process, status_process, stop_process};
use mowes_next::orchestrator::service::{prepare_service_mode, service_status};
use mowes_next::plugins::discover_plugins;
use mowes_next::update::{apply_update_bundle, check_local_update, UpdateManifest, write_update_manifest};
use rfd::FileDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTab {
    Build,
    Runtime,
    Maintenance,
}

struct ServerUiApp {
    project_root: PathBuf,
    preset_path: Option<PathBuf>,
    status_message: String,
    details: Vec<String>,
    selected_tab: UiTab,
}

impl ServerUiApp {
    fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            preset_path: Some(PathBuf::from("presets/default.json")),
            status_message: "Ready".to_string(),
            details: Vec::new(),
            selected_tab: UiTab::Build,
        }
    }

    fn runtime_root(&self) -> PathBuf {
        self.project_root.join("dist").join("mowes-next-package").join("runtime")
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

    fn build_from_components(&mut self) {
        match build_from_components(&self.project_root) {
            Ok(summary) => {
                self.status_message = format!("Built package: {}", summary.output_dir.display());
                self.refresh_overview();
            }
            Err(err) => self.status_message = format!("Build failed: {err}"),
        }
    }

    fn build_from_preset(&mut self) {
        let Some(preset_path) = &self.preset_path else {
            self.status_message = "No preset selected".to_string();
            return;
        };

        match build_from_preset(&self.project_root, preset_path) {
            Ok(summary) => {
                self.status_message = format!("Built from preset: {}", summary.output_dir.display());
                self.refresh_overview();
            }
            Err(err) => self.status_message = format!("Preset build failed: {err}"),
        }
    }

    fn build_installable(&mut self) {
        match build_installable_from_components(&self.project_root) {
            Ok(summary) => {
                self.status_message = format!("Installable built: {}", summary.installable_dir.display());
                self.refresh_overview();
            }
            Err(err) => self.status_message = format!("Installable build failed: {err}"),
        }
    }

    fn build_installable_from_preset(&mut self) {
        let Some(preset_path) = &self.preset_path else {
            self.status_message = "No preset selected".to_string();
            return;
        };

        match build_installable_from_preset(&self.project_root, preset_path) {
            Ok(summary) => {
                self.status_message = format!("Installable from preset built: {}", summary.installable_dir.display());
                self.refresh_overview();
            }
            Err(err) => self.status_message = format!("Installable preset build failed: {err}"),
        }
    }

    fn start_server(&mut self) {
        let runtime = self.runtime_root();
        let pids = self.pid_dir();
        let apache = start_process("apache", &runtime.join("apache"), &pids, &["bin/httpd.exe", "httpd.exe"], &[]);
        let mariadb = start_process("mariadb", &runtime.join("mariadb"), &pids, &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"], &[]);

        match (apache, mariadb) {
            (Ok(a), Ok(m)) => self.status_message = format!("Started: {} / {}", a.details, m.details),
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
            if ui.button("Build from Components").clicked() {
                self.build_from_components();
            }
            if ui.button("Build from Preset").clicked() {
                self.build_from_preset();
            }
            if ui.button("Build Installable").clicked() {
                self.build_installable();
            }
            if ui.button("Build Installable from Preset").clicked() {
                self.build_installable_from_preset();
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
        });
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

    let options = eframe::NativeOptions::default();
    let app = ServerUiApp::new(project_root);

    if let Err(err) = eframe::run_native("MoWeS Next Control Center", options, Box::new(|_cc| Ok(Box::new(app)))) {
        eprintln!("failed to run control center ui: {err}");
        std::process::exit(1);
    }
}
