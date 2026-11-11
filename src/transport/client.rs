use crate::connectors::ImportedAsset;
use reqwest::{Client, Url, header::CONTENT_TYPE, redirect::Policy};
use std::time::Duration;
use thiserror::Error;

const MAX_INVENTORY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct TransportConfig {
    pub timeout: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(20),
        }
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("inventory API URL must use HTTPS")]
    HttpsRequired,
    #[error("inventory API URL must not contain embedded credentials")]
    EmbeddedCredentials,
    #[error("inventory API response was not successful: HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("inventory API response is not JSON")]
    InvalidContentType,
    #[error("inventory API response exceeds the {MAX_INVENTORY_BYTES}-byte limit")]
    ResponseTooLarge,
    #[error("inventory API request failed")]
    Request(#[source] reqwest::Error),
    #[error("invalid inventory JSON")]
    InvalidJson(#[source] serde_json::Error),
}

#[derive(Clone)]
pub struct NetworkTransport {
    client: Client,
}

impl NetworkTransport {
    pub fn new(config: TransportConfig) -> Result<Self, TransportError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .redirect(Policy::none())
            .https_only(true)
            .build()
            .map_err(TransportError::Request)?;
        Ok(Self { client })
    }

    pub async fn fetch_inventory(&self, url: &Url) -> Result<Vec<ImportedAsset>, TransportError> {
        validate_inventory_url(url)?;
        let mut response = self
            .client
            .get(url.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| TransportError::Request(error.without_url()))?;

        if !response.status().is_success() {
            return Err(TransportError::HttpStatus(response.status()));
        }

        let is_json = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| {
                let media_type = value.split(';').next().unwrap_or("").trim();
                media_type.eq_ignore_ascii_case("application/json")
                    || media_type.to_ascii_lowercase().ends_with("+json")
            });
        if !is_json {
            return Err(TransportError::InvalidContentType);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_INVENTORY_BYTES as u64)
        {
            return Err(TransportError::ResponseTooLarge);
        }

        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| TransportError::Request(error.without_url()))?
        {
            if body.len().saturating_add(chunk.len()) > MAX_INVENTORY_BYTES {
                return Err(TransportError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }

        serde_json::from_slice(&body).map_err(TransportError::InvalidJson)
    }
}

pub fn validate_inventory_url(url: &Url) -> Result<(), TransportError> {
    if url.scheme() != "https" {
        return Err(TransportError::HttpsRequired);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(TransportError::EmbeddedCredentials);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_plain_http_and_embedded_credentials() {
        let http = Url::parse("http://inventory.example.test/assets").unwrap();
        assert!(matches!(
            validate_inventory_url(&http),
            Err(TransportError::HttpsRequired)
        ));

        let credentials = Url::parse("https://user:secret@inventory.example.test/assets").unwrap();
        assert!(matches!(
            validate_inventory_url(&credentials),
            Err(TransportError::EmbeddedCredentials)
        ));
    }

    #[test]
    fn accepts_https_without_credentials() {
        let url = Url::parse("https://inventory.example.test/v1/assets").unwrap();
        assert!(validate_inventory_url(&url).is_ok());
    }

    #[test]
    fn client_is_shareable_for_concurrent_requests_and_accepts_timeout() {
        let transport = NetworkTransport::new(TransportConfig {
            timeout: Duration::from_millis(1),
        })
        .unwrap();
        let _other_handle = transport.clone();
    }
}
