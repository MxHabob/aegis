use crate::{
    core::models::{AppState, Finding, Severity},
    scope::{is_within_scope, normalize_host},
};
use anyhow::Result;
use std::collections::HashSet;

pub struct InventoryIntegrity;

impl super::CheckModule for InventoryIntegrity {
    fn name(&self) -> &'static str {
        "inventory-integrity"
    }

    fn description(&self) -> &'static str {
        "Checks local inventory consistency without contacting assets."
    }

    fn run(&self, state: &AppState) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let mut seen = HashSet::new();

        for asset in &state.assets {
            if !["host", "certificate", "web_property", "camera"]
                .contains(&asset.asset_type.as_str())
            {
                findings.push(Finding {
                    id: "inventory.invalid-asset-type".into(),
                    severity: Severity::Medium,
                    title: "Unknown asset type".into(),
                    target: Some(asset.host.clone()),
                    details: format!("The asset type '{}' is not supported.", asset.asset_type),
                    recommendation: "Use host, certificate, web_property, or camera.".into(),
                });
            }
            if normalize_host(&asset.host).is_err() {
                findings.push(Finding {
                    id: "inventory.invalid-host".into(),
                    severity: Severity::High,
                    title: "Invalid host in local inventory".into(),
                    target: Some(asset.host.clone()),
                    details: "The stored asset host is not a valid bare domain or IP address."
                        .into(),
                    recommendation: "Review and correct the local inventory entry.".into(),
                });
            }
            if asset.label.trim().is_empty() {
                findings.push(Finding {
                    id: "inventory.empty-label".into(),
                    severity: Severity::Low,
                    title: "Asset label is empty".into(),
                    target: Some(asset.host.clone()),
                    details: "This stored inventory record has no descriptive label.".into(),
                    recommendation: "Add a short, non-sensitive label identifying the asset."
                        .into(),
                });
            }
            let unique_key = format!("{}\0{}", asset.group, asset.host);
            if !seen.insert(unique_key) {
                findings.push(Finding {
                    id: "inventory.duplicate-host".into(),
                    severity: Severity::Medium,
                    title: "Duplicate asset record".into(),
                    target: Some(asset.host.clone()),
                    details: "The same host appears more than once in the same inventory group."
                        .into(),
                    recommendation: "Remove duplicate records and retain one canonical entry."
                        .into(),
                });
            }
            if !state.scopes.iter().any(|scope| {
                scope.group == asset.group && is_within_scope(&asset.host, &scope.target)
            }) {
                findings.push(Finding {
                    id: "scope.asset-outside-scope".into(),
                    severity: Severity::High,
                    title: "Inventory asset is outside configured scope".into(),
                    target: Some(asset.host.clone()),
                    details:
                        "The local inventory contains an asset not covered by an authorized scope."
                            .into(),
                    recommendation:
                        "Verify authorization, then add the correct scope or remove the asset."
                            .into(),
                });
            }
        }

        Ok(findings)
    }
}
