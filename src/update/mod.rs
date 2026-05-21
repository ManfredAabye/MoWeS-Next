use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub download_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCheck {
    pub current_version: String,
    pub manifest_path: PathBuf,
    pub update_available: bool,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct UpdateApplyReport {
    pub manifest_path: PathBuf,
    pub applied_to: PathBuf,
    pub files_copied: usize,
}

pub fn check_local_update(project_root: &Path, current_version: &str) -> io::Result<UpdateCheck> {
    let manifest_path = project_root.join("updates").join("update.manifest.json");
    if !manifest_path.exists() {
        return Ok(UpdateCheck {
            current_version: current_version.to_string(),
            manifest_path,
            update_available: false,
            details: "no update manifest found".to_string(),
        });
    }

    let raw = fs::read_to_string(&manifest_path)?;
    let manifest: UpdateManifest = serde_json::from_str(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let update_available = compare_version_strings(current_version, &manifest.version) == std::cmp::Ordering::Less;
    Ok(UpdateCheck {
        current_version: current_version.to_string(),
        manifest_path,
        update_available,
        details: if update_available {
            format!("update available: {}", manifest.version)
        } else {
            format!("up to date: {}", current_version)
        },
    })
}

pub fn write_update_manifest(project_root: &Path, manifest: &UpdateManifest) -> io::Result<PathBuf> {
    let updates_dir = project_root.join("updates");
    fs::create_dir_all(&updates_dir)?;
    let manifest_path = updates_dir.join("update.manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(manifest).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
    )?;
    Ok(manifest_path)
}

pub fn apply_update_bundle(project_root: &Path, bundle_dir: &Path) -> io::Result<UpdateApplyReport> {
    let manifest_path = bundle_dir.join("update.manifest.json");
    let raw = fs::read_to_string(&manifest_path)?;
    let manifest: UpdateManifest = serde_json::from_str(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let target_dir = project_root.join("updates").join("applied").join(&manifest.version);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    let mut files_copied = 0usize;
    copy_bundle_contents(bundle_dir, &target_dir, &manifest_path, &mut files_copied)?;

    Ok(UpdateApplyReport {
        manifest_path,
        applied_to: target_dir,
        files_copied,
    })
}

pub fn compare_version_strings(a: &str, b: &str) -> std::cmp::Ordering {
    parse_version_tuple(a).cmp(&parse_version_tuple(b))
}

fn parse_version_tuple(value: &str) -> (u32, u32, u32) {
    let mut parts = value.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

fn copy_bundle_contents(source: &Path, destination: &Path, manifest_path: &Path, files_copied: &mut usize) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let src_path = entry.path();
        if src_path == manifest_path {
            continue;
        }

        let dst_path = destination.join(entry.file_name());
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_bundle_contents(&src_path, &dst_path, manifest_path, files_copied)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
            *files_copied += 1;
        }
    }
    Ok(())
}
