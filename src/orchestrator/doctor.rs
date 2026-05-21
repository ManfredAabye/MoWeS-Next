use std::path::{Path, PathBuf};

use crate::builder::package::detect_components;

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub details: String,
}

impl DoctorReport {
    pub fn is_ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }
}

pub fn run_doctor(project_root: &Path) -> DoctorReport {
    let mut checks = Vec::new();

    let components_dir = project_root.join("Components");
    checks.push(exists_check(
        "components_dir",
        &components_dir,
        "Components folder exists",
    ));

    match detect_components(project_root) {
        Ok((apache_zip, mariadb_zip)) => {
            checks.push(DoctorCheck {
                name: "apache_zip_detected".to_string(),
                ok: true,
                details: apache_zip.display().to_string(),
            });
            checks.push(DoctorCheck {
                name: "mariadb_zip_detected".to_string(),
                ok: true,
                details: mariadb_zip.display().to_string(),
            });
        }
        Err(err) => {
            checks.push(DoctorCheck {
                name: "component_scan".to_string(),
                ok: false,
                details: err.to_string(),
            });
        }
    }

    let dist_package = project_root.join("dist").join("mowes-next-package");
    checks.push(exists_check(
        "dist_package_dir",
        &dist_package,
        "Built package exists",
    ));

    let runtime_apache = dist_package.join("runtime").join("apache");
    let runtime_mariadb = dist_package.join("runtime").join("mariadb");
    checks.push(exists_check(
        "runtime_apache_dir",
        &runtime_apache,
        "Apache runtime extracted",
    ));
    checks.push(exists_check(
        "runtime_mariadb_dir",
        &runtime_mariadb,
        "MariaDB runtime extracted",
    ));

    let generated_dir = dist_package.join("runtime").join("generated");
    checks.push(exists_check(
        "generated_configs_dir",
        &generated_dir,
        "Generated config directory exists",
    ));

    checks.push(file_any_check(
        "apache_binary",
        &runtime_apache,
        &["bin/httpd.exe", "httpd.exe"],
    ));
    checks.push(file_any_check(
        "mariadb_binary",
        &runtime_mariadb,
        &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"],
    ));

    DoctorReport { checks }
}

fn exists_check(name: &str, path: &Path, label: &str) -> DoctorCheck {
    DoctorCheck {
        name: name.to_string(),
        ok: path.exists(),
        details: format!("{}: {}", label, path.display()),
    }
}

fn file_any_check(name: &str, root: &Path, candidates: &[&str]) -> DoctorCheck {
    let found = find_first_existing(root, candidates);
    match found {
        Some(path) => DoctorCheck {
            name: name.to_string(),
            ok: true,
            details: path.display().to_string(),
        },
        None => DoctorCheck {
            name: name.to_string(),
            ok: false,
            details: format!(
                "none of [{}] found under {}",
                candidates.join(", "),
                root.display()
            ),
        },
    }
}

fn find_first_existing(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    for rel in candidates {
        let p = root.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
}
