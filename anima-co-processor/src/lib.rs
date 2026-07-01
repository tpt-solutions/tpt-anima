//! Tier 2 Anima Co-Processor — eBPF kernel module (Linux) + Windows driver.
//!
//! This crate provides the co-processor layer that intercepts AI workloads on existing
//! operating systems through eBPF (Linux) or kernel drivers (Windows/macOS).
//!
//! # Features
//!
//! - **eBPF Interception**: Hook into Linux kernel to intercept AI workloads
//! - **Windows Driver**: Kernel-mode driver for Windows systems
//! - **Workload Translation**: Convert intercepted workloads to Anima intents
//! - **Cross-tier Communication**: Forward to Tier 1 hypervisor or Tier 3 edge runtime

#[cfg(target_os = "linux")]
pub mod bpf;

pub mod driver;

pub use driver::{
    CoProcessorConfig, CoProcessorError, WorkloadIntercept, WorkloadType,
};

/// Co-processor module for eBPF-based workload interception.
#[derive(Debug, Clone)]
pub struct CoProcessor {
    #[allow(dead_code)]
    config: CoProcessorConfig,
    intercepts: Vec<WorkloadIntercept>,
}

impl CoProcessor {
    /// Create a new co-processor module.
    pub fn new(config: CoProcessorConfig) -> Result<Self, CoProcessorError> {
        Ok(Self {
            config,
            intercepts: Vec::new(),
        })
    }

    /// Start intercepting workloads.
    pub fn start(&mut self) -> Result<(), CoProcessorError> {
        #[cfg(target_os = "linux")]
        {
            // On Linux, load and attach eBPF programs
            // This is a stub implementation
        }
        Ok(())
    }

    /// Stop intercepting workloads.
    pub fn stop(&mut self) {
        // Detach eBPF programs
    }

    /// Record an intercepted workload.
    pub fn record_intercept(&mut self, intercept: WorkloadIntercept) {
        self.intercepts.push(intercept);
    }

    /// Get recent intercepts.
    pub fn recent_intercepts(&self, n: usize) -> &[WorkloadIntercept] {
        let start = self.intercepts.len().saturating_sub(n);
        &self.intercepts[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co_processor_config_defaults() {
        let config = CoProcessorConfig::default();
        assert!(config.enabled);
        assert!(config.workload_patterns.contains(&"inference.run".to_string()));
    }

    #[test]
    fn workload_intercept_to_intent() {
        let intercept = WorkloadIntercept::new(
            WorkloadType::Inference,
            1234,
            "test-model".into(),
            serde_json::json!({"model_id": "gpt-4"}),
        );
        
        let intent = intercept.to_intent().unwrap();
        assert_eq!(intent["action"], "inference.run");
        assert_eq!(intent["process_id"], 1234);
    }

    #[test]
    fn workload_intercept_unknown_returns_none() {
        let intercept = WorkloadIntercept::new(
            WorkloadType::Unknown,
            0,
            "".into(),
            serde_json::json!({}),
        );
        
        assert!(intercept.to_intent().is_none());
    }

    #[test]
    fn co_processor_creates() {
        let config = CoProcessorConfig::default();
        let co_proc = CoProcessor::new(config).unwrap();
        assert!(co_proc.recent_intercepts(10).is_empty());
    }
}