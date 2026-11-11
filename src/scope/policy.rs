use anyhow::{Result, bail};
use std::net::IpAddr;

pub fn normalize_host(input: &str) -> Result<String> {
    let host = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.parse::<IpAddr>().is_ok() {
        return Ok(host);
    }
    if host.is_empty() || host.contains(['/', '\\', '@', ':']) {
        bail!("enter a bare domain name or IP address (no URL, port, or path)");
    }
    if host.len() > 253 || host.starts_with("*.") {
        bail!("invalid domain name");
    }
    let valid = host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    });
    if !valid || !host.contains('.') {
        bail!("invalid domain name; use a fully qualified domain or IP address");
    }
    Ok(host)
}

pub fn normalize_group(input: &str) -> Result<String> {
    let group = input.trim().to_ascii_lowercase();
    let valid = !group.is_empty()
        && group.len() <= 64
        && group.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
        })
        && !group.starts_with(['-', '_', '.'])
        && !group.ends_with(['-', '_', '.']);
    if !valid {
        bail!("group names must be 1-64 characters using letters, numbers, '.', '_' or '-'");
    }
    Ok(group)
}

pub fn is_within_scope(host: &str, scope: &str) -> bool {
    if scope.parse::<IpAddr>().is_ok() {
        return host == scope;
    }
    host == scope
        || host
            .strip_suffix(scope)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_scope_matches_only_domain_boundary() {
        assert!(is_within_scope("api.example.com", "example.com"));
        assert!(is_within_scope("example.com", "example.com"));
        assert!(!is_within_scope("notexample.com", "example.com"));
    }

    #[test]
    fn host_parser_rejects_urls_and_wildcards() {
        assert!(normalize_host("https://example.com").is_err());
        assert!(normalize_host("*.example.com").is_err());
        assert_eq!(normalize_host("EXAMPLE.com.").unwrap(), "example.com");
    }

    #[test]
    fn group_name_is_normalized_and_validated() {
        assert_eq!(normalize_group("Production").unwrap(), "production");
        assert!(normalize_group("../outside").is_err());
    }
}
