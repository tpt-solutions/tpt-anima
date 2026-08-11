//! Translation layer between native INP intents and external MCP/A2A JSON formats.

use serde::{Deserialize, Serialize};
use std::fmt;

pub mod transport;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InpIntent {
    pub id: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub max_cost: Option<f64>,
    pub max_latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpPayload {
    pub method: String,
    pub params: serde_json::Value,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2aPayload {
    pub task_id: String,
    pub message: String,
    pub metadata: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TranslationError {
    InvalidPayload(String),
    MissingField(String),
    UnsupportedFormat(String),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPayload(msg) => write!(f, "invalid payload: {msg}"),
            Self::MissingField(field) => write!(f, "missing field: {field}"),
            Self::UnsupportedFormat(msg) => write!(f, "unsupported format: {msg}"),
        }
    }
}

impl std::error::Error for TranslationError {}

// ---------------------------------------------------------------------------
// INP → MCP
// ---------------------------------------------------------------------------

pub struct InpToMcpTranslator;

impl InpToMcpTranslator {
    pub fn translate(intent: &InpIntent) -> McpPayload {
        McpPayload {
            method: intent.action.clone(),
            params: intent.payload.clone(),
            id: intent.id.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP → INP
// ---------------------------------------------------------------------------

pub struct McpToInpTranslator;

impl McpToInpTranslator {
    pub fn translate(payload: &McpPayload) -> Result<InpIntent, TranslationError> {
        if payload.method.is_empty() {
            return Err(TranslationError::MissingField("method".into()));
        }
        if payload.id.is_empty() {
            return Err(TranslationError::MissingField("id".into()));
        }
        Ok(InpIntent {
            id: payload.id.clone(),
            action: payload.method.clone(),
            payload: payload.params.clone(),
            max_cost: None,
            max_latency_ms: None,
        })
    }
}

// ---------------------------------------------------------------------------
// INP → A2A
// ---------------------------------------------------------------------------

pub struct InpToA2aTranslator;

impl InpToA2aTranslator {
    pub fn translate(intent: &InpIntent) -> A2aPayload {
        let message = intent
            .payload
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or(&intent.action);
        A2aPayload {
            task_id: intent.id.clone(),
            message: message.to_string(),
            metadata: serde_json::json!({
                "action": intent.action,
                "max_cost": intent.max_cost,
                "max_latency_ms": intent.max_latency_ms,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// A2A → INP
// ---------------------------------------------------------------------------

pub struct A2aToInpTranslator;

impl A2aToInpTranslator {
    pub fn translate(payload: &A2aPayload) -> Result<InpIntent, TranslationError> {
        if payload.task_id.is_empty() {
            return Err(TranslationError::MissingField("task_id".into()));
        }
        if payload.message.is_empty() {
            return Err(TranslationError::MissingField("message".into()));
        }

        let action = payload
            .metadata
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let max_cost = payload.metadata.get("max_cost").and_then(|v| v.as_f64());
        let max_latency_ms = payload
            .metadata
            .get("max_latency_ms")
            .and_then(|v| v.as_u64());

        Ok(InpIntent {
            id: payload.task_id.clone(),
            action,
            payload: serde_json::json!({ "message": payload.message }),
            max_cost,
            max_latency_ms,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_intent() -> InpIntent {
        InpIntent {
            id: "intent-1".into(),
            action: "analyze".into(),
            payload: serde_json::json!({ "message": "hello", "input": "hello" }),
            max_cost: Some(0.5),
            max_latency_ms: Some(1000),
        }
    }

    #[test]
    fn inp_to_mcp_roundtrip_fields() {
        let intent = sample_intent();
        let mcp = InpToMcpTranslator::translate(&intent);
        assert_eq!(mcp.id, "intent-1");
        assert_eq!(mcp.method, "analyze");
        assert_eq!(mcp.params["input"], "hello");
    }

    #[test]
    fn mcp_to_inp_roundtrip_fields() {
        let mcp = McpPayload {
            method: "summarize".into(),
            params: serde_json::json!({ "text": "world" }),
            id: "mcp-42".into(),
        };
        let intent = McpToInpTranslator::translate(&mcp).unwrap();
        assert_eq!(intent.id, "mcp-42");
        assert_eq!(intent.action, "summarize");
        assert_eq!(intent.payload["text"], "world");
        assert!(intent.max_cost.is_none());
    }

    #[test]
    fn mcp_to_inp_missing_method() {
        let mcp = McpPayload {
            method: String::new(),
            params: serde_json::json!({}),
            id: "x".into(),
        };
        let err = McpToInpTranslator::translate(&mcp).unwrap_err();
        assert_eq!(err, TranslationError::MissingField("method".into()));
    }

    #[test]
    fn mcp_to_inp_missing_id() {
        let mcp = McpPayload {
            method: "do".into(),
            params: serde_json::json!({}),
            id: String::new(),
        };
        let err = McpToInpTranslator::translate(&mcp).unwrap_err();
        assert_eq!(err, TranslationError::MissingField("id".into()));
    }

    #[test]
    fn inp_to_a2a_includes_metadata() {
        let intent = sample_intent();
        let a2a = InpToA2aTranslator::translate(&intent);
        assert_eq!(a2a.task_id, "intent-1");
        assert_eq!(a2a.message, "hello");
        assert_eq!(a2a.metadata["action"], "analyze");
        assert_eq!(a2a.metadata["max_cost"], 0.5);
        assert_eq!(a2a.metadata["max_latency_ms"], 1000);
    }

    #[test]
    fn inp_to_a2a_fallback_action_as_message() {
        let intent = InpIntent {
            id: "i2".into(),
            action: "ping".into(),
            payload: serde_json::json!({ "other": 1 }),
            max_cost: None,
            max_latency_ms: None,
        };
        let a2a = InpToA2aTranslator::translate(&intent);
        assert_eq!(a2a.message, "ping");
    }

    #[test]
    fn a2a_to_inp_roundtrip() {
        let a2a = A2aPayload {
            task_id: "t-10".into(),
            message: "compute".into(),
            metadata: serde_json::json!({
                "action": "compute",
                "max_cost": 1.5,
                "max_latency_ms": 2000
            }),
        };
        let intent = A2aToInpTranslator::translate(&a2a).unwrap();
        assert_eq!(intent.id, "t-10");
        assert_eq!(intent.action, "compute");
        assert_eq!(intent.payload["message"], "compute");
        assert_eq!(intent.max_cost, Some(1.5));
        assert_eq!(intent.max_latency_ms, Some(2000));
    }

    #[test]
    fn a2a_to_inp_missing_task_id() {
        let a2a = A2aPayload {
            task_id: String::new(),
            message: "hi".into(),
            metadata: serde_json::json!({}),
        };
        assert_eq!(
            A2aToInpTranslator::translate(&a2a).unwrap_err(),
            TranslationError::MissingField("task_id".into())
        );
    }

    #[test]
    fn a2a_to_inp_missing_message() {
        let a2a = A2aPayload {
            task_id: "t1".into(),
            message: String::new(),
            metadata: serde_json::json!({}),
        };
        assert_eq!(
            A2aToInpTranslator::translate(&a2a).unwrap_err(),
            TranslationError::MissingField("message".into())
        );
    }

    #[test]
    fn a2a_to_inp_defaults_action_to_unknown() {
        let a2a = A2aPayload {
            task_id: "t2".into(),
            message: "hello".into(),
            metadata: serde_json::json!({}),
        };
        let intent = A2aToInpTranslator::translate(&a2a).unwrap();
        assert_eq!(intent.action, "unknown");
    }

    #[test]
    fn translation_error_display() {
        let e1 = TranslationError::InvalidPayload("bad".into());
        assert!(format!("{e1}").contains("invalid payload"));
        let e2 = TranslationError::UnsupportedFormat("csv".into());
        assert!(format!("{e2}").contains("unsupported format"));
    }
}
