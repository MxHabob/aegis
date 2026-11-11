use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    #[serde(default)]
    pub scopes: Vec<Scope>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub target: String,
    pub added_at: u64,
    #[serde(default = "default_group")]
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Asset {
    pub host: String,
    pub label: String,
    pub added_at: u64,
    #[serde(default = "default_asset_type")]
    pub asset_type: String,
    #[serde(default = "default_group")]
    pub group: String,
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

fn default_group() -> String {
    "default".into()
}

fn default_asset_type() -> String {
    "host".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub target: Option<String>,
    pub details: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}
