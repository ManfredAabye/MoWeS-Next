use std::env;
use std::path::{Path, PathBuf};

use mowes_next::builder::package::{build_from_components, build_from_preset, build_installable_from_components, build_installable_from_preset};
use mowes_next::orchestrator::doctor::run_doctor;
use mowes_next::orchestrator::health::check_health;
use mowes_next::orchestrator::manager::{start_process, status_process, stop_process};
use mowes_next::orchestrator::service::{prepare_service_mode, service_status};
use mowes_next::plugins::discover_plugins;
use mowes_next::update::{apply_update_bundle, check_local_update, UpdateManifest, write_update_manifest};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let project_root = match env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("failed to resolve project root: {err}");
            std::process::exit(1);
        }
    };

    match args.first().map(String::as_str).unwrap_or("help") {
        "--headless" | "headless" => run_headless(&project_root),
        "build" | "build-package" => run_build(&project_root),
        "build-installable" => run_build_installable(&project_root),
        "build-preset" => run_build_preset(&project_root, args.get(1)),
        "build-installable-preset" => run_build_installable_preset(&project_root, args.get(1)),
        "start" => run_start(&project_root),
        "stop" => run_stop(&project_root),
        "restart" => run_restart(&project_root),
        "status" => run_status(&project_root),
        "doctor" => run_doctor_cmd(&project_root),
        "service-prepare" => run_service_prepare(&project_root),
        "service-status" => run_service_status(&project_root),
        "plugins" => run_plugins(&project_root),
        "update-check" => run_update_check(&project_root),
        "update-prepare" => run_update_prepare(&project_root, args.get(1)),
        "update-apply" => run_update_apply(&project_root, args.get(1)),
        _ => print_help(),
    }
}

fn runtime_root(project_root: &Path) -> PathBuf {
    project_root.join("dist").join("mowes-next-package").join("runtime")
}

fn pid_dir(project_root: &Path) -> PathBuf {
    project_root.join("temp").join("pids")
}

fn run_headless(project_root: &Path) {
    run_status(project_root);
    run_doctor_cmd(project_root);
}

fn run_build(project_root: &Path) {
    match build_from_components(project_root) {
        Ok(summary) => {
            println!("Build successful");
            println!("Output: {}", summary.output_dir.display());
            println!("HTTP port: {}", summary.http_port);
            println!("DB port: {}", summary.db_port);
        }
        Err(err) => {
            eprintln!("Build failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_build_installable(project_root: &Path) {
    match build_installable_from_components(project_root) {
        Ok(summary) => {
            println!("Installable build successful");
            println!("Base output: {}", summary.base_package.output_dir.display());
            println!("Installable dir: {}", summary.installable_dir.display());
        }
        Err(err) => {
            eprintln!("Installable build failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_build_preset(project_root: &Path, preset_arg: Option<&String>) {
    let Some(preset_arg) = preset_arg else {
        eprintln!("Fehler: Pfad zur Preset-JSON fehlt!");
        std::process::exit(2);
    };

    match build_from_preset(project_root, Path::new(preset_arg)) {
        Ok(summary) => {
            println!("Build from preset erfolgreich");
            println!("Output: {}", summary.output_dir.display());
            println!("HTTP port: {}", summary.http_port);
            println!("DB port: {}", summary.db_port);
        }
        Err(err) => {
            eprintln!("Build from preset fehlgeschlagen: {err}");
            std::process::exit(2);
        }
    }
}

fn run_build_installable_preset(project_root: &Path, preset_arg: Option<&String>) {
    let Some(preset_arg) = preset_arg else {
        eprintln!("Fehler: Pfad zur Preset-JSON fehlt!");
        std::process::exit(2);
    };

    match build_installable_from_preset(project_root, Path::new(preset_arg)) {
        Ok(summary) => {
            println!("Installable build from preset successful");
            println!("Base output: {}", summary.base_package.output_dir.display());
            println!("Installable dir: {}", summary.installable_dir.display());
        }
        Err(err) => {
            eprintln!("Installable build from preset failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_start(project_root: &Path) {
    let runtime = runtime_root(project_root);
    let pids = pid_dir(project_root);

    match start_process(
        "apache",
        &runtime.join("apache"),
        &pids,
        &["bin/httpd.exe", "httpd.exe"],
        &[],
    ) {
        Ok(status) => println!("Apache: {}", status.details),
        Err(err) => {
            eprintln!("Apache konnte nicht gestartet werden: {err}");
            std::process::exit(2);
        }
    }

    match start_process(
        "mariadb",
        &runtime.join("mariadb"),
        &pids,
        &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"],
        &[],
    ) {
        Ok(status) => println!("MariaDB: {}", status.details),
        Err(err) => {
            eprintln!("MariaDB konnte nicht gestartet werden: {err}");
            std::process::exit(2);
        }
    }

    run_status(project_root);
}

fn run_stop(project_root: &Path) {
    let pids = pid_dir(project_root);

    match stop_process("mariadb", &pids) {
        Ok(status) => println!("MariaDB: {}", status.details),
        Err(err) => eprintln!("MariaDB stop failed: {err}"),
    }
    match stop_process("apache", &pids) {
        Ok(status) => println!("Apache: {}", status.details),
        Err(err) => eprintln!("Apache stop failed: {err}"),
    }
}

fn run_restart(project_root: &Path) {
    run_stop(project_root);
    run_start(project_root);
}

fn run_status(project_root: &Path) {
    let pids = pid_dir(project_root);
    let dist_package = project_root.join("dist").join("mowes-next-package");
    let logs = project_root.join("logs").join("builder.jsonl");

    println!("MoWeS-Next status");
    println!("Project root: {}", project_root.display());
    println!("Package: {}", if dist_package.exists() { "present" } else { "missing" });
    println!("Builder log: {}", if logs.exists() { "present" } else { "missing" });

    match check_health(&pids) {
        Ok(health) => {
            println!("Apache running: {} ({})", health.apache.running, health.apache.details);
            println!("MariaDB running: {} ({})", health.mariadb.running, health.mariadb.details);
            println!("Ready: {}", health.is_ready());
        }
        Err(err) => println!("Health check unavailable: {err}"),
    }

    if let Ok(status) = status_process("apache", &pids) {
        println!("Apache PID: {:?}", status.pid);
    }
    if let Ok(status) = status_process("mariadb", &pids) {
        println!("MariaDB PID: {:?}", status.pid);
    }
}

fn run_doctor_cmd(project_root: &Path) {
    let report = run_doctor(project_root);

    println!("Doctor report");
    for check in &report.checks {
        let marker = if check.ok { "OK" } else { "FAIL" };
        println!("[{marker}] {}: {}", check.name, check.details);
    }

    if !report.is_ok() {
        std::process::exit(3);
    }
}

fn run_service_prepare(project_root: &Path) {
    match prepare_service_mode(project_root) {
        Ok(status) => {
            println!("Service mode prepared: {}", status.config_path.display());
            println!("Details: {}", status.details);
        }
        Err(err) => {
            eprintln!("Service prep failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_service_status(project_root: &Path) {
    match service_status(project_root) {
        Ok(status) => {
            println!("Service configured: {}", status.configured);
            println!("Config: {}", status.config_path.display());
            println!("Details: {}", status.details);
        }
        Err(err) => {
            eprintln!("Service status failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_plugins(project_root: &Path) {
    match discover_plugins(project_root) {
        Ok(inventory) => {
            println!("Plugin directory: {}", inventory.plugin_dir.display());
            println!("Plugins found: {}", inventory.plugins.len());
            for plugin in inventory.plugins {
                println!("- {} {} [{}] enabled={}", plugin.name, plugin.version, plugin.id, plugin.enabled);
            }
        }
        Err(err) => {
            eprintln!("Plugin scan failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_update_check(project_root: &Path) {
    match check_local_update(project_root, "26.05.0") {
        Ok(check) => {
            println!("Current version: {}", check.current_version);
            println!("Manifest: {}", check.manifest_path.display());
            println!("Update available: {}", check.update_available);
            println!("Details: {}", check.details);
        }
        Err(err) => {
            eprintln!("Update check failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_update_prepare(project_root: &Path, version_arg: Option<&String>) {
    let version = version_arg.cloned().unwrap_or_else(|| "26.05.0".to_string());
    let manifest = UpdateManifest {
        version,
        download_url: None,
        notes: Some("Prepared locally from current workspace".to_string()),
    };

    match write_update_manifest(project_root, &manifest) {
        Ok(path) => {
            println!("Update manifest written: {}", path.display());
        }
        Err(err) => {
            eprintln!("Update prepare failed: {err}");
            std::process::exit(2);
        }
    }
}

fn run_update_apply(project_root: &Path, bundle_arg: Option<&String>) {
    let Some(bundle_arg) = bundle_arg else {
        eprintln!("Fehler: Pfad zum Update-Bundle fehlt!");
        std::process::exit(2);
    };

    match apply_update_bundle(project_root, Path::new(bundle_arg)) {
        Ok(report) => {
            println!("Update applied from: {}", report.manifest_path.display());
            println!("Applied to: {}", report.applied_to.display());
            println!("Files copied: {}", report.files_copied);
        }
        Err(err) => {
            eprintln!("Update apply failed: {err}");
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!("MoWeS-Next CLI");
    println!("Usage:");
    println!("  mowes-next build                Build package from Components ZIPs");
    println!("  mowes-next build-installable    Build installable package variant");
    println!("  mowes-next build-preset <file>  Build package aus Preset-JSON");
    println!("  mowes-next build-installable-preset <file>  Build installable package aus Preset-JSON");
    println!("  mowes-next start                Startet Apache und MariaDB");
    println!("  mowes-next stop                 Stoppt laufende Prozesse");
    println!("  mowes-next restart              Restartet beide Prozesse");
    println!("  mowes-next doctor               Run diagnostics for production-readiness");
    println!("  mowes-next status               Show quick project/package status");
    println!("  mowes-next service-prepare      Prepare service mode files");
    println!("  mowes-next service-status       Show service mode status");
    println!("  mowes-next plugins              List discovered plugins");
    println!("  mowes-next update-check         Check local update manifest");
    println!("  mowes-next update-prepare [v]   Write local update manifest");
    println!("  mowes-next update-apply <dir>   Apply update bundle from directory");
    println!("  mowes-next --headless           Headless diagnostics mode");
}
