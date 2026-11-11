use crate::core::models::AppState;
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn load_state(directory: &Path) -> Result<AppState> {
    let path = directory.join("state.json");
    if !path.exists() {
        return Ok(AppState::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("invalid state file {}", path.display()))
}

pub fn save_state(directory: &Path, state: &AppState) -> Result<()> {
    fs::create_dir_all(directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;
    let path = directory.join("state.json");
    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_state_loads_as_empty() {
        let directory = tempfile::tempdir().unwrap();
        let state = load_state(directory.path()).unwrap();
        assert!(state.scopes.is_empty());
        assert!(state.assets.is_empty());
    }

    #[test]
    fn older_state_without_assets_loads() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path()).unwrap();
        fs::write(
            directory.path().join("state.json"),
            r#"{"scopes":[{"target":"example.com","added_at":1}],"assets":[{"host":"api.example.com","label":"API","added_at":2}]}"#,
        )
        .unwrap();
        let state = load_state(directory.path()).unwrap();
        assert_eq!(state.scopes[0].group, "default");
        assert_eq!(state.assets[0].group, "default");
    }
}
