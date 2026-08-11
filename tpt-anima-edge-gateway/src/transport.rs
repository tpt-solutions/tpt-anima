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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receive_roundtrips_mcp_payload() {
        let payload = McpPayload {
            method: "analyze".into(),
            params: serde_json::json!({ "text": "hi" }),
            id: "mcp-1".into(),
        };
        let body = serde_json::to_string(&payload).unwrap();
        let back: McpPayload = HttpTransport::receive(&body).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn receive_errors_on_invalid_json() {
        let err = HttpTransport::receive::<McpPayload>("not json").unwrap_err();
        assert!(matches!(err, TranslationError::InvalidPayload(_)));
    }

    #[test]
    fn receive_errors_on_type_mismatch() {
        // A well-formed JSON object that is not an McpPayload.
        let err = HttpTransport::receive::<McpPayload>("{\"foo\": 1}").unwrap_err();
        assert!(matches!(err, TranslationError::InvalidPayload(_)));
    }

    #[tokio::test]
    async fn send_mcp_requires_endpoint() {
        let t = HttpTransport::new(None, Some("https://a2a.example".into()));
        let err = t
            .send_mcp(&McpPayload {
                method: "x".into(),
                params: serde_json::json!({}),
                id: "1".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(err, TranslationError::MissingField("mcp_endpoint".into()));
    }

    #[tokio::test]
    async fn send_a2a_requires_endpoint() {
        let t = HttpTransport::new(Some("https://mcp.example".into()), None);
        let err = t
            .send_a2a(&A2aPayload {
                task_id: "t".into(),
                message: "m".into(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap_err();
        assert_eq!(err, TranslationError::MissingField("a2a_endpoint".into()));
    }
}
