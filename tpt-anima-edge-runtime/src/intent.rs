use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EdgeError {
    #[error("intent handler not registered for action: {0}")]
    UnhandledAction(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeIntent {
    pub id: String,
    pub action: String,
    pub payload: serde_json::Value,
}

impl EdgeIntent {
    pub fn new(action: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            action,
            payload,
        }
    }
}

/// Type alias for intent handlers.
pub type IntentHandler =
    Box<dyn Fn(&EdgeIntent) -> Result<serde_json::Value, EdgeError> + Send + Sync>;

/// Registers and dispatches lightweight intents on edge devices.
pub struct EdgeIntentHandler {
    handlers: std::collections::HashMap<String, IntentHandler>,
}

impl EdgeIntentHandler {
    pub fn new() -> Self {
        Self {
            handlers: std::collections::HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        action: &str,
        handler: impl Fn(&EdgeIntent) -> Result<serde_json::Value, EdgeError> + Send + Sync + 'static,
    ) {
        self.handlers.insert(action.to_string(), Box::new(handler));
    }

    pub fn handle(&self, intent: &EdgeIntent) -> Result<serde_json::Value, EdgeError> {
        self.handlers
            .get(&intent.action)
            .ok_or_else(|| EdgeError::UnhandledAction(intent.action.clone()))?(intent)
    }

    pub fn has_handler(&self, action: &str) -> bool {
        self.handlers.contains_key(action)
    }

    pub fn count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for EdgeIntentHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_intent_has_unique_id() {
        let a = EdgeIntent::new("ping".into(), serde_json::json!({}));
        let b = EdgeIntent::new("ping".into(), serde_json::json!({}));
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn register_and_handle() {
        let mut handler = EdgeIntentHandler::new();
        handler.register("echo", |intent| Ok(intent.payload.clone()));
        let intent = EdgeIntent::new("echo".into(), serde_json::json!({"msg": "hi"}));
        let result = handler.handle(&intent).unwrap();
        assert_eq!(result, serde_json::json!({"msg": "hi"}));
    }

    #[test]
    fn unregistered_action_returns_error() {
        let handler = EdgeIntentHandler::new();
        let intent = EdgeIntent::new("unknown".into(), serde_json::json!({}));
        assert!(matches!(
            handler.handle(&intent),
            Err(EdgeError::UnhandledAction(_))
        ));
    }

    #[test]
    fn has_handler() {
        let mut handler = EdgeIntentHandler::new();
        assert!(!handler.has_handler("ping"));
        handler.register("ping", |_| Ok(serde_json::json!({})));
        assert!(handler.has_handler("ping"));
    }

    #[test]
    fn multiple_handlers() {
        let mut handler = EdgeIntentHandler::new();
        handler.register("a", |_| Ok(serde_json::json!(1)));
        handler.register("b", |_| Ok(serde_json::json!(2)));
        let ia = EdgeIntent::new("a".into(), serde_json::json!({}));
        let ib = EdgeIntent::new("b".into(), serde_json::json!({}));
        assert_eq!(handler.handle(&ia).unwrap(), 1);
        assert_eq!(handler.handle(&ib).unwrap(), 2);
    }
}
