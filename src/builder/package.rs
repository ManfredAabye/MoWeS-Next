use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::collections::HashMap;

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

    for dir in ["config", "Data/http", "Data/SQL", "logs", "temp"] {
        fs::create_dir_all(output_dir.join(dir))?;
    }

    let runtime_dir = output_dir.join("runtime");
    let apache_target = runtime_dir.join("apache");
    let mariadb_target = runtime_dir.join("mariadb");

    extract_runtime_component(&apache_zip, &apache_target, "apache")?;
    extract_runtime_component(&mariadb_zip, &mariadb_target, "mariadb")?;
    normalize_component_layout(&apache_target, &["bin/httpd.exe", "httpd.exe"])?;
    normalize_component_layout(&mariadb_target, &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;
    compact_component_runtime(&apache_target, "apache")?;
    compact_component_runtime(&mariadb_target, "mariadb")?;
    ensure_apache_runtime_layout(&apache_target)?;

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

    write_default_config(
        &output_dir,
        http_port,
        db_port,
        None,
        None,
        None,
        None,
        None,
        None,
    )?;
    let version = read_version_file(project_root);
    let apache_version = component_version_from_zip(&apache_zip);
    let mariadb_version = component_version_from_zip(&mariadb_zip);
    write_welcome_page(
        &output_dir,
        &runtime_dir,
        None,
        None,
        &version,
        &apache_version,
        &mariadb_version,
    )?;
    write_runtime_templates(&output_dir, http_port, db_port)?;
    patch_apache_httpd_conf(&output_dir, http_port)?;
    write_launchers(&output_dir)?;
    write_control_gui(&output_dir)?;
    write_third_party_notices(&output_dir)?;
    write_manifest(&output_dir, &apache_zip, &mariadb_zip, http_port, db_port)?;
    apply_release_cleanup(&output_dir)?;

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
    pub output_base_dir: Option<String>,
    pub apache_zip: Option<String>,
    pub mariadb_zip: Option<String>,
    pub http_port: Option<u16>,
    pub db_port: Option<u16>,
    pub php_version: Option<String>,
    pub php_extensions: Option<Vec<String>>,
    pub root_password: Option<String>,
    pub portable: Option<bool>,
    pub service: Option<bool>,
    pub database_name: Option<String>,
    pub database_names: Option<Vec<String>>,
    pub database_connections: Option<HashMap<String, String>>,
    pub web_root: Option<String>,
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

    let output_base_dir = resolve_output_base_dir(project_root, preset.output_base_dir.as_deref());
    let output_dir = output_base_dir.join(&package_name);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;
    for dir in ["config", "Data/http", "Data/SQL", "logs", "temp"] {
        fs::create_dir_all(output_dir.join(dir))?;
    }
    let runtime_dir = output_dir.join("runtime");
    let apache_target = runtime_dir.join("apache");
    let mariadb_target = runtime_dir.join("mariadb");
    extract_runtime_component(&apache_zip, &apache_target, "apache")?;
    extract_runtime_component(&mariadb_zip, &mariadb_target, "mariadb")?;
    normalize_component_layout(&apache_target, &["bin/httpd.exe", "httpd.exe"])?;
    normalize_component_layout(&mariadb_target, &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;
    compact_component_runtime(&apache_target, "apache")?;
    compact_component_runtime(&mariadb_target, "mariadb")?;
    ensure_apache_runtime_layout(&apache_target)?;
    validate_runtime_binaries(&apache_target, "Apache", &["bin/httpd.exe", "httpd.exe"])?;
    validate_runtime_binaries(&mariadb_target, "MariaDB", &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;

    let mut configured_database_names: Vec<String> = Vec::new();
    if let Some(name) = preset.database_name.as_ref() {
        configured_database_names.push(name.clone());
    }
    if let Some(names) = preset.database_names.as_ref() {
        configured_database_names.extend(names.iter().cloned());
    }
    if let Some(connections) = preset.database_connections.as_ref() {
        configured_database_names.extend(connections.keys().cloned());
    }
    configured_database_names.sort();
    configured_database_names.dedup();

    write_default_config(
        &output_dir,
        http_port,
        db_port,
        preset.root_password.as_deref(),
        preset.portable,
        preset.service,
        preset.database_name.as_deref(),
        Some(configured_database_names.as_slice()),
        preset.web_root.as_deref(),
    )?;
    let version = read_version_file(project_root);
    let apache_version = component_version_from_zip(&apache_zip);
    let mariadb_version = component_version_from_zip(&mariadb_zip);
    write_welcome_page(
        &output_dir,
        &runtime_dir,
        preset.web_root.as_deref(),
        Some(configured_database_names.as_slice()),
        &version,
        &apache_version,
        &mariadb_version,
    )?;
    write_runtime_templates(&output_dir, http_port, db_port)?;
    patch_apache_httpd_conf(&output_dir, http_port)?;
    write_launchers(&output_dir)?;
    write_control_gui(&output_dir)?;
    write_third_party_notices(&output_dir)?;
    write_manifest(&output_dir, &apache_zip, &mariadb_zip, http_port, db_port)?;
    apply_release_cleanup(&output_dir)?;

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
    let output_base = base_package
        .output_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| project_root.join("dist"));
    let installable_dir = output_base.join("mowes-next-installable");
    prepare_installable_variant(&base_package.output_dir, &installable_dir)?;
    Ok(InstallableSummary {
        base_package,
        installable_dir,
    })
}

fn resolve_output_base_dir(project_root: &Path, configured: Option<&str>) -> PathBuf {
    let Some(raw) = configured else {
        return project_root.join("dist");
    };

    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

fn apply_release_cleanup(output_dir: &Path) -> Result<(), BuilderError> {
    let data_sql = output_dir.join("Data").join("SQL");
    let logs = output_dir.join("logs");
    let temp = output_dir.join("temp");

    clear_directory_contents(&data_sql)?;
    clear_directory_contents(&logs)?;
    clear_directory_contents(&temp)?;

    Ok(())
}

fn clear_directory_contents(path: &Path) -> Result<(), BuilderError> {
    fs::create_dir_all(path)?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(entry_path)?;
        } else {
            fs::remove_file(entry_path)?;
        }
    }

    Ok(())
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
        let normalized = candidate.replace('\\', "/").to_ascii_lowercase();
        names.iter().any(|n| {
            let lower = n.to_ascii_lowercase();
            lower == normalized || lower.ends_with(&format!("/{normalized}"))
        })
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

fn extract_runtime_component(
    zip_path: &Path,
    target_dir: &Path,
    component: &str,
) -> Result<(), BuilderError> {
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

        let Some(relative_path) = map_runtime_component_path(component, &safe_name) else {
            continue;
        };

        let out_path = target_dir.join(relative_path);
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

fn map_runtime_component_path(component: &str, zip_path: &Path) -> Option<PathBuf> {
    let normalized = zip_path.to_string_lossy().replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let anchors: &[&str] = if component.eq_ignore_ascii_case("apache") {
        &["bin", "conf", "modules", "htdocs", "logs"]
    } else if component.eq_ignore_ascii_case("mariadb") {
        &["bin", "lib", "share", "plugin"]
    } else {
        &[]
    };

    let start_idx = parts
        .iter()
        .position(|part| anchors.iter().any(|anchor| part.eq_ignore_ascii_case(anchor)))?;

    Some(PathBuf::from(parts[start_idx..].join("/")))
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

fn normalize_component_layout(
    extracted_root: &Path,
    required_candidates: &[&str],
) -> Result<(), BuilderError> {
    if required_candidates
        .iter()
        .any(|rel| extracted_root.join(rel).exists())
    {
        return Ok(());
    }

    let mut candidate_dir: Option<PathBuf> = None;
    for entry in fs::read_dir(extracted_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if required_candidates.iter().any(|rel| path.join(rel).exists()) {
            candidate_dir = Some(path);
            break;
        }
    }

    let Some(nested_root) = candidate_dir else {
        return Ok(());
    };

    for entry in fs::read_dir(&nested_root)? {
        let entry = entry?;
        let source = entry.path();
        let destination = extracted_root.join(entry.file_name());

        if destination.exists() {
            if destination.is_dir() {
                fs::remove_dir_all(&destination)?;
            } else {
                fs::remove_file(&destination)?;
            }
        }

        fs::rename(source, destination)?;
    }

    fs::remove_dir_all(nested_root)?;
    Ok(())
}

fn compact_component_runtime(component_root: &Path, component: &str) -> Result<(), BuilderError> {
    let common_dirs = [
        "docs",
        "doc",
        "manual",
        "man",
        "examples",
        "example",
        "samples",
        "sample",
        "tests",
        "test",
        "benchmark",
        "benchmarks",
    ];

    for rel in common_dirs {
        remove_path_if_exists(&component_root.join(rel))?;
    }

    if component.eq_ignore_ascii_case("apache") {
        for rel in ["htdocs/manual", "include", "cgi-bin", "manual", "icons", "error"] {
            remove_path_if_exists(&component_root.join(rel))?;
        }
        prune_apache_modules_for_html_php(component_root)?;
    }

    if component.eq_ignore_ascii_case("mariadb") {
        for rel in ["mysql-test", "sql-bench", "support-files", "include", "man", "scripts"] {
            remove_path_if_exists(&component_root.join(rel))?;
        }
        prune_mariadb_bin_for_runtime(component_root)?;
    }

    remove_files_by_extension(component_root, &["pdb", "lib", "exp", "a"])?;
    remove_empty_dirs(component_root)?;
    Ok(())
}

fn remove_path_if_exists(path: &Path) -> Result<(), BuilderError> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn remove_files_by_extension(root: &Path, extensions: &[&str]) -> Result<(), BuilderError> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            remove_files_by_extension(&path, extensions)?;
            continue;
        }

        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| v.to_ascii_lowercase());
        if let Some(ext) = ext {
            if extensions.iter().any(|allowed| *allowed == ext) {
                fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}

fn remove_empty_dirs(root: &Path) -> Result<(), BuilderError> {
    if !root.exists() || !root.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            remove_empty_dirs(&path)?;

            if fs::read_dir(&path)?.next().is_none() {
                fs::remove_dir(&path)?;
            }
        }
    }

    Ok(())
}

fn ensure_apache_runtime_layout(apache_root: &Path) -> Result<(), BuilderError> {
    // Apache default config expects these directories under ServerRoot.
    fs::create_dir_all(apache_root.join("logs"))?;
    Ok(())
}

fn prune_apache_modules_for_html_php(component_root: &Path) -> Result<(), BuilderError> {
    let modules_dir = component_root.join("modules");
    if !modules_dir.exists() {
        return Ok(());
    }

    let keep = [
        "mod_access_compat.so",
        "mod_alias.so",
        "mod_authn_core.so",
        "mod_authz_core.so",
        "mod_authz_host.so",
        "mod_dir.so",
        "mod_env.so",
        "mod_headers.so",
        "mod_log_config.so",
        "mod_mime.so",
        "mod_mpm_winnt.so",
        "mod_rewrite.so",
        "mod_setenvif.so",
    ];

    for entry in fs::read_dir(&modules_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let keep_module = keep.iter().any(|name| file_name == *name)
            || file_name.starts_with("mod_php")
            || file_name.contains("php");
        if !keep_module {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn prune_mariadb_bin_for_runtime(component_root: &Path) -> Result<(), BuilderError> {
    let bin_dir = component_root.join("bin");
    if !bin_dir.exists() {
        return Ok(());
    }

    let keep_exe = [
        "mariadb.exe",
        "mysql.exe",
        "mariadbd.exe",
        "mysqld.exe",
        "mariadb-install-db.exe",
        "mysql_install_db.exe",
        "mariadb-admin.exe",
        "mysqladmin.exe",
        "my_print_defaults.exe",
    ];

    for entry in fs::read_dir(&bin_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if ext == "dll" {
            continue;
        }

        if ext == "exe" {
            let keep = keep_exe.iter().any(|name| file_name == *name);
            if !keep {
                fs::remove_file(path)?;
            }
            continue;
        }

        if ["bat", "cmd", "ps1", "pl", "sh"].iter().any(|v| *v == ext) {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn write_default_config(
    output_dir: &Path,
    http_port: u16,
    db_port: u16,
    root_password: Option<&str>,
    portable: Option<bool>,
    service: Option<bool>,
    database_name: Option<&str>,
    database_names: Option<&[String]>,
    web_root: Option<&str>,
) -> Result<(), BuilderError> {
    let root_password = root_password.unwrap_or("root");
    let portable = portable.unwrap_or(true);
    let service = service.unwrap_or(false);
    let web_root = web_root.unwrap_or("./Data/http");

    let mut mariadb = json!({
        "port": db_port,
        "auto_port": true,
        "root_password": root_password
    });
    if let Some(name) = database_name {
        mariadb["database_name"] = json!(name);
    }
    if let Some(names) = database_names {
        let filtered: Vec<String> = names
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect();
        if !filtered.is_empty() {
            mariadb["database_names"] = json!(filtered);
        }
    }

    let cfg = json!({
        "server": {
            "host": "127.0.0.1",
            "port": http_port,
            "auto_port": true
        },
        "mariadb": mariadb,
        "paths": {
            "projects": web_root,
            "logs": "./logs",
            "data": "./Data",
            "temp": "./temp"
        },
        "mode": {
            "portable": portable,
            "service": service
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
        "[mysqld]\nport={}\ndatadir=./Data/SQL\nlog-error=./logs/mariadb-error.log\nlog-basename=mariadb\n",
        db_port
    );

    let php_ini = "[PHP]\nengine=On\nshort_open_tag=Off\nmax_execution_time=60\ndate.timezone=UTC\n";

    fs::write(generated_dir.join("httpd.generated.conf"), apache_conf)?;
    fs::write(generated_dir.join("my.generated.ini"), mariadb_ini)?;
    fs::write(generated_dir.join("php.generated.ini"), php_ini)?;
    Ok(())
}

fn patch_apache_httpd_conf(output_dir: &Path, http_port: u16) -> Result<(), BuilderError> {
    let conf_path = output_dir
        .join("runtime")
        .join("apache")
        .join("conf")
        .join("httpd.conf");

    if !conf_path.exists() {
        return Ok(());
    }

    let server_root = output_dir
        .join("runtime")
        .join("apache")
        .to_string_lossy()
        .replace('\\', "/");

    let input = fs::read_to_string(&conf_path)?;
    let mut output = String::with_capacity(input.len() + 64);
    let mut replaced_server_root = false;
    let mut replaced_listen = false;

    for line in input.lines() {
        let trimmed = line.trim_start();
        if !replaced_server_root && trimmed.starts_with("Define SRVROOT ") {
            output.push_str(&format!("Define SRVROOT \"{}\"\n", server_root));
            replaced_server_root = true;
            continue;
        }

        if !replaced_listen && trimmed.starts_with("Listen ") {
            output.push_str(&format!("Listen {}\n", http_port));
            replaced_listen = true;
            continue;
        }

        if let Some(module_file) = load_module_file_from_line(trimmed) {
            let module_path = output_dir
                .join("runtime")
                .join("apache")
                .join(module_file.replace('/', "\\"));
            if !module_path.exists() {
                output.push('#');
                output.push_str(line);
                output.push('\n');
                continue;
            }
        }

        output.push_str(line);
        output.push('\n');
    }

    if !replaced_server_root {
        output.push_str(&format!("Define SRVROOT \"{}\"\n", server_root));
    }
    if !replaced_listen {
        output.push_str(&format!("Listen {}\n", http_port));
    }

    fs::write(conf_path, output)?;
    Ok(())
}

fn load_module_file_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("LoadModule ") {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let _ = parts.next()?;
    let _ = parts.next()?;
    let file = parts.next()?;
    Some(file.replace('"', ""))
}

fn read_version_file(project_root: &Path) -> String {
    let version_path = project_root.join("version");
    let content = fs::read_to_string(version_path).unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let version = content.trim();
    if version.is_empty() {
        env!("CARGO_PKG_VERSION").to_string()
    } else {
        version.to_string()
    }
}

fn component_version_from_zip(zip_path: &Path) -> String {
    let file = zip_path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let stem = file.strip_suffix(".zip").unwrap_or(file);

    for token in stem.split(|c: char| !c.is_ascii_alphanumeric() && c != '.') {
        if token.is_empty() {
            continue;
        }

        if token.chars().next().map(|c| c.is_ascii_digit()) != Some(true) {
            continue;
        }

        let looks_like_version = token
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.');
        if looks_like_version {
            return token.to_string();
        }
    }

    "unbekannt".to_string()
}

fn write_welcome_page(
    output_dir: &Path,
    runtime_dir: &Path,
    web_root: Option<&str>,
    database_names: Option<&[String]>,
    version: &str,
    _apache_version: &str,
    _mariadb_version: &str,
) -> Result<(), BuilderError> {
    let web_root = web_root.unwrap_or("./Data/http");
    let configured_databases = database_names
        .map(|names| {
            let mut filtered: Vec<String> = names
                .iter()
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(|v| v.to_string())
                .collect();
            filtered.sort();
            filtered.dedup();
            filtered
        })
        .unwrap_or_default();
    let database_text = if configured_databases.is_empty() {
        "nicht konfiguriert".to_string()
    } else {
        configured_databases.join(", ")
    };
    let html = format!(
        "<!doctype html>\n<html lang=\"de\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>mowes-next</title>\n  <style>body {{ font-family: Segoe UI, sans-serif; margin: 2rem; }} .card {{ border: 1px solid #ddd; border-radius: 10px; padding: 1rem; max-width: 720px; }} h1 {{ margin-top: 0; }}</style>\n</head>\n<body>\n  <div class=\"card\">\n    <h1>Willkommen bei mowes-next {version}</h1>\n    <p><strong>Apache-Version:</strong> <span id=\"apache-version\">wird geladen...</span></p>\n    <p><strong>MariaDB-Version:</strong> <span id=\"mariadb-version\">wird geladen...</span></p>\n    <p><strong>Datenbanken:</strong> <span id=\"db-names\">{database_text}</span></p>\n  </div>\n  <script>\n    fetch('/versions.json', {{ cache: 'no-store' }})\n      .then(function(r) {{ return r.ok ? r.json() : Promise.reject(); }})\n      .then(function(v) {{\n        document.getElementById('apache-version').textContent = v.apache || 'unbekannt';\n        document.getElementById('mariadb-version').textContent = v.mariadb || 'unbekannt';\n      }})\n      .catch(function() {{\n        document.getElementById('apache-version').textContent = 'unbekannt';\n        document.getElementById('mariadb-version').textContent = 'unbekannt';\n      }});\n  </script>\n</body>\n</html>\n"
    );

    let root_path = Path::new(web_root);
    let package_web_root = if root_path.is_absolute() {
        output_dir.join("Data").join("http")
    } else {
        output_dir.join(root_path)
    };
    fs::create_dir_all(&package_web_root)?;
    fs::write(package_web_root.join("index.html"), &html)?;

    let apache_htdocs = runtime_dir.join("apache").join("htdocs");
    fs::create_dir_all(&apache_htdocs)?;
    fs::write(apache_htdocs.join("index.html"), html)?;

    Ok(())
}

fn write_launchers(output_dir: &Path) -> Result<(), BuilderError> {
    let start_bat = "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\nstart \"\" \"%~dp0mowes-next.exe\"\r\n";
    let headless_bat = "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\n\"%~dp0mowes-next.exe\" --headless\r\n";

    fs::write(output_dir.join("Start.bat"), start_bat)?;
    fs::write(output_dir.join("StartHeadless.bat"), headless_bat)?;

    Ok(())
}

fn write_control_gui(output_dir: &Path) -> Result<(), BuilderError> {
    let launcher_bat = "@echo off\r\nsetlocal\r\ncd /d \"%~dp0\"\r\npowershell -NoProfile -ExecutionPolicy Bypass -File \"%~dp0ControlGUI.ps1\"\r\n";

    let control_gui_ps1 = r#"[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidAssignmentToAutomaticVariable', '', Justification='Generated script false positive')]
param()

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$processStateDir = Join-Path $root "temp\procstate"
New-Item -ItemType Directory -Path $processStateDir -Force | Out-Null

function Get-ExistingExe {
    param([string[]]$Candidates)
    foreach ($candidate in $Candidates) {
        $path = Join-Path $root $candidate
        if (Test-Path $path) { return $path }
    }
    return $null
}

function Start-ServiceProcess {
    param(
        [string]$Name,
        [string[]]$Candidates,
        [string]$WorkingDir,
        [string[]]$Arguments = @()
    )

    $exe = Get-ExistingExe -Candidates $Candidates
    if (-not $exe) {
        throw "Executable fuer $Name nicht gefunden."
    }

    $workingPath = Join-Path $root $WorkingDir
    $sanitizedArguments = @($Arguments | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $startProcessParams = @{
        FilePath = $exe
        WorkingDirectory = $workingPath
        PassThru = $true
    }
    if ($sanitizedArguments.Count -gt 0) {
        $startProcessParams.ArgumentList = $sanitizedArguments
    }
    $process = Start-Process @startProcessParams
    Set-Content -Path (Join-Path $processStateDir "$Name.state") -Value $process.Id -Encoding ascii
    return "$Name gestartet (ID $($process.Id))"
}

function Stop-ServiceProcess {
    param([string]$Name)

    $processStateFile = Join-Path $processStateDir "$Name.state"
    if (-not (Test-Path $processStateFile)) {
        return "$Name nicht aktiv"
    }

    if ([string]::IsNullOrWhiteSpace((Get-Content $processStateFile | Select-Object -First 1).Trim())) {
        Remove-Item $processStateFile -Force -ErrorAction SilentlyContinue
        return "$Name Status-Datei leer"
    }

    $pid = [int](Get-Content $processStateFile | Select-Object -First 1).Trim()
    $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
    if (-not $proc) {
        Remove-Item $processStateFile -Force -ErrorAction SilentlyContinue
        return "$Name war bereits beendet"
    }

    try {
        Stop-Process -Id $pid -Force -ErrorAction Stop
        Remove-Item $processStateFile -Force -ErrorAction SilentlyContinue
        return "$Name gestoppt"
    } catch {
        Remove-Item $processStateFile -Force -ErrorAction SilentlyContinue
        return "$Name konnte nicht gestoppt werden: $($_.Exception.Message)"
    }
}

function Get-DbConfig {
    $cfgPath = Join-Path $root "config\default.config.json"
    if (-not (Test-Path $cfgPath)) {
        return $null
    }

    try {
        return Get-Content $cfgPath -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Get-ConfiguredDatabaseNames {
    $cfg = Get-DbConfig
    if (-not $cfg -or -not $cfg.mariadb) {
        return @()
    }

    $names = @()
    if ($cfg.mariadb.database_name) {
        $names += [string]$cfg.mariadb.database_name
    }
    if ($cfg.mariadb.database_names) {
        foreach ($name in $cfg.mariadb.database_names) {
            if ($name) {
                $names += [string]$name
            }
        }
    }

    return @($names |
        ForEach-Object { $_.Trim() } |
        Where-Object { $_ -match '^[A-Za-z0-9_$]+$' } |
        Sort-Object -Unique)
}

function Ensure-ConfiguredDatabases {
    $cfg = Get-DbConfig
    $port = 3306
    $password = "root"
    if ($cfg -and $cfg.mariadb) {
        if ($cfg.mariadb.port) { $port = [int]$cfg.mariadb.port }
        if ($cfg.mariadb.root_password) { $password = [string]$cfg.mariadb.root_password }
    }

    $dbNames = Get-ConfiguredDatabaseNames
    if ($dbNames.Count -eq 0) {
        return "keine konfigurierten Datenbanken"
    }

    $client = Get-ExistingExe -Candidates @("runtime\\mariadb\\bin\\mariadb.exe", "runtime\\mariadb\\bin\\mysql.exe")
    if (-not $client) {
        throw "mariadb.exe/mysql.exe nicht gefunden (fuer DB-Initialisierung erforderlich)."
    }

    $baseArgs = @("-h", "127.0.0.1", "-P", "$port", "-u", "root", "-N")
    if (-not [string]::IsNullOrWhiteSpace($password)) {
        $baseArgs += "-p$password"
    }

    $ready = $false
    $lastErr = ""
    for ($i = 0; $i -lt 20; $i++) {
        $probe = & $client @baseArgs -e "SELECT 1;" 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0) {
            $ready = $true
            break
        }
        $lastErr = ($probe.Trim())
        Start-Sleep -Milliseconds 350
    }

    if (-not $ready) {
        throw "DB-Ready-Check fehlgeschlagen: $lastErr"
    }

    $created = @()
    $existing = @()
    foreach ($dbName in $dbNames) {
        $existsSql = "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '$dbName';"
        $existsRaw = & $client @baseArgs -e $existsSql 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "DB-Existenzpruefung fehlgeschlagen fuer $dbName: $($existsRaw.Trim())"
        }

        $exists = @($existsRaw -split "`r?`n" | Where-Object { $_.Trim() -eq $dbName }).Count -gt 0
        if ($exists) {
            $existing += $dbName
            continue
        }

        $createSql = "CREATE DATABASE IF NOT EXISTS ``$dbName`` CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;"
        $createRaw = & $client @baseArgs -e $createSql 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) {
            throw "DB-Anlage fehlgeschlagen fuer $dbName: $($createRaw.Trim())"
        }
        $created += $dbName
    }

    if ($created.Count -gt 0 -and $existing.Count -gt 0) {
        return "erstellt ($($created -join ', ')), bereits vorhanden ($($existing -join ', '))"
    }
    if ($created.Count -gt 0) {
        return "erstellt ($($created -join ', '))"
    }
    return "bereits vorhanden ($($existing -join ', '))"
}

$form = New-Object System.Windows.Forms.Form
$form.Text = "MoWeS-Next Control"
$form.Size = New-Object System.Drawing.Size(460, 220)
$form.StartPosition = "CenterScreen"

$startButton = New-Object System.Windows.Forms.Button
$startButton.Text = "Start"
$startButton.Size = New-Object System.Drawing.Size(120, 34)
$startButton.Location = New-Object System.Drawing.Point(20, 20)

$stopButton = New-Object System.Windows.Forms.Button
$stopButton.Text = "Stop"
$stopButton.Size = New-Object System.Drawing.Size(120, 34)
$stopButton.Location = New-Object System.Drawing.Point(160, 20)

$statusBox = New-Object System.Windows.Forms.RichTextBox
$statusBox.ReadOnly = $true
$statusBox.Multiline = $true
$statusBox.DetectUrls = $false
$statusBox.Size = New-Object System.Drawing.Size(410, 105)
$statusBox.Location = New-Object System.Drawing.Point(20, 70)

function Add-StatusLine {
    param(
        [string]$line,
        [string]$level = "INFO"
    )

    $color = [System.Drawing.Color]::Black
    switch ($level.ToUpperInvariant()) {
        "OK" { $color = [System.Drawing.Color]::ForestGreen; break }
        "WARN" { $color = [System.Drawing.Color]::DarkOrange; break }
        "ERR" { $color = [System.Drawing.Color]::Firebrick; break }
        default { $color = [System.Drawing.Color]::Black; break }
    }

    $statusBox.SelectionStart = $statusBox.TextLength
    $statusBox.SelectionLength = 0
    $statusBox.SelectionColor = $color
    $statusBox.AppendText("$line`r`n")
    $statusBox.SelectionColor = $statusBox.ForeColor
    $statusBox.ScrollToCaret()
}

$startButton.Add_Click({
    try {
        New-Item -ItemType Directory -Path (Join-Path $root "Data\\SQL") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $root "logs") -Force | Out-Null

        $apache = Start-ServiceProcess -Name "apache" -Candidates @("runtime\\apache\\bin\\httpd.exe", "runtime\\apache\\httpd.exe") -WorkingDir "runtime\\apache"
        $mariadbArgs = @(
            "--defaults-file=$($root)\\runtime\\generated\\my.generated.ini",
            "--datadir=$($root)\\Data\\SQL",
            "--log-error=$($root)\\logs\\mariadb-error.log",
            "--pid-file=$($root)\\temp\\mariadb.pid"
        )
        $mariadb = Start-ServiceProcess -Name "mariadb" -Candidates @("runtime\\mariadb\\bin\\mariadbd.exe", "runtime\\mariadb\\mariadbd.exe", "runtime\\mariadb\\bin\\mysqld.exe", "runtime\\mariadb\\mysqld.exe") -WorkingDir "." -Arguments $mariadbArgs
        $dbReport = Ensure-ConfiguredDatabases
        Add-StatusLine -line $apache -level "OK"
        Add-StatusLine -line $mariadb -level "OK"
        Add-StatusLine -line "DB: $dbReport" -level "INFO"
    } catch {
        Add-StatusLine -line "Start fehlgeschlagen: $($_.Exception.Message)" -level "ERR"
    }
})

$stopButton.Add_Click({
    Add-StatusLine -line (Stop-ServiceProcess -Name "mariadb") -level "INFO"
    Add-StatusLine -line (Stop-ServiceProcess -Name "apache") -level "INFO"
})

$form.Controls.Add($startButton)
$form.Controls.Add($stopButton)
$form.Controls.Add($statusBox)
[void]$form.ShowDialog()
"#;

    fs::write(output_dir.join("ControlGUI.bat"), launcher_bat)?;
    fs::write(output_dir.join("ControlGUI.ps1"), control_gui_ps1)?;
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
