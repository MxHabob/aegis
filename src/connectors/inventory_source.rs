use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImportedAsset {
    pub host: String,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_asset_type")]
    pub asset_type: String,
    #[serde(default)]
    pub ip_addresses: Vec<String>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default)]
    pub locations: Vec<String>,
    #[serde(default)]
    pub applications: Vec<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub service_labels: Vec<String>,
    #[serde(default)]
    pub web_labels: Vec<String>,
    #[serde(default)]
    pub software_vendors: Vec<String>,
    #[serde(default)]
    pub software_products: Vec<String>,
    #[serde(default)]
    pub certificate_labels: Vec<String>,
    #[serde(default)]
    pub issuers: Vec<String>,
    #[serde(default)]
    pub endpoints: Vec<String>,
    #[serde(default)]
    pub matched_fields: BTreeMap<String, String>,
}

impl Default for ImportedAsset {
    fn default() -> Self {
        Self {
            host: String::new(),
            label: String::new(),
            asset_type: default_asset_type(),
            ip_addresses: Vec::new(),
            ports: Vec::new(),
            networks: Vec::new(),
            locations: Vec::new(),
            applications: Vec::new(),
            protocols: Vec::new(),
            service_labels: Vec::new(),
            web_labels: Vec::new(),
            software_vendors: Vec::new(),
            software_products: Vec::new(),
            certificate_labels: Vec::new(),
            issuers: Vec::new(),
            endpoints: Vec::new(),
            matched_fields: BTreeMap::new(),
        }
    }
}

fn default_asset_type() -> String {
    "host".into()
}

pub trait InventorySource {
    fn name(&self) -> &'static str;

    // Sources should read from local files or authorized inventory APIs, not probe targets.
    fn load_assets(&self) -> Result<Vec<ImportedAsset>>;
}

pub struct JsonFileSource {
    path: PathBuf,
}

impl JsonFileSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl InventorySource for JsonFileSource {
    fn name(&self) -> &'static str {
        "json-file"
    }

    fn load_assets(&self) -> Result<Vec<ImportedAsset>> {
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
        serde_json::from_str(content)
            .with_context(|| format!("invalid inventory JSON in {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_source_accepts_utf8_bom() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inventory.json");
        fs::write(
            &path,
            "\u{feff}[{\"host\":\"api.example.com\",\"label\":\"API\"}]",
        )
        .unwrap();
        let assets = JsonFileSource::new(path).load_assets().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].host, "api.example.com");
    }

    #[test]
    fn json_source_loads_inventory_facet_data() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inventory.json");
        fs::write(
            &path,
            r#"[{"host":"api.example.com","label":"API","ports":[443],"applications":["HTTP"],"locations":["Saudi Arabia"],"software_products":["nginx"]}]"#,
        )
        .unwrap();
        let assets = JsonFileSource::new(path).load_assets().unwrap();
        assert_eq!(assets[0].ports, vec![443]);
        assert_eq!(assets[0].applications, vec!["HTTP"]);
        assert_eq!(assets[0].locations, vec!["Saudi Arabia"]);
        assert_eq!(assets[0].software_products, vec!["nginx"]);
    }
}
