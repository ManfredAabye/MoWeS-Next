use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");

    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(value) => PathBuf::from(value),
        Err(err) => {
            println!("cargo:warning=Could not read CARGO_MANIFEST_DIR: {err}");
            return;
        }
    };

    let version = match env::var("CARGO_PKG_VERSION") {
        Ok(value) => value,
        Err(err) => {
            println!("cargo:warning=Could not read CARGO_PKG_VERSION: {err}");
            return;
        }
    };

    let version_path = manifest_dir.join("version");
    let desired_content = format!("{version}\n");

    let should_write = match fs::read_to_string(&version_path) {
        Ok(current) => current != desired_content,
        Err(_) => true,
    };

    if should_write {
        if let Err(err) = fs::write(&version_path, desired_content) {
            println!(
                "cargo:warning=Could not update version file at {}: {err}",
                version_path.display()
            );
        }
    }
}
