use crate::core::models::{Finding, Severity};
use std::{collections::BTreeMap, fs, path::Path};

pub fn audit_sshd_config(path: &Path) -> anyhow::Result<Vec<Finding>> {
    let content = fs::read_to_string(path)?;
    Ok(audit_sshd_config_text(
        &content,
        &path.display().to_string(),
    ))
}

pub fn audit_sshd_config_text(content: &str, source: &str) -> Vec<Finding> {
    let mut settings = BTreeMap::<String, String>::new();
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };
        settings
            .entry(key.to_ascii_lowercase())
            .or_insert_with(|| value.to_ascii_lowercase());
    }

    let mut findings = Vec::new();
    let checks = [
        (
            "permitrootlogin",
            "yes",
            Severity::High,
            "Root SSH login is enabled",
            "Set PermitRootLogin no, or use prohibit-password if operationally required.",
        ),
        (
            "passwordauthentication",
            "yes",
            Severity::Medium,
            "Password-based SSH authentication is enabled",
            "Prefer key-based authentication and disable PasswordAuthentication where feasible.",
        ),
        (
            "permitemptypasswords",
            "yes",
            Severity::High,
            "Empty-password SSH accounts are permitted",
            "Set PermitEmptyPasswords no.",
        ),
        (
            "pubkeyauthentication",
            "no",
            Severity::Medium,
            "Public-key SSH authentication is disabled",
            "Enable PubkeyAuthentication and provision managed keys.",
        ),
        (
            "protocol",
            "1",
            Severity::High,
            "Legacy SSH protocol 1 is configured",
            "Use a maintained OpenSSH release that supports SSH protocol 2 only.",
        ),
    ];

    for (key, risky_value, severity, title, recommendation) in checks {
        if settings.get(key).is_some_and(|value| value == risky_value) {
            findings.push(Finding {
                id: format!("ssh-config.{key}"),
                severity,
                title: title.into(),
                target: Some(source.into()),
                details: format!("{key} is configured as {risky_value}."),
                recommendation: recommendation.into(),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_risky_ssh_settings_without_network_access() {
        let findings = audit_sshd_config_text(
            "PermitRootLogin yes\nPasswordAuthentication yes\nPubkeyAuthentication yes\n",
            "fixture",
        );
        assert_eq!(findings.len(), 2);
        assert!(
            findings
                .iter()
                .any(|finding| finding.id == "ssh-config.permitrootlogin")
        );
    }

    #[test]
    fn ignores_comments_and_keeps_first_match() {
        let findings = audit_sshd_config_text(
            "# PermitRootLogin yes\nPermitRootLogin no\nPermitRootLogin yes\n",
            "fixture",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn reads_utf8_bom_configuration_files() {
        let findings = audit_sshd_config_text("\u{feff}PermitRootLogin yes\n", "fixture");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "ssh-config.permitrootlogin");
    }
}
