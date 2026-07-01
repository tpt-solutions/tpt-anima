//! Driver interfaces for Windows and macOS co-processor support.
//!
//! This module provides stub implementations for Windows kernel driver and macOS
//! kernel extension support. The actual kernel driver code is platform-specific.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Configuration for the co-processor module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoProcessorConfig {
    /// Unique identifier for this co-processor instance.
    pub id: String,
    /// Whether interception is enabled.
    pub enabled: bool,
    /// Maximum memory buffer size in bytes.
    pub max_buffer_bytes: u64,
    /// Intercepted workload patterns to watch for.
    pub workload_patterns: Vec<String>,
}

impl Default for CoProcessorConfig {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            enabled: true,
            max_buffer_bytes: 64 * 1024 * 1024, // 64 MB
            workload_patterns: vec![
                "model.load".to_string(),
                "tensor.compute".to_string(),
                "inference.run".to_string(),
            ],
        }
    }
}

/// Errors that can occur in the co-processor.
#[derive(Debug, Error)]
pub enum CoProcessorError {
    #[error("co-processor not initialized")]
    NotInitialized,

    #[error("interception failed: {0}")]
    InterceptionFailed(String),

    #[error("workload translation failed: {0}")]
    TranslationFailed(String),

    #[error("cross-tier communication error: {0}")]
    CrossTierError(String),

    #[error("driver initialization failed: {0}")]
    DriverInitFailed(String),
}

/// Types of workloads that can be intercepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Model loading operation.
    ModelLoad,
    /// Tensor computation.
    TensorCompute,
    /// Inference execution.
    Inference,
    /// Memory allocation.
    MemoryAlloc,
    /// Unknown/custom workload.
    Unknown,
}

/// An intercepted workload that can be translated to an Anima intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadIntercept {
    /// Unique identifier for the intercept.
    pub id: String,
    /// Type of workload intercepted.
    pub workload_type: WorkloadType,
    /// Process ID that triggered the intercept.
    pub process_id: u32,
    /// Process name.
    pub process_name: String,
    /// Payload data captured from the intercept.
    pub payload: serde_json::Value,
    /// Timestamp when intercept occurred.
    pub timestamp: u64,
}

impl WorkloadIntercept {
    /// Create a new workload intercept.
    pub fn new(workload_type: WorkloadType, process_id: u32, process_name: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workload_type,
            process_id,
            process_name,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Translate this intercept to an Anima intent.
    pub fn to_intent(&self) -> Option<serde_json::Value> {
        let action = match self.workload_type {
            WorkloadType::ModelLoad => "model.load",
            WorkloadType::TensorCompute => "tensor.compute",
            WorkloadType::Inference => "inference.run",
            WorkloadType::MemoryAlloc => "memory.allocate",
            WorkloadType::Unknown => return None,
        };

        Some(serde_json::json!({
            "action": action,
            "source_process": self.process_name,
            "process_id": self.process_id,
            "payload": self.payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        let config = CoProcessorConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn intercept_to_intent() {
        let intercept = WorkloadIntercept::new(
            WorkloadType::ModelLoad,
            100,
            "test-process".into(),
            serde_json::json!({}),
        );
        let intent = intercept.to_intent().unwrap();
        assert_eq!(intent["action"], "model.load");
    }
}