use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct JsonLogger {
    log_file: std::path::PathBuf,
}

impl JsonLogger {
    pub fn new(log_file: std::path::PathBuf) -> io::Result<Self> {
        if let Some(parent) = log_file.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { log_file })
    }

    pub fn info(&self, event: &str, message: &str, data: Value) -> io::Result<()> {
        self.write("info", event, message, data)
    }

    pub fn error(&self, event: &str, message: &str, data: Value) -> io::Result<()> {
        self.write("error", event, message, data)
    }

    pub fn file_path(&self) -> &Path {
        &self.log_file
    }

    fn write(&self, level: &str, event: &str, message: &str, data: Value) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let line = json!({
            "ts_unix_ms": ts,
            "level": level,
            "event": event,
            "message": message,
            "data": data
        });

        writeln!(file, "{}", line)?;
        Ok(())
    }
}
