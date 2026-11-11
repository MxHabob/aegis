use crate::{
    audit::append_event,
    connectors::ImportedAsset,
    core::models::{AppState, Asset, Scope},
    modules::default_modules,
    scope::{is_within_scope, normalize_group, normalize_host},
    storage::{SavedMap, list_maps, load_map, load_state, save_map, save_state},
};
use anyhow::{Context, Result, bail};
use std::{fs, path::PathBuf};

pub struct AppContext {
    data_dir: PathBuf,
    state: AppState,
}

impl AppContext {
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        let state = load_state(&data_dir)?;
        Ok(Self { data_dir, state })
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    pub fn save_map(&self, id: &str, target: &str, group: &str) -> Result<SavedMap> {
        let saved = save_map(&self.data_dir, id, target, group, &self.state)?;
        append_event(
            &self.data_dir,
            "map.save",
            &format!("{}:{}:{}", saved.group, saved.target, saved.id),
        )?;
        Ok(saved)
    }

    pub fn saved_maps(&self) -> Result<Vec<SavedMap>> {
        list_maps(&self.data_dir)
    }

    pub fn open_saved_map(&self, id: &str) -> Result<SavedMap> {
        let saved = load_map(&self.data_dir, id)?;
        append_event(
            &self.data_dir,
            "map.open",
            &format!("{}:{}", saved.group, saved.id),
        )?;
        Ok(saved)
    }

    pub fn add_scope(&mut self, input: &str, group: &str) -> Result<()> {
        let target = normalize_host(input)?;
        let group = normalize_group(group)?;
        if self
            .state
            .scopes
            .iter()
            .any(|scope| scope.target == target && scope.group == group)
        {
            bail!("{target} is already in the {group} scope");
        }
        let mut next = self.state.clone();
        next.scopes.push(Scope {
            target: target.clone(),
            added_at: now()?,
            group: group.clone(),
        });
        self.persist(next, "scope.add", &format!("{group}:{target}"))
    }

    pub fn remove_scope(&mut self, input: &str, group: &str) -> Result<()> {
        let target = normalize_host(input)?;
        let group = normalize_group(group)?;
        if self
            .state
            .assets
            .iter()
            .any(|asset| asset.group == group && is_within_scope(&asset.host, &target))
        {
            bail!("cannot remove {target} from {group} while inventory assets still depend on it");
        }
        let mut next = self.state.clone();
        let previous_len = next.scopes.len();
        next.scopes
            .retain(|scope| scope.target != target || scope.group != group);
        if next.scopes.len() == previous_len {
            bail!("{target} is not in the {group} scope");
        }
        self.persist(next, "scope.remove", &format!("{group}:{target}"))
    }

    pub fn add_asset(&mut self, input: &str, label: &str, group: &str) -> Result<()> {
        self.add_asset_with_type(input, label, group, "host")
    }

    pub fn add_asset_with_type(
        &mut self,
        input: &str,
        label: &str,
        group: &str,
        asset_type: &str,
    ) -> Result<()> {
        let host = normalize_host(input)?;
        let group = normalize_group(group)?;
        let asset_type = asset_type.trim().to_ascii_lowercase();
        if !["host", "certificate", "web_property", "camera"].contains(&asset_type.as_str()) {
            bail!("unsupported asset type '{asset_type}'");
        }
        if !self
            .state
            .scopes
            .iter()
            .any(|scope| scope.group == group && is_within_scope(&host, &scope.target))
        {
            bail!("{host} is not covered by an explicitly authorized {group} scope");
        }
        if self
            .state
            .assets
            .iter()
            .any(|asset| asset.host == host && asset.group == group)
        {
            bail!("{host} is already in the {group} inventory");
        }
        let label = label.trim();
        if label.is_empty() {
            bail!("asset label cannot be empty");
        }
        if label.len() > 120 {
            bail!("asset label cannot exceed 120 bytes");
        }
        let mut next = self.state.clone();
        next.assets.push(Asset {
            host: host.clone(),
            label: label.to_owned(),
            added_at: now()?,
            asset_type,
            group: group.clone(),
            ..Asset::default()
        });
        self.persist(next, "asset.add", &format!("{group}:{host}"))
    }

    pub fn remove_asset(&mut self, input: &str, group: &str) -> Result<()> {
        let host = normalize_host(input)?;
        let group = normalize_group(group)?;
        let mut next = self.state.clone();
        let previous_len = next.assets.len();
        next.assets
            .retain(|asset| asset.host != host || asset.group != group);
        if next.assets.len() == previous_len {
            bail!("{host} is not in the {group} inventory");
        }
        self.persist(next, "asset.remove", &format!("{group}:{host}"))
    }

    pub fn import_assets(&mut self, assets: &[ImportedAsset], group: &str) -> Result<usize> {
        let group = normalize_group(group)?;
        let mut next = self.state.clone();
        let mut imported = Vec::with_capacity(assets.len());
        for item in assets {
            let host = normalize_host(&item.host)?;
            if !next
                .scopes
                .iter()
                .any(|scope| scope.group == group && is_within_scope(&host, &scope.target))
            {
                bail!("{host} is not covered by an explicitly authorized {group} scope");
            }
            if next
                .assets
                .iter()
                .any(|asset| asset.host == host && asset.group == group)
                || imported
                    .iter()
                    .any(|asset: &Asset| asset.host == host && asset.group == group)
            {
                bail!("{host} is already present in inventory");
            }
            let label = item.label.trim();
            if label.is_empty() || label.len() > 120 {
                bail!("asset labels must contain between 1 and 120 bytes");
            }
            let asset_type = item.asset_type.trim().to_ascii_lowercase();
            if !["host", "certificate", "web_property", "camera"].contains(&asset_type.as_str()) {
                bail!(
                    "unsupported asset type '{}'; use host, certificate, web_property, or camera",
                    item.asset_type
                );
            }
            imported.push(Asset {
                host,
                label: label.to_owned(),
                added_at: now()?,
                asset_type,
                group: group.clone(),
                ip_addresses: item.ip_addresses.clone(),
                ports: item.ports.clone(),
                networks: item.networks.clone(),
                locations: item.locations.clone(),
                applications: item.applications.clone(),
                protocols: item.protocols.clone(),
                service_labels: item.service_labels.clone(),
                web_labels: item.web_labels.clone(),
                software_vendors: item.software_vendors.clone(),
                software_products: item.software_products.clone(),
                certificate_labels: item.certificate_labels.clone(),
                issuers: item.issuers.clone(),
                endpoints: item.endpoints.clone(),
                matched_fields: item.matched_fields.clone(),
            });
        }
        let count = imported.len();
        if count == 0 {
            return Ok(0);
        }
        next.assets.extend(imported);
        self.persist(next, "inventory.import", &format!("{group}:{count} assets"))?;
        Ok(count)
    }

    pub fn findings(&self) -> Result<Vec<crate::core::models::Finding>> {
        let mut findings = Vec::new();
        for module in default_modules() {
            findings.extend(
                module
                    .run(&self.state)
                    .with_context(|| format!("module '{}' failed", module.name()))?,
            );
        }
        Ok(findings)
    }

    pub fn audit_log_path(&self) -> PathBuf {
        self.data_dir.join("audit.jsonl")
    }

    pub fn read_audit_log(&self) -> Result<String> {
        let path = self.audit_log_path();
        if !path.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
    }

    fn persist(&mut self, next: AppState, action: &str, target: &str) -> Result<()> {
        save_state(&self.data_dir, &next)?;
        append_event(&self.data_dir, action, target)?;
        self.state = next;
        Ok(())
    }
}

fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::Severity;

    #[test]
    fn scopes_and_inventory_persist_and_remove_safely() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = AppContext::open(directory.path()).unwrap();
        app.add_scope("example.com", "default").unwrap();
        app.add_asset("api.example.com", "API", "default").unwrap();
        assert!(app.remove_scope("example.com", "default").is_err());
        app.remove_asset("api.example.com", "default").unwrap();
        app.remove_scope("example.com", "default").unwrap();
        assert!(
            AppContext::open(directory.path())
                .unwrap()
                .state()
                .scopes
                .is_empty()
        );
    }

    #[test]
    fn integrity_check_reports_edited_out_of_scope_asset() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = AppContext::open(directory.path()).unwrap();
        app.add_scope("example.com", "default").unwrap();
        app.state.assets.push(Asset {
            host: "other.example.net".into(),
            label: "Unexpected".into(),
            added_at: 0,
            asset_type: "host".into(),
            group: "default".into(),
            ..Asset::default()
        });
        let findings = app.findings().unwrap();
        assert_eq!(findings.len(), 1);
        assert!(matches!(findings[0].severity, Severity::High));
    }

    #[test]
    fn import_validates_every_record_before_persisting() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = AppContext::open(directory.path()).unwrap();
        app.add_scope("example.com", "production").unwrap();
        let batch = vec![
            ImportedAsset {
                host: "api.example.com".into(),
                label: "API".into(),
                ..ImportedAsset::default()
            },
            ImportedAsset {
                host: "other.example.net".into(),
                label: "Outside".into(),
                ..ImportedAsset::default()
            },
        ];

        assert!(app.import_assets(&batch, "production").is_err());
        assert!(app.state().assets.is_empty());
        assert_eq!(app.import_assets(&batch[..1], "production").unwrap(), 1);
        assert_eq!(app.state().assets[0].group, "production");
    }
}
