use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DEFAULT_PACKAGE_NAME: &str = "mowes-next-package";

pub fn resolve_package_dir(project_root: &Path) -> io::Result<PathBuf> {
    let dist_dir = project_root.join("dist");
    let default_dir = dist_dir.join(DEFAULT_PACKAGE_NAME);
    if default_dir.exists() {
        return Ok(default_dir);
    }

    let mut candidates: Vec<(PathBuf, SystemTime)> = Vec::new();
    for entry in fs::read_dir(&dist_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if !path.join("runtime").exists() {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        candidates.push((path, modified));
    }

    candidates
        .into_iter()
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no built package found under dist"))
}
