use std::path::Path;
use std::process::{Child, Command, Stdio};

#[derive(Debug)]
pub struct ServiceProcess {
    pub name: String,
    pub child: Option<Child>,
}

impl ServiceProcess {
    pub fn start_apache(runtime_dir: &Path) -> std::io::Result<Self> {
        let exe = find_exe(runtime_dir, &["bin/httpd.exe", "httpd.exe"])?;
        let child = Command::new(exe)
            .current_dir(runtime_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(ServiceProcess { name: "apache".to_string(), child: Some(child) })
    }
    pub fn start_mariadb(runtime_dir: &Path) -> std::io::Result<Self> {
        let exe = find_exe(runtime_dir, &["bin/mariadbd.exe", "mariadbd.exe", "bin/mysqld.exe", "mysqld.exe"])?;
        let child = Command::new(exe)
            .current_dir(runtime_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(ServiceProcess { name: "mariadb".to_string(), child: Some(child) })
    }
    pub fn stop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }
    pub fn is_running(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_)) => false,
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

fn find_exe(runtime_dir: &Path, candidates: &[&str]) -> std::io::Result<String> {
    for rel in candidates {
        let exe = runtime_dir.join(rel);
        if exe.exists() {
            return Ok(exe.to_string_lossy().to_string());
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Executable not found"))
}
