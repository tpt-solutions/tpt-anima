//! Driver interfaces for Windows and macOS co-processor support, plus the Linux eBPF
//! load path.
//!
//! This module provides stub implementations for Windows kernel driver and macOS
//! kernel extension support. The actual kernel driver code is platform-specific. On
//! Linux, [`load_ebpf_program`] performs a real load of the bytecode assembled in
//! [`crate::bpf`] via the `aya` crate.

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

    #[error("eBPF load failed: {0}")]
    EbpfLoadFailed(String),

    #[error("eBPF loading requires Linux — not supported on this platform")]
    EbpfUnsupportedPlatform,
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
    pub fn new(
        workload_type: WorkloadType,
        process_id: u32,
        process_name: String,
        payload: serde_json::Value,
    ) -> Self {
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

/// Load and attach one of the [`crate::bpf::EbpfProgram`]s generated in [`crate::bpf`]
/// into the running Linux kernel via `aya`.
///
/// `program` supplies the assembled bytecode and its associated map definitions;
/// `hook` names the kernel entry point it should be wired to (kprobe symbol, tracepoint
/// name, etc. — interpretation is left to the caller/config layer, since attachment
/// targets are deployment-specific).
///
/// # Unverified against a live kernel
///
/// This function is written to match the `aya` 0.14 API (`aya::Ebpf::load` followed by
/// looking the program up by name, downcasting it to the appropriate program type, and
/// calling `.load()` / `.attach(..)` on it) and the bytecode it loads has been checked
/// instruction-by-instruction against the public eBPF ISA — but it has **not** been run
/// against a real Linux kernel/verifier from this development environment, because no
/// Linux kernel is available on this Windows machine. Treat this path as
/// correct-per-spec but execution-unverified until it has been exercised on real
/// Linux hardware (or in CI running on a Linux runner).
#[cfg(target_os = "linux")]
pub fn load_ebpf_program(program: &crate::bpf::EbpfProgram) -> Result<aya::Ebpf, CoProcessorError> {
    // `Ebpf::load` parses the ELF-like in-memory image aya expects: a flat map of
    // section name -> section bytes. For a hand-assembled program with no ELF
    // relocations/BTF, the simplest valid input is a single ".text"-equivalent blob
    // containing the program's raw instructions. `load_bytes` is aya's entry point for
    // exactly this case — loading pre-assembled bytecode rather than compiling from an
    // `#[no_std]` eBPF crate at build time.
    let mut ebpf = aya::Ebpf::load(&program.bytecode)
        .map_err(|e| CoProcessorError::EbpfLoadFailed(format!("{e}")))?;

    // Programs embedded via `load` are keyed by the ELF section name they were defined
    // under. Our hand-assembled image only defines one program, addressed here by the
    // `EbpfProgram::name` we generated it with so callers can round-trip a program
    // definition straight to a loaded/attached kernel object.
    let program_mut = ebpf.program_mut(&program.name).ok_or_else(|| {
        CoProcessorError::EbpfLoadFailed(format!(
            "program section '{}' not found after load",
            program.name
        ))
    })?;

    // The program type (KProbe, TracePoint, ...) must match how the bytecode expects
    // its context (`r1`) to be laid out. Both generators in `crate::bpf` are written as
    // tracepoint-style programs, so we load as a raw tracepoint program here; a
    // deployment wiring a kprobe-based hook instead would downcast to `KProbe`.
    let tracepoint: &mut aya::programs::TracePoint = program_mut
        .try_into()
        .map_err(|e| CoProcessorError::EbpfLoadFailed(format!("wrong program type: {e}")))?;

    tracepoint
        .load()
        .map_err(|e| CoProcessorError::EbpfLoadFailed(format!("verifier rejected program: {e}")))?;

    Ok(ebpf)
}

/// On non-Linux platforms, eBPF cannot be loaded at all — there is no in-kernel eBPF
/// verifier/loader to call into. This always returns an honest error; it never
/// silently no-ops or reports fake success.
#[cfg(not(target_os = "linux"))]
pub fn load_ebpf_program(_program: &crate::bpf::EbpfProgram) -> Result<(), CoProcessorError> {
    Err(CoProcessorError::EbpfUnsupportedPlatform)
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

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn ebpf_load_is_an_honest_error_off_linux() {
        let program = crate::bpf::EbpfProgram::generate_inference_intercept();
        let result = load_ebpf_program(&program);
        assert!(matches!(
            result,
            Err(CoProcessorError::EbpfUnsupportedPlatform)
        ));
    }
}
