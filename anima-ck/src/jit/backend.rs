//! Backend trait and platform-specific implementations.

use std::fmt;

use super::detection::HardwarePlatform;

/// Errors produced during JIT compilation or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JitError {
    CompilationFailed(String),
    ExecutionFailed(String),
    BackendUnavailable(String),
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompilationFailed(msg) => write!(f, "compilation failed: {msg}"),
            Self::ExecutionFailed(msg) => write!(f, "execution failed: {msg}"),
            Self::BackendUnavailable(name) => write!(f, "backend unavailable: {name}"),
        }
    }
}

impl std::error::Error for JitError {}

/// Trait that all hardware backends implement.
pub trait Backend: Send + Sync {
    /// Human-readable backend name.
    fn name(&self) -> &str;

    /// Compile an inference description into executable code (bytes).
    fn compile(&self, inference: &str) -> Result<Vec<u8>, JitError>;

    /// Execute previously compiled code and return the result.
    fn execute(&self, code: &[u8]) -> Result<String, JitError>;

    /// Whether this backend is available on the current system.
    fn is_available(&self) -> bool;

    /// The hardware platform this backend targets.
    fn platform(&self) -> HardwarePlatform;
}

// ── NVIDIA CUDA ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CudaBackend;

impl Backend for CudaBackend {
    fn name(&self) -> &str {
        "cuda"
    }

    fn compile(&self, inference: &str) -> Result<Vec<u8>, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("cuda".into()));
        }
        // Stub: emit placeholder PTX bytes
        Ok(format!("// compiled cuda kernel for: {inference}").into_bytes())
    }

    fn execute(&self, code: &[u8]) -> Result<String, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("cuda".into()));
        }
        let kernel = String::from_utf8_lossy(code);
        Ok(format!("cuda execution result for kernel: {kernel}"))
    }

    fn is_available(&self) -> bool {
        // Stub: detect via env var for testing
        std::env::var("ANIMA_CUDA_AVAILABLE")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false)
    }

    fn platform(&self) -> HardwarePlatform {
        HardwarePlatform::NVIDIACuda
    }
}

// ── Apple ANE ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AnEBackend;

impl Backend for AnEBackend {
    fn name(&self) -> &str {
        "apple_ane"
    }

    fn compile(&self, inference: &str) -> Result<Vec<u8>, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("apple_ane".into()));
        }
        // Stub: emit placeholder CoreML bytes
        Ok(format!("// compiled ane model for: {inference}").into_bytes())
    }

    fn execute(&self, code: &[u8]) -> Result<String, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("apple_ane".into()));
        }
        let model = String::from_utf8_lossy(code);
        Ok(format!("ane execution result for model: {model}"))
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "macos") && cfg!(target_arch = "aarch64")
    }

    fn platform(&self) -> HardwarePlatform {
        HardwarePlatform::AppleANE
    }
}

// ── AMD ROCm ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RocmBackend;

impl Backend for RocmBackend {
    fn name(&self) -> &str {
        "rocm"
    }

    fn compile(&self, inference: &str) -> Result<Vec<u8>, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("rocm".into()));
        }
        Ok(format!("// compiled rocm kernel for: {inference}").into_bytes())
    }

    fn execute(&self, code: &[u8]) -> Result<String, JitError> {
        if !self.is_available() {
            return Err(JitError::BackendUnavailable("rocm".into()));
        }
        let kernel = String::from_utf8_lossy(code);
        Ok(format!("rocm execution result for kernel: {kernel}"))
    }

    fn is_available(&self) -> bool {
        std::env::var("ANIMA_ROCM_AVAILABLE")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false)
    }

    fn platform(&self) -> HardwarePlatform {
        HardwarePlatform::AMDRocm
    }
}

// ── CPU Fallback ────────────────────────────────────────────────────────────

/// Always-available CPU backend. Used as the final fallback.
#[derive(Debug, Clone)]
pub struct CpuFallbackBackend;

impl Backend for CpuFallbackBackend {
    fn name(&self) -> &str {
        "cpu_fallback"
    }

    fn compile(&self, inference: &str) -> Result<Vec<u8>, JitError> {
        // Passthrough — CPU doesn't need real compilation
        Ok(inference.as_bytes().to_vec())
    }

    fn execute(&self, code: &[u8]) -> Result<String, JitError> {
        let input = String::from_utf8_lossy(code);
        Ok(format!("cpu fallback result: {input}"))
    }

    fn is_available(&self) -> bool {
        true
    }

    fn platform(&self) -> HardwarePlatform {
        HardwarePlatform::GenericCpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_fallback_always_available() {
        let backend = CpuFallbackBackend;
        assert!(backend.is_available());
        assert_eq!(backend.platform(), HardwarePlatform::GenericCpu);
    }

    #[test]
    fn cpu_fallback_compile_execute_roundtrip() {
        let backend = CpuFallbackBackend;
        let code = backend.compile("test inference").unwrap();
        let result = backend.execute(&code).unwrap();
        assert!(result.contains("test inference"));
    }

    #[test]
    fn cuda_unavailable_by_default() {
        let backend = CudaBackend;
        assert!(!backend.is_available());
        let err = backend.compile("test").unwrap_err();
        assert!(matches!(err, JitError::BackendUnavailable(_)));
    }

    #[test]
    fn ane_unavailable_on_windows() {
        let backend = AnEBackend;
        // On Windows CI this is always false
        if !cfg!(target_os = "macos") || !cfg!(target_arch = "aarch64") {
            assert!(!backend.is_available());
        }
    }

    #[test]
    fn rocm_unavailable_by_default() {
        let backend = RocmBackend;
        assert!(!backend.is_available());
    }

    #[test]
    fn jit_error_display() {
        let err = JitError::CompilationFailed("bad kernel".into());
        assert_eq!(err.to_string(), "compilation failed: bad kernel");

        let err = JitError::ExecutionFailed("segfault".into());
        assert_eq!(err.to_string(), "execution failed: segfault");

        let err = JitError::BackendUnavailable("cuda".into());
        assert_eq!(err.to_string(), "backend unavailable: cuda");
    }

    #[test]
    fn jit_error_is_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(JitError::CompilationFailed("test".into()));
        assert!(err.to_string().contains("compilation failed"));
    }

    #[test]
    fn all_backends_have_names() {
        let backends: Vec<Box<dyn Backend>> = vec![
            Box::new(CudaBackend),
            Box::new(AnEBackend),
            Box::new(RocmBackend),
            Box::new(CpuFallbackBackend),
        ];
        for b in &backends {
            assert!(!b.name().is_empty());
        }
    }
}
