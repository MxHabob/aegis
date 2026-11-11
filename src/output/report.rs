use crate::core::models::{AppState, Finding};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct Report<'a> {
    generated_at: u64,
    scope_count: usize,
    asset_count: usize,
    scopes: &'a [crate::core::models::Scope],
    assets: &'a [crate::core::models::Asset],
    findings: &'a [Finding],
}

pub fn render_json_report(state: &AppState, findings: &[Finding]) -> Result<String> {
    let report = Report {
        generated_at: now()?,
        scope_count: state.scopes.len(),
        asset_count: state.assets.len(),
        scopes: &state.scopes,
        assets: &state.assets,
        findings,
    };
    Ok(serde_json::to_string_pretty(&report)?)
}

pub fn render_text_report(state: &AppState, findings: &[Finding]) -> String {
    let mut output = format!(
        "Asset inventory report\nAuthorized scopes: {}\nAssets: {}\n",
        state.scopes.len(),
        state.assets.len()
    );
    if state.assets.is_empty() {
        output.push_str("\nInventory is empty.\n");
    } else {
        output.push_str("\nAssets:\n");
        for asset in &state.assets {
            output.push_str(&format!(
                "  [{}] {} — {}\n",
                asset.group, asset.host, asset.label
            ));
        }
    }
    output.push_str(&format!(
        "\nLocal consistency findings: {}\n",
        findings.len()
    ));
    for finding in findings {
        output.push_str(&format!(
            "  [{}] {}{}\n    {}\n    Recommendation: {}\n",
            finding.severity.as_str(),
            finding.title,
            finding
                .target
                .as_ref()
                .map(|target| format!(" ({target})"))
                .unwrap_or_default(),
            finding.details,
            finding.recommendation
        ));
    }
    output
}

fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}
