use serde::{Deserialize, Serialize};

use crate::intent::{EdgeError, EdgeIntent, EdgeIntentHandler};
use crate::memory::EdgeMemory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub memory_entries: usize,
    pub memory_bytes: usize,
    pub handler_count: usize,
    pub total_processed: u64,
}

/// Lightweight runtime that coordinates SSE memory and INP intent handling
/// for edge devices (phones, IoT, browsers).
pub struct EdgeRuntime {
    memory: EdgeMemory,
    handler: EdgeIntentHandler,
    total_processed: u64,
}

impl EdgeRuntime {
    pub fn new() -> Self {
        Self {
            memory: EdgeMemory::new(),
            handler: EdgeIntentHandler::new(),
            total_processed: 0,
        }
    }

    /// Process an intent through the intent handler.
    pub fn process(&mut self, intent: &EdgeIntent) -> Result<serde_json::Value, EdgeError> {
        self.total_processed += 1;
        self.handler.handle(intent)
    }

    /// Access the underlying memory store.
    pub fn memory(&mut self) -> &mut EdgeMemory {
        &mut self.memory
    }

    /// Register an intent handler.
    pub fn register_handler(
        &mut self,
        action: &str,
        handler: impl Fn(&EdgeIntent) -> Result<serde_json::Value, EdgeError> + Send + Sync + 'static,
    ) {
        self.handler.register(action, handler);
    }

    /// Emit a heartbeat: decay memory and return current stats.
    pub fn heartbeat(&mut self, decay_factor: f32) -> RuntimeStats {
        self.memory.decay(decay_factor);
        self.stats()
    }

    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            memory_entries: self.memory.count(),
            memory_bytes: self.memory.total_bytes(),
            handler_count: self.handler.count(),
            total_processed: self.total_processed,
        }
    }
}

impl Default for EdgeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::EdgeIntent;

    #[test]
    fn new_runtime_is_empty() {
        let rt = EdgeRuntime::new();
        let stats = rt.stats();
        assert_eq!(stats.memory_entries, 0);
        assert_eq!(stats.total_processed, 0);
    }

    #[test]
    fn process_increments_counter() {
        let mut rt = EdgeRuntime::new();
        rt.register_handler("ping", |_| Ok(serde_json::json!("pong")));
        let intent = EdgeIntent::new("ping".into(), serde_json::json!({}));
        rt.process(&intent).unwrap();
        assert_eq!(rt.stats().total_processed, 1);
    }

    #[test]
    fn process_unregistered_returns_error() {
        let mut rt = EdgeRuntime::new();
        let intent = EdgeIntent::new("unknown".into(), serde_json::json!({}));
        assert!(rt.process(&intent).is_err());
    }

    #[test]
    fn heartbeat_applies_decay() {
        let mut rt = EdgeRuntime::new();
        rt.memory().store(vec![0; 100], vec!["test".into()]);
        assert_eq!(rt.stats().memory_bytes, 100);
        let stats = rt.heartbeat(0.5);
        assert_eq!(stats.memory_entries, 1);
    }

    #[test]
    fn memory_store_and_stats() {
        let mut rt = EdgeRuntime::new();
        rt.memory().store(vec![0; 42], vec!["a".into()]);
        rt.memory().store(vec![0; 58], vec!["b".into()]);
        let stats = rt.stats();
        assert_eq!(stats.memory_entries, 2);
        assert_eq!(stats.memory_bytes, 100);
    }

    #[test]
    fn register_and_process_chain() {
        let mut rt = EdgeRuntime::new();
        rt.register_handler("double", |intent| {
            let n = intent.payload.as_i64().unwrap_or(0);
            Ok(serde_json::json!(n * 2))
        });
        let intent = EdgeIntent::new("double".into(), serde_json::json!(21));
        let result = rt.process(&intent).unwrap();
        assert_eq!(result, 42);
    }
}
