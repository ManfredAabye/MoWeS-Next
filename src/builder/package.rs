use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use zip::ZipArchive;

use crate::core::logging::JsonLogger;
use crate::core::ports::find_free_port;

const DEFAULT_PACKAGE_NAME: &str = "mowes-next-package";

#[derive(Debug, Clone)]
pub struct PackageSummary {
    pub output_dir: PathBuf,
    pub apache_zip: PathBuf,
    pub mariadb_zip: PathBuf,
    pub http_port: u16,
    pub db_port: u16,
}

#[derive(Debug, Clone)]
pub struct InstallableSummary {
    pub base_package: PackageSummary,
    pub installable_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum BuilderError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("zip error: {0}")]
    Zip(String),
    #[error("components directory missing: {0}")]
    ComponentsDirMissing(String),
    #[error("apache zip not found in Components")]
    ApacheZipMissing,
    #[error("mariadb zip not found in Components")]
    MariaDbZipMissing,
    #[error("component archive missing required files for {component}: expected one of {required:?}")]
    MissingRequiredFiles {
        component: &'static str,
        required: Vec<&'static str>,
    },
}

#[derive(Debug, Serialize)]
struct BuilderManifest {
    apache_zip: String,
    mariadb_zip: String,
    package_name: String,
    http_port: u16,
    db_port: u16,
}

pub fn build_from_components(project_root: &Path) -> Result<PackageSummary, BuilderError> {
    let components_dir = project_root.join("Components");
    if !components_dir.exists() {
        return Err(BuilderError::ComponentsDirMissing(
            components_dir.display().to_string(),
        ));
    }

    let apache_zip = find_component_zip(&components_dir, "apache")
        .ok_or(BuilderError::ApacheZipMissing)?;
    let mariadb_zip = find_component_zip(&components_dir, "mariadb")
        .ok_or(BuilderError::MariaDbZipMissing)?;

    validate_zip_contains_any(
        &apache_zip,
        "Apache",
        &["bin/httpd.exe", "httpd.exe"],
    )?;
    validate_zip_contains_any(
        &mariadb_zip,
        "MariaDB",
        &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"],
    )?;

    let logger = JsonLogger::new(project_root.join("logs").join("builder.jsonl"))?;
    let _ = logger.info(
        "builder_start",
        "building package from Components",
        json!({
            "apache_zip": apache_zip.display().to_string(),
            "mariadb_zip": mariadb_zip.display().to_string()
        }),
    );

    let http_port = find_free_port(8080)?;
    let db_port = find_free_port(3306)?;

    let output_dir = project_root.join("dist").join(DEFAULT_PACKAGE_NAME);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    for dir in ["config", "projects", "data", "logs", "temp"] {
        fs::create_dir_all(output_dir.join(dir))?;
    }

    let runtime_dir = output_dir.join("runtime");
    let apache_target = runtime_dir.join("apache");
    let mariadb_target = runtime_dir.join("mariadb");

    extract_zip_into(&apache_zip, &apache_target)?;
    extract_zip_into(&mariadb_zip, &mariadb_target)?;

    validate_runtime_binaries(
        &apache_target,
        "Apache",
        &["bin/httpd.exe", "httpd.exe"],
    )?;
    validate_runtime_binaries(
        &mariadb_target,
        "MariaDB",
        &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"],
    )?;

    write_default_config(&output_dir, http_port, db_port)?;
    write_runtime_templates(&output_dir, http_port, db_port)?;
    write_launchers(&output_dir)?;
    write_third_party_notices(&output_dir)?;
    write_manifest(&output_dir, &apache_zip, &mariadb_zip, http_port, db_port)?;

    let _ = logger.info(
        "builder_success",
        "package built successfully",
        json!({
            "output_dir": output_dir.display().to_string(),
            "http_port": http_port,
            "db_port": db_port
        }),
    );

    Ok(PackageSummary {
        output_dir,
        apache_zip,
        mariadb_zip,
        http_port,
        db_port,
    })
}

// --- Preset-Unterstützung ---
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BuildPreset {
    pub package_name: Option<String>,
    pub apache_zip: Option<String>,
    pub mariadb_zip: Option<String>,
    pub http_port: Option<u16>,
    pub db_port: Option<u16>,
    pub php_version: Option<String>,
    pub php_extensions: Option<Vec<String>>,
    pub root_password: Option<String>,
    pub portable: Option<bool>,
    pub service: Option<bool>,
}

pub fn build_from_preset(project_root: &Path, preset_path: &Path) -> Result<PackageSummary, BuilderError> {
    let preset_file = std::fs::File::open(preset_path)?;
    let preset: BuildPreset = serde_json::from_reader(preset_file)
        .map_err(|e| BuilderError::Zip(format!("invalid preset: {e}")))?;

    let apache_zip = preset.apache_zip
        .map(|p| project_root.join(p))
        .or_else(|| find_component_zip(&project_root.join("Components"), "apache"))
        .ok_or(BuilderError::ApacheZipMissing)?;
    let mariadb_zip = preset.mariadb_zip
        .map(|p| project_root.join(p))
        .or_else(|| find_component_zip(&project_root.join("Components"), "mariadb"))
        .ok_or(BuilderError::MariaDbZipMissing)?;

    validate_zip_contains_any(&apache_zip, "Apache", &["bin/httpd.exe", "httpd.exe"])?;
    validate_zip_contains_any(&mariadb_zip, "MariaDB", &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;

    let logger = JsonLogger::new(project_root.join("logs").join("builder.jsonl"))?;
    let _ = logger.info(
        "builder_start",
        "building package from preset",
        json!({
            "preset": preset_path.display().to_string(),
            "apache_zip": apache_zip.display().to_string(),
            "mariadb_zip": mariadb_zip.display().to_string()
        }),
    );

    let http_port = preset.http_port.unwrap_or(find_free_port(8080)?);
    let db_port = preset.db_port.unwrap_or(find_free_port(3306)?);
    let package_name = preset.package_name.unwrap_or_else(|| DEFAULT_PACKAGE_NAME.to_string());

    let output_dir = project_root.join("dist").join(&package_name);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    for dir in ["config", "projects", "data", "logs", "temp"] {
        fs::create_dir_all(output_dir.join(dir))?;
    }
    let runtime_dir = output_dir.join("runtime");
    let apache_target = runtime_dir.join("apache");
    let mariadb_target = runtime_dir.join("mariadb");
    extract_zip_into(&apache_zip, &apache_target)?;
    extract_zip_into(&mariadb_zip, &mariadb_target)?;
    validate_runtime_binaries(&apache_target, "Apache", &["bin/httpd.exe", "httpd.exe"])?;
    validate_runtime_binaries(&mariadb_target, "MariaDB", &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;

    write_default_config(&output_dir, http_port, db_port)?;
    write_runtime_templates(&output_dir, http_port, db_port)?;
    write_launchers(&output_dir)?;
    write_third_party_notices(&output_dir)?;
    write_manifest(&output_dir, &apache_zip, &mariadb_zip, http_port, db_port)?;

    let _ = logger.info(
        "builder_success",
        "package built from preset successfully",
        json!({
            "output_dir": output_dir.display().to_string(),
            "http_port": http_port,
            "db_port": db_port
        }),
    );

    Ok(PackageSummary {
        output_dir,
        apache_zip,
        mariadb_zip,
        http_port,
        db_port,
    })
}

pub fn build_installable_from_components(project_root: &Path) -> Result<InstallableSummary, BuilderError> {
    let base_package = build_from_components(project_root)?;
    let installable_dir = project_root.join("dist").join("mowes-next-installable");
    prepare_installable_variant(&base_package.output_dir, &installable_dir)?;
    Ok(InstallableSummary {
        base_package,
        installable_dir,
    })
}

pub fn build_installable_from_preset(project_root: &Path, preset_path: &Path) -> Result<InstallableSummary, BuilderError> {
    let base_package = build_from_preset(project_root, preset_path)?;
    let installable_dir = project_root.join("dist").join("mowes-next-installable");
    prepare_installable_variant(&base_package.output_dir, &installable_dir)?;
    Ok(InstallableSummary {
        base_package,
        installable_dir,
    })
}

pub fn detect_components(project_root: &Path) -> Result<(PathBuf, PathBuf), BuilderError> {
    let components_dir = project_root.join("Components");
    if !components_dir.exists() {
        return Err(BuilderError::ComponentsDirMissing(
            components_dir.display().to_string(),
        ));
    }

    let apache_zip = find_component_zip(&components_dir, "apache")
        .ok_or(BuilderError::ApacheZipMissing)?;
    let mariadb_zip = find_component_zip(&components_dir, "mariadb")
        .ok_or(BuilderError::MariaDbZipMissing)?;

    Ok((apache_zip, mariadb_zip))
}

fn find_component_zip(components_dir: &Path, needle: &str) -> Option<PathBuf> {
    let mut matches = Vec::new();

    let entries = fs::read_dir(components_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| v.eq_ignore_ascii_case("zip"))
            != Some(true)
        {
            continue;
        }

        let lower_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if lower_name.contains(needle) {
            let modified = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            matches.push((modified, path));
        }
    }

    matches.sort_by_key(|(modified, _)| *modified);
    matches.pop().map(|(_, p)| p)
}

fn validate_zip_contains_any(
    zip_path: &Path,
    component: &'static str,
    required_candidates: &[&'static str],
) -> Result<(), BuilderError> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file).map_err(|err| BuilderError::Zip(err.to_string()))?;
    let names = collect_zip_file_names(&mut archive)?;

    let has_required = required_candidates.iter().any(|candidate| {
        names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&candidate.replace('\\', "/")))
    });

    if has_required {
        return Ok(());
    }

    Err(BuilderError::MissingRequiredFiles {
        component,
        required: required_candidates.to_vec(),
    })
}

fn collect_zip_file_names(archive: &mut ZipArchive<fs::File>) -> Result<Vec<String>, BuilderError> {
    let mut names = Vec::new();

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|err| BuilderError::Zip(err.to_string()))?;

        if let Some(name) = entry.enclosed_name() {
            names.push(name.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(names)
}

fn extract_zip_into(zip_path: &Path, target_dir: &Path) -> Result<(), BuilderError> {
    fs::create_dir_all(target_dir)?;

    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file).map_err(|err| BuilderError::Zip(err.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| BuilderError::Zip(err.to_string()))?;

        let Some(safe_name) = entry.enclosed_name().map(|p| p.to_owned()) else {
            continue;
        };

        let out_path = target_dir.join(safe_name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut output)?;
    }

    Ok(())
}

fn validate_runtime_binaries(
    extracted_root: &Path,
    component: &'static str,
    required_candidates: &[&'static str],
) -> Result<(), BuilderError> {
    if required_candidates
        .iter()
        .any(|rel| extracted_root.join(rel).exists())
    {
        return Ok(());
    }

    Err(BuilderError::MissingRequiredFiles {
        component,
        required: required_candidates.to_vec(),
    })
}

fn write_default_config(output_dir: &Path, http_port: u16, db_port: u16) -> Result<(), BuilderError> {
    let cfg = json!({
        "server": {
            "host": "127.0.0.1",
            "port": http_port,
            "auto_port": true
        },
        "mariadb": {
            "port": db_port,
            "auto_port": true,
            "root_password": "root"
        },
        "paths": {
            "projects": "./projects",
            "logs": "./logs",
            "data": "./data",
            "temp": "./temp"
        },
        "mode": {
            "portable": true,
            "service": false
        }
    });

    fs::write(
        output_dir.join("config").join("default.config.json"),
        serde_json::to_string_pretty(&cfg).map_err(|err| BuilderError::Zip(err.to_string()))?,
    )?;

    Ok(())
}

fn write_runtime_templates(output_dir: &Path, http_port: u16, db_port: u16) -> Result<(), BuilderError> {
    let generated_dir = output_dir.join("runtime").join("generated");
    fs::create_dir_all(&generated_dir)?;

    let apache_conf = format!(
        "ServerRoot \"./runtime/apache\"\nListen {}\nServerName 127.0.0.1\nPidFile ./temp/httpd.pid\nErrorLog ./logs/apache-error.log\n",
        http_port
    );

    let mariadb_ini = format!(
        "[mysqld]\nport={}\ndatadir=./data/mariadb\nlog-error=./logs/mariadb-error.log\n",
        db_port
    );

    let php_ini = "[PHP]\nengine=On\nshort_open_tag=Off\nmax_execution_time=60\ndate.timezone=UTC\n";

    fs::write(generated_dir.join("httpd.generated.conf"), apache_conf)?;
    fs::write(generated_dir.join("my.generated.ini"), mariadb_ini)?;
    fs::write(generated_dir.join("php.generated.ini"), php_ini)?;
    Ok(())
}

fn write_launchers(output_dir: &Path) -> Result<(), BuilderError> {
    let start_bat = "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nstart \"\" \"%~dp0mowes-next.exe\"\r\n";
    let headless_bat = "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\n\"%~dp0mowes-next.exe\" --headless\r\n";

    fs::write(output_dir.join("Start.bat"), start_bat)?;
    fs::write(output_dir.join("StartHeadless.bat"), headless_bat)?;

    Ok(())
}

fn write_third_party_notices(output_dir: &Path) -> Result<(), BuilderError> {
    let notices = r#"Third-Party Notices

This package may contain or reference third-party components such as Apache HTTP Server,
MariaDB, PHP, and their dependencies.

Each component remains subject to its own license terms and redistribution conditions.
Review the upstream license files and notices before shipping a derived package.

Apache HTTP Server: Apache License 2.0
MariaDB: GNU GPL / server-side component license terms as applicable
PHP: PHP License / bundled extension notices as applicable
"#;

    fs::write(output_dir.join("THIRD_PARTY_NOTICES.txt"), notices)?;
    Ok(())
}

fn write_manifest(
    output_dir: &Path,
    apache_zip: &Path,
    mariadb_zip: &Path,
    http_port: u16,
    db_port: u16,
) -> Result<(), BuilderError> {
    let manifest = BuilderManifest {
        apache_zip: apache_zip.display().to_string(),
        mariadb_zip: mariadb_zip.display().to_string(),
        package_name: DEFAULT_PACKAGE_NAME.to_string(),
        http_port,
        db_port,
    };

    fs::write(
        output_dir.join("builder.manifest.json"),
        serde_json::to_string_pretty(&manifest)
            .map_err(|err| BuilderError::Zip(err.to_string()))?,
    )?;

    Ok(())
}

fn prepare_installable_variant(source_package_dir: &Path, installable_dir: &Path) -> Result<(), BuilderError> {
    if installable_dir.exists() {
        fs::remove_dir_all(installable_dir)?;
    }
    copy_dir_recursive(source_package_dir, installable_dir)?;

    let install_dir = installable_dir.join("install");
    fs::create_dir_all(&install_dir)?;

    fs::write(
        install_dir.join("install.bat"),
        "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\necho Installing MoWeS-Next installable package...\r\ncopy /y ..\\mowes-next.exe ..\\mowes-next-installed.exe >nul\r\necho Done.\r\n",
    )?;
    fs::write(
        install_dir.join("uninstall.bat"),
        "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\necho Uninstall placeholder for MoWeS-Next installable package...\r\n",
    )?;
    fs::write(
        installable_dir.join("INSTALLABLE.txt"),
        "This directory represents the installable distribution variant of MoWeS-Next.\r\nIt includes install/uninstall scaffolding and can be wrapped by a real installer later.\r\n",
    )?;

    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), BuilderError> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = destination.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
