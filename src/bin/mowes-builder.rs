use std::env;
use std::path::PathBuf;

use eframe::egui;
use mowes_next::builder::package::{build_from_components, detect_components};
use rfd::FileDialog;
use mowes_next::orchestrator::doctor::run_doctor;

struct BuilderApp {
    project_root: PathBuf,
    apache_zip: Option<PathBuf>,
    mariadb_zip: Option<PathBuf>,
    status_message: String,
    doctor_lines: Vec<String>,
}

impl BuilderApp {
    fn new(project_root: PathBuf) -> Self {
        let mut app = Self {
            project_root,
            apache_zip: None,
            mariadb_zip: None,
            status_message: "Ready".to_string(),
            doctor_lines: Vec::new(),
        };
        app.refresh_components();
        app
    }

    fn refresh_components(&mut self) {
        match detect_components(&self.project_root) {
            Ok((apache, mariadb)) => {
                self.apache_zip = Some(apache);
                self.mariadb_zip = Some(mariadb);
                self.status_message = "Components found".to_string();
            }
            Err(err) => {
                self.apache_zip = None;
                self.mariadb_zip = None;
                self.status_message = format!("Component scan failed: {err}");
            }
        }
    }

    fn build_package(&mut self) {
        // Wenn ZIPs manuell gewählt, verwende diese, sonst Standard-Scan
        let apache_zip = self.apache_zip.clone();
        let mariadb_zip = self.mariadb_zip.clone();
        let result = if let (Some(apache), Some(mariadb)) = (apache_zip, mariadb_zip) {
            // Temporär: Symlink/Copy in Components, damit build_from_components sie findet
            // (Optional: build_from_components anpassen für direkte Pfadangabe)
            std::fs::copy(&apache, self.project_root.join("Components/apache.zip")).ok();
            std::fs::copy(&mariadb, self.project_root.join("Components/mariadb.zip")).ok();
            build_from_components(&self.project_root)
        } else {
            build_from_components(&self.project_root)
        };
        match result {
            Ok(summary) => {
                self.status_message = format!(
                    "Package created: {} (http {}, db {})",
                    summary.output_dir.display(),
                    summary.http_port,
                    summary.db_port
                );
                self.run_doctor();
            }
            Err(err) => {
                self.status_message = format!("Build failed: {err}");
            }
        }
    }

    fn run_doctor(&mut self) {
        let report = run_doctor(&self.project_root);
        self.doctor_lines = report
            .checks
            .into_iter()
            .map(|c| {
                let marker = if c.ok { "OK" } else { "FAIL" };
                format!("[{marker}] {}: {}", c.name, c.details)
            })
            .collect();
    }
}

impl eframe::App for BuilderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MoWeS Builder");
            ui.separator();

            ui.label(format!("Project root: {}", self.project_root.display()));


            ui.horizontal(|ui| {
                if ui.button("Apache ZIP auswählen").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                        self.apache_zip = Some(path);
                    }
                }
                if let Some(path) = &self.apache_zip {
                    ui.label(format!("Apache ZIP: {}", path.display()));
                } else {
                    ui.label("Apache ZIP: not selected");
                }
            });

            ui.horizontal(|ui| {
                if ui.button("MariaDB ZIP auswählen").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("ZIP", &["zip"]).pick_file() {
                        self.mariadb_zip = Some(path);
                    }
                }
                if let Some(path) = &self.mariadb_zip {
                    ui.label(format!("MariaDB ZIP: {}", path.display()));
                } else {
                    ui.label("MariaDB ZIP: not selected");
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Rescan Components").clicked() {
                    self.refresh_components();
                }

                if ui.button("Create package").clicked() {
                    self.build_package();
                }

                if ui.button("Run doctor").clicked() {
                    self.run_doctor();
                }
            });

            ui.separator();
            ui.label(&self.status_message);

            if !self.doctor_lines.is_empty() {
                ui.add_space(8.0);
                ui.label("Diagnostics:");
                for line in &self.doctor_lines {
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
    let app = BuilderApp::new(project_root);

    if let Err(err) = eframe::run_native(
        "MoWeS Builder",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    ) {
        eprintln!("failed to run builder ui: {err}");
        std::process::exit(1);
    }
}
