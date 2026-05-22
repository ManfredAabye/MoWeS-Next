use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::paths::resolve_package_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceModeConfig {
    pub enabled: bool,
    pub executable: String,
    pub working_directory: String,
    pub pid_directory: String,
    pub auto_start: bool,
}

#[derive(Debug, Clone)]
pub struct ServiceModeStatus {
    pub configured: bool,
    pub config_path: PathBuf,
    pub details: String,
}

pub fn prepare_service_mode(project_root: &Path) -> io::Result<ServiceModeStatus> {
    let service_dir = project_root.join("service");
    fs::create_dir_all(&service_dir)?;
    fs::create_dir_all(project_root.join("temp").join("pids"))?;

    let package_dir = resolve_package_dir(project_root)
        .unwrap_or_else(|_| project_root.join("dist").join("mowes-next-package"));

    let config = ServiceModeConfig {
        enabled: true,
        executable: package_dir.join("mowes-next.exe").display().to_string(),
        working_directory: project_root.display().to_string(),
        pid_directory: project_root.join("temp").join("pids").display().to_string(),
        auto_start: true,
    };

    let config_path = service_dir.join("service-mode.json");
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
    )?;

    fs::write(
        service_dir.join("install-service.bat"),
        "@echo off\r\necho Service mode is prepared. Integrate this config with a Windows service wrapper.\r\n",
    )?;
    fs::write(
        service_dir.join("uninstall-service.bat"),
        "@echo off\r\necho Remove Windows service wrapper integration here.\r\n",
    )?;

    Ok(ServiceModeStatus {
        configured: true,
        config_path,
        details: "service configuration prepared".to_string(),
    })
}

pub fn service_status(project_root: &Path) -> io::Result<ServiceModeStatus> {
    let config_path = project_root.join("service").join("service-mode.json");
    if config_path.exists() {
        Ok(ServiceModeStatus {
            configured: true,
            config_path,
            details: "service configuration present".to_string(),
        })
    } else {
        Ok(ServiceModeStatus {
            configured: false,
            config_path,
            details: "service configuration missing".to_string(),
        })
    }
}
