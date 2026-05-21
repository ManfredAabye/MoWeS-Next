use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry_point: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PluginInventory {
    pub plugin_dir: PathBuf,
    pub plugins: Vec<PluginDescriptor>,
}

pub fn discover_plugins(project_root: &Path) -> io::Result<PluginInventory> {
    let plugin_dir = project_root.join("plugins");
    fs::create_dir_all(&plugin_dir)?;

    let mut plugins = Vec::new();
    for entry in fs::read_dir(&plugin_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }

        let raw = fs::read_to_string(&path)?;
        if let Ok(plugin) = serde_json::from_str::<PluginDescriptor>(&raw) {
            plugins.push(plugin);
        }
    }

    plugins.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(PluginInventory { plugin_dir, plugins })
}

pub fn write_default_plugin_descriptor(project_root: &Path) -> io::Result<PathBuf> {
    let plugin_dir = project_root.join("plugins");
    fs::create_dir_all(&plugin_dir)?;

    let descriptor = PluginDescriptor {
        id: "sample-plugin".to_string(),
        name: "Sample Plugin".to_string(),
        version: "1.0.0".to_string(),
        entry_point: None,
        enabled: false,
    };

    let path = plugin_dir.join("sample-plugin.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&descriptor)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?,
    )?;

    Ok(path)
}
