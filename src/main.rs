use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use mowes_next::builder::package::{build_from_components, build_from_preset, build_installable_from_components, build_installable_from_preset};
use mowes_next::core::paths::resolve_package_dir;
use mowes_next::orchestrator::doctor::run_doctor;
use mowes_next::orchestrator::health::check_health;
use mowes_next::orchestrator::manager::{start_process, status_process, stop_process};
use mowes_next::orchestrator::service::{prepare_service_mode, service_status};
use mowes_next::plugins::discover_plugins;
use mowes_next::update::{apply_update_bundle, check_local_update, UpdateManifest, write_update_manifest};
use serde_json::{json, Value};

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

fn package_root(project_root: &Path) -> PathBuf {
    resolve_package_dir(project_root)
        .unwrap_or_else(|_| project_root.join("dist").join("mowes-next-package"))
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
    let package = package_root(project_root);
    let pids = pid_dir(project_root);
    let mariadb_ini = package.join("runtime").join("generated").join("my.generated.ini");
    let mariadb_data = package.join("Data").join("SQL");
    let mariadb_log = package.join("logs").join("mariadb-error.log");
    let mariadb_pid = package.join("temp").join("mariadb.pid");
    let apache_args: Vec<String> = Vec::new();
    let mariadb_args = vec![
        format!("--defaults-file={}", mariadb_ini.display()),
        format!("--datadir={}", mariadb_data.display()),
        format!("--log-error={}", mariadb_log.display()),
        format!("--pid-file={}", mariadb_pid.display()),
    ];

    let apache_exe = find_first_existing(
        &package.join("runtime").join("apache"),
        &["bin/httpd.exe", "httpd.exe"],
    );
    let mariadb_exe = find_first_existing(
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
        .map(|exe| detect_binary_version(exe, &["-v"]))
        .unwrap_or_else(|| "unbekannt".to_string());
    let mariadb_version = mariadb_exe
        .as_deref()
        .map(|exe| detect_binary_version(exe, &["--version"]))
        .unwrap_or_else(|| "unbekannt".to_string());

    let _ = write_runtime_versions_file(&package, &apache_version, &mariadb_version);

    if let Err(err) = ensure_mariadb_initialized(&package) {
        eprintln!("MariaDB konnte nicht initialisiert werden: {err}");
        std::process::exit(2);
    }

    match start_process(
        "apache",
        &package.join("runtime").join("apache"),
        &pids,
        &["bin/httpd.exe", "httpd.exe"],
        &apache_args,
    ) {
        Ok(status) => println!("Apache: {}", status.details),
        Err(err) => {
            eprintln!("Apache konnte nicht gestartet werden: {err}");
            std::process::exit(2);
        }
    }

    match start_process(
        "mariadb",
        &package,
        &pids,
        &["runtime/mariadb/bin/mariadbd.exe", "runtime/mariadb/mariadbd.exe", "runtime/mariadb/bin/mysqld.exe", "runtime/mariadb/mysqld.exe"],
        &mariadb_args,
    ) {
        Ok(status) => {
            println!("MariaDB: {}", status.details);
            match ensure_configured_databases(&package) {
                Ok(report) => println!("MariaDB Datenbanken: {report}"),
                Err(err) => {
                    eprintln!("MariaDB Datenbanken konnten nicht angelegt werden: {err}");
                    std::process::exit(2);
                }
            }
        }
        Err(err) => {
            eprintln!("MariaDB konnte nicht gestartet werden: {err}");
            std::process::exit(2);
        }
    }

    run_status(project_root);
}

fn find_first_existing(base: &Path, candidates: &[&str]) -> Option<PathBuf> {
    for rel in candidates {
        let p = base.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn detect_binary_version(exe: &Path, args: &[&str]) -> String {
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

fn write_runtime_versions_file(package: &Path, apache_version: &str, mariadb_version: &str) -> Result<(), String> {
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

fn ensure_mariadb_initialized(package: &Path) -> Result<(), String> {
    let data_dir = package.join("Data").join("SQL");
    let logs_dir = package.join("logs");
    fs::create_dir_all(&data_dir).map_err(|e| format!("datadir create failed: {e}"))?;
    fs::create_dir_all(&logs_dir).map_err(|e| format!("logs dir create failed: {e}"))?;

    // If mysql system tables exist, initialization has already happened.
    if data_dir.join("mysql").exists() {
        prune_sql_data_dir(&data_dir).map_err(|e| format!("sql datadir cleanup failed: {e}"))?;
        return Ok(());
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
        prune_sql_data_dir(&data_dir).map_err(|e| format!("sql datadir cleanup failed: {e}"))?;
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("installer failed: {} {}", stdout.trim(), stderr.trim()))
    }
}

fn prune_sql_data_dir(data_dir: &Path) -> io::Result<()> {
    let non_runtime_dirs = ["test"];
    let remove_files_by_name = [
        "my.ini",
        "multi-master.info",
        "ddl_recovery.log",
        "ib_buffer_pool",
        "tc.log",
        "auto.cnf",
    ];
    for name in non_runtime_dirs {
        let path = data_dir.join(name);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
    }

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let file_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if ["bak", "tmp", "old", "err", "pid"].iter().any(|v| *v == ext)
            || remove_files_by_name.iter().any(|v| *v == file_name)
        {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn ensure_configured_databases(package: &Path) -> Result<String, String> {
    let cfg_path = package.join("config").join("default.config.json");
    let raw_cfg = fs::read_to_string(&cfg_path)
        .map_err(|e| format!("config read failed ({}): {e}", cfg_path.display()))?;
    let cfg: Value = serde_json::from_str(&raw_cfg)
        .map_err(|e| format!("config parse failed ({}): {e}", cfg_path.display()))?;

    let mariadb_cfg = cfg.get("mariadb").and_then(Value::as_object);
    let mut db_names: Vec<String> = Vec::new();

    if let Some(name) = mariadb_cfg
        .and_then(|v| v.get("database_name"))
        .and_then(Value::as_str)
    {
        db_names.push(name.to_string());
    }

    if let Some(list) = mariadb_cfg
        .and_then(|v| v.get("database_names"))
        .and_then(Value::as_array)
    {
        for item in list {
            if let Some(name) = item.as_str() {
                db_names.push(name.to_string());
            }
        }
    }

    let mut db_names: Vec<String> = db_names
        .into_iter()
        .filter_map(|name| normalize_db_name(&name))
        .collect();
    db_names.sort();
    db_names.dedup();

    if db_names.is_empty() {
        return Ok("keine konfigurierten Datenbanken".to_string());
    }

    let db_port = mariadb_cfg
        .and_then(|v| v.get("port"))
        .and_then(Value::as_u64)
        .and_then(|v| u16::try_from(v).ok())
        .unwrap_or(3306);
    let db_password = mariadb_cfg
        .and_then(|v| v.get("root_password"))
        .and_then(Value::as_str)
        .unwrap_or("root")
        .to_string();

    let client = find_first_existing(
        &package.join("runtime").join("mariadb").join("bin"),
        &["mariadb.exe", "mysql.exe"],
    )
    .ok_or_else(|| "mariadb.exe/mysql.exe nicht gefunden (fuer DB-Initialisierung erforderlich)".to_string())?;

    let mut last_error = String::new();
    for _ in 0..20 {
        let mut cmd = Command::new(&client);
        cmd.arg("-h")
            .arg("127.0.0.1")
            .arg("-P")
            .arg(db_port.to_string())
            .arg("-u")
            .arg("root")
            .arg("-N");
        if !db_password.is_empty() {
            cmd.arg(format!("-p{}", db_password));
        }
        let output = cmd
            .arg("-e")
            .arg("SELECT 1;")
            .current_dir(package)
            .output();

        match output {
            Ok(out) if out.status.success() => break,
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                last_error = if !stderr.is_empty() { stderr } else { stdout };
            }
            Err(err) => last_error = err.to_string(),
        }

        thread::sleep(Duration::from_millis(350));
    }

    if !last_error.is_empty() {
        let mut ready_check = Command::new(&client);
        ready_check
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-P")
            .arg(db_port.to_string())
            .arg("-u")
            .arg("root")
            .arg("-N");
        if !db_password.is_empty() {
            ready_check.arg(format!("-p{}", db_password));
        }
        let ready_output = ready_check
            .arg("-e")
            .arg("SELECT 1;")
            .current_dir(package)
            .output()
            .map_err(|e| format!("DB-Ready-Check fehlgeschlagen: {e}"))?;
        if !ready_output.status.success() {
            return Err(format!("DB-Initialisierung fehlgeschlagen: {last_error}"));
        }
    }

    let run_sql = |sql: &str| -> Result<std::process::Output, String> {
        let mut cmd = Command::new(&client);
        cmd.arg("-h")
            .arg("127.0.0.1")
            .arg("-P")
            .arg(db_port.to_string())
            .arg("-u")
            .arg("root")
            .arg("-N");
        if !db_password.is_empty() {
            cmd.arg(format!("-p{}", db_password));
        }
        cmd.arg("-e")
            .arg(sql)
            .current_dir(package)
            .output()
            .map_err(|e| format!("SQL-Ausfuehrung fehlgeschlagen: {e}"))
    };

    let mut created: Vec<String> = Vec::new();
    let mut existing: Vec<String> = Vec::new();

    for name in &db_names {
        let exists_sql = format!(
            "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '{}';",
            name
        );
        let exists_output = run_sql(&exists_sql)?;
        if !exists_output.status.success() {
            let stderr = String::from_utf8_lossy(&exists_output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&exists_output.stdout).trim().to_string();
            let reason = if !stderr.is_empty() { stderr } else { stdout };
            return Err(format!("DB-Existenzpruefung fehlgeschlagen fuer {name}: {reason}"));
        }

        let exists = String::from_utf8_lossy(&exists_output.stdout)
            .lines()
            .any(|line| line.trim() == name);
        if exists {
            existing.push(name.clone());
            continue;
        }

        let create_sql = format!(
            "CREATE DATABASE IF NOT EXISTS `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;"
        );
        let create_output = run_sql(&create_sql)?;
        if !create_output.status.success() {
            let stderr = String::from_utf8_lossy(&create_output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&create_output.stdout).trim().to_string();
            let reason = if !stderr.is_empty() { stderr } else { stdout };
            return Err(format!("DB-Anlage fehlgeschlagen fuer {name}: {reason}"));
        }

        created.push(name.clone());
    }

    let report = if !created.is_empty() && !existing.is_empty() {
        format!(
            "erstellt ({}), bereits vorhanden ({})",
            created.join(", "),
            existing.join(", ")
        )
    } else if !created.is_empty() {
        format!("erstellt ({})", created.join(", "))
    } else if !existing.is_empty() {
        format!("bereits vorhanden ({})", existing.join(", "))
    } else {
        "keine konfigurierten Datenbanken".to_string()
    };

    Ok(report)
}

fn normalize_db_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        Some(trimmed.to_string())
    } else {
        None
    }
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
    let dist_package = package_root(project_root);
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
