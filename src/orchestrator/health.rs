use std::io;
use std::path::Path;

use super::manager::{status_process, ProcessStatus};

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub apache: ProcessStatus,
    pub mariadb: ProcessStatus,
}

impl HealthReport {
    pub fn is_ready(&self) -> bool {
        self.apache.running && self.mariadb.running
    }
}

pub fn check_health(pid_dir: &Path) -> io::Result<HealthReport> {
    let apache = status_process("apache", pid_dir)?;
    let mariadb = status_process("mariadb", pid_dir)?;
    Ok(HealthReport { apache, mariadb })
}