use crate::core::models::AppState;
use crate::scope::{is_within_scope, normalize_group, normalize_host};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMap {
    pub id: String,
    pub group: String,
    pub target: String,
    pub created_at: u64,
    pub state: AppState,
}

pub fn save_map(
    directory: &Path,
    id: &str,
    target: &str,
    group: &str,
    state: &AppState,
) -> Result<SavedMap> {
    validate_id(id)?;
    let target = normalize_host(target)?;
    let group = normalize_group(group)?;
    if !state
        .scopes
        .iter()
        .any(|scope| scope.group == group && is_within_scope(&target, &scope.target))
    {
        bail!("{target} is not covered by an authorized scope in group '{group}'");
    }
    let mut snapshot = state.clone();
    snapshot
        .scopes
        .retain(|scope| scope.group == group && is_within_scope(&target, &scope.target));
    snapshot
        .assets
        .retain(|asset| asset.group == group && is_within_scope(&asset.host, &target));
    let saved = SavedMap {
        id: id.to_owned(),
        group: group.to_owned(),
        target,
        created_at: now()?,
        state: snapshot,
    };
    let archive_dir = directory.join("archives");
    fs::create_dir_all(&archive_dir)
        .with_context(|| format!("failed to create {}", archive_dir.display()))?;
    let path = archive_dir.join(format!("{id}.json"));
    if path.exists() {
        bail!("an archive named '{id}' already exists");
    }
    let content = serde_json::to_string_pretty(&saved)?;
    fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(saved)
}

pub fn list_maps(directory: &Path) -> Result<Vec<SavedMap>> {
    let archive_dir = directory.join("archives");
    if !archive_dir.exists() {
        return Ok(Vec::new());
    }
    let mut maps = Vec::new();
    for entry in fs::read_dir(&archive_dir)
        .with_context(|| format!("failed to read {}", archive_dir.display()))?
    {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            let path = entry.path();
            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .context("archive filename is not valid UTF-8")?;
            maps.push(load_map(directory, id)?);
        }
    }
    maps.sort_by_key(|map| Reverse(map.created_at));
    Ok(maps)
}

pub fn load_map(directory: &Path, id: &str) -> Result<SavedMap> {
    validate_id(id)?;
    let path = directory.join("archives").join(format!("{id}.json"));
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read archive {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("invalid archive {}", path.display()))
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        || id == "."
        || id == ".."
    {
        bail!("archive names must be 1-80 characters using letters, numbers, '.', '_' or '-'");
    }
    Ok(())
}

fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{Asset, Scope};

    #[test]
    fn archive_round_trip_keeps_only_selected_group() {
        let directory = tempfile::tempdir().unwrap();
        let state = AppState {
            scopes: vec![
                Scope {
                    target: "example.com".into(),
                    group: "prod".into(),
                    added_at: 1,
                },
                Scope {
                    target: "example.net".into(),
                    group: "prod".into(),
                    added_at: 2,
                },
            ],
            assets: vec![
                Asset {
                    host: "api.example.com".into(),
                    group: "prod".into(),
                    ..Asset::default()
                },
                Asset {
                    host: "api.example.net".into(),
                    group: "prod".into(),
                    ..Asset::default()
                },
            ],
        };
        save_map(
            directory.path(),
            "prod-snapshot",
            "example.com",
            "prod",
            &state,
        )
        .unwrap();
        let loaded = load_map(directory.path(), "prod-snapshot").unwrap();
        assert_eq!(loaded.state.scopes.len(), 1);
        assert_eq!(loaded.state.assets.len(), 1);
        assert_eq!(loaded.state.assets[0].host, "api.example.com");
        assert_eq!(loaded.target, "example.com");
        assert_eq!(list_maps(directory.path()).unwrap().len(), 1);
    }

    #[test]
    fn archive_rejects_path_traversal() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            save_map(
                directory.path(),
                "..\\escape",
                "example.com",
                "prod",
                &AppState::default()
            )
            .is_err()
        );
    }

    #[test]
    fn archive_name_cannot_be_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let state = AppState {
            scopes: vec![Scope {
                target: "example.com".into(),
                group: "prod".into(),
                added_at: 1,
            }],
            ..AppState::default()
        };
        save_map(directory.path(), "snapshot", "example.com", "prod", &state).unwrap();
        assert!(save_map(directory.path(), "snapshot", "example.com", "prod", &state).is_err());
    }

    #[test]
    fn archive_requires_authorized_scope_in_group() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            save_map(
                directory.path(),
                "snapshot",
                "example.com",
                "prod",
                &AppState::default()
            )
            .is_err()
        );
    }
}
