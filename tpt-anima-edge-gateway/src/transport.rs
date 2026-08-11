//! HTTP transport for MCP and A2A payload delivery.

use crate::{A2aPayload, McpPayload, TranslationError};
use reqwest::Client;

/// Configuration for outbound HTTP transport.
pub struct HttpTransport {
    client: Client,
    mcp_endpoint: Option<String>,
    a2a_endpoint: Option<String>,
}

impl HttpTransport {
    pub fn new(mcp_endpoint: Option<String>, a2a_endpoint: Option<String>) -> Self {
        Self {
            client: Client::new(),
            mcp_endpoint,
            a2a_endpoint,
        }
    }

    /// Send an MCP payload to the configured MCP endpoint.
    pub async fn send_mcp(&self, payload: &McpPayload) -> Result<String, TranslationError> {
        let url = self
            .mcp_endpoint
            .as_ref()
            .ok_or_else(|| TranslationError::MissingField("mcp_endpoint".into()))?;

        let resp = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|e| TranslationError::InvalidPayload(format!("HTTP error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(TranslationError::InvalidPayload(format!(
                "MCP request failed with status {status}"
            )));
        }

        resp.text()
            .await
            .map_err(|e| TranslationError::InvalidPayload(format!("read body: {e}")))
    }

    /// Send an A2A payload to the configured A2A endpoint.
    pub async fn send_a2a(&self, payload: &A2aPayload) -> Result<String, TranslationError> {
        let url = self
            .a2a_endpoint
            .as_ref()
            .ok_or_else(|| TranslationError::MissingField("a2a_endpoint".into()))?;

        let resp = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|e| TranslationError::InvalidPayload(format!("HTTP error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(TranslationError::InvalidPayload(format!(
                "A2A request failed with status {status}"
            )));
        }

        resp.text()
            .await
            .map_err(|e| TranslationError::InvalidPayload(format!("read body: {e}")))
    }

    /// Receive and deserialize a JSON string from a raw body.
    pub fn receive<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, TranslationError> {
        serde_json::from_str(body)
            .map_err(|e| TranslationError::InvalidPayload(format!("deserialization: {e}")))
    }
}
