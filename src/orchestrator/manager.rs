use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct RuntimeLayout {
    pub runtime_root: PathBuf,
    pub temp_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProcessStatus {
    pub name: &'static str,
    pub running: bool,
    pub pid: Option<u32>,
    pub details: String,
}

pub fn start_process(name: &'static str, runtime_dir: &Path, pid_dir: &Path, candidates: &[&str], args: &[&str]) -> io::Result<ProcessStatus> {
    fs::create_dir_all(pid_dir)?;
    let exe = find_executable(runtime_dir, candidates)?;
    let child = Command::new(&exe)
        .args(args)
        .current_dir(runtime_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let pid = child.id();
    let pid_file = pid_dir.join(format!("{name}.pid"));
    fs::write(&pid_file, pid.to_string())?;

    Ok(ProcessStatus {
        name,
        running: true,
        pid: Some(pid),
        details: format!("started pid {pid}"),
    })
}

pub fn stop_process(name: &'static str, pid_dir: &Path) -> io::Result<ProcessStatus> {
    let pid_file = pid_dir.join(format!("{name}.pid"));
    let pid = read_pid(&pid_file)?;

    if let Some(pid) = pid {
        terminate_pid(pid)?;
        let _ = fs::remove_file(&pid_file);
        Ok(ProcessStatus {
            name,
            running: false,
            pid: Some(pid),
            details: format!("stopped pid {pid}"),
        })
    } else {
        Ok(ProcessStatus {
            name,
            running: false,
            pid: None,
            details: "not running".to_string(),
        })
    }
}

pub fn status_process(name: &'static str, pid_dir: &Path) -> io::Result<ProcessStatus> {
    let pid_file = pid_dir.join(format!("{name}.pid"));
    let pid = read_pid(&pid_file)?;

    if let Some(pid) = pid {
        let running = is_pid_running(pid)?;
        Ok(ProcessStatus {
            name,
            running,
            pid: Some(pid),
            details: if running {
                format!("running pid {pid}")
            } else {
                format!("stale pid file {pid}")
            },
        })
    } else {
        Ok(ProcessStatus {
            name,
            running: false,
            pid: None,
            details: "not running".to_string(),
        })
    }
}

fn find_executable(runtime_dir: &Path, candidates: &[&str]) -> io::Result<PathBuf> {
    for rel in candidates {
        let path = runtime_dir.join(rel);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "executable not found"))
}

fn read_pid(pid_file: &Path) -> io::Result<Option<u32>> {
    if !pid_file.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(pid_file)?;
    let pid = raw.trim().parse::<u32>().ok();
    Ok(pid)
}

fn terminate_pid(pid: u32) -> io::Result<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, format!("taskkill failed for pid {pid}")))
    }
}

fn is_pid_running(pid: u32) -> io::Result<bool> {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}")])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.contains(&pid.to_string()))
}