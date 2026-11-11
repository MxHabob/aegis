use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct AuditEvent<'a> {
    pub timestamp: u64,
    pub action: &'a str,
    pub target: &'a str,
}

pub fn append_event(directory: &Path, action: &str, target: &str) -> Result<()> {
    let path = directory.join("audit.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_secs();
    serde_json::to_writer(
        &mut file,
        &AuditEvent {
            timestamp,
            action,
            target,
        },
    )?;
    file.write_all(b"\n")?;
    Ok(())
}
