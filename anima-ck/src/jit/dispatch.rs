//! JIT dispatch engine — maps platforms to backends and executes with fallback.

use std::collections::HashMap;

use super::backend::{AnEBackend, Backend, CpuFallbackBackend, CudaBackend, JitError, RocmBackend};
use super::detection::{HardwareInfo, HardwarePlatform};

/// Central dispatcher that holds registered backends and routes inferences.
pub struct JitDispatcher {
    backends: HashMap<HardwarePlatform, Box<dyn Backend>>,
}

impl std::fmt::Debug for JitDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let platforms: Vec<_> = self.backends.keys().collect();
        f.debug_struct("JitDispatcher")
            .field("platforms", &platforms)
            .finish()
    }
}

impl JitDispatcher {
    /// Create a dispatcher with all known backends registered.
    pub fn new() -> Self {
        let mut backends: HashMap<HardwarePlatform, Box<dyn Backend>> = HashMap::new();

        // Register GPU backends (will report is_available() = false on unsupported systems)
        backends.insert(HardwarePlatform::NVIDIACuda, Box::new(CudaBackend));
        backends.insert(HardwarePlatform::AppleANE, Box::new(AnEBackend));
        backends.insert(HardwarePlatform::AMDRocm, Box::new(RocmBackend));

        // CPU fallback is always available
        backends.insert(HardwarePlatform::GenericCpu, Box::new(CpuFallbackBackend));

        Self { backends }
    }

    /// Register a custom backend for a platform.
    pub fn register(&mut self, platform: HardwarePlatform, backend: Box<dyn Backend>) {
        self.backends.insert(platform, backend);
    }

    /// Compile and execute an inference on the specified platform.
    pub fn dispatch(
        &self,
        platform: HardwarePlatform,
        inference: &str,
    ) -> Result<String, JitError> {
        let backend = self
            .backends
            .get(&platform)
            .ok_or_else(|| JitError::BackendUnavailable(format!("no backend for {platform:?}")))?;

        if !backend.is_available() {
            return Err(JitError::BackendUnavailable(format!(
                "{} is not available",
                backend.name()
            )));
        }

        let code = backend.compile(inference)?;
        backend.execute(&code)
    }

    /// List platforms that have registered AND available backends.
    pub fn available_platforms(&self) -> Vec<HardwarePlatform> {
        self.backends
            .iter()
            .filter(|(_, b)| b.is_available())
            .map(|(p, _)| *p)
            .collect()
    }

    /// Try the best platform from `HardwareInfo`, falling back to CPU on failure.
    pub fn fallback(&self, inference: &str, hw: &HardwareInfo) -> Result<String, JitError> {
        // Try the detected platform first
        if let Ok(result) = self.dispatch(hw.platform, inference) {
            return Ok(result);
        }

        // Fall back to CPU
        self.dispatch(HardwarePlatform::GenericCpu, inference)
    }
}

impl Default for JitDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::detection::HardwareDetector;

    fn make_dispatcher() -> JitDispatcher {
        JitDispatcher::new()
    }

    #[test]
    fn new_registers_four_platforms() {
        let d = make_dispatcher();
        assert_eq!(d.backends.len(), 4);
    }

    #[test]
    fn dispatch_cpu_succeeds() {
        let d = make_dispatcher();
        let result = d.dispatch(HardwarePlatform::GenericCpu, "hello");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));
    }

    #[test]
    fn dispatch_cuda_fails_when_unavailable() {
        let d = make_dispatcher();
        let result = d.dispatch(HardwarePlatform::NVIDIACuda, "test");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            JitError::BackendUnavailable(_)
        ));
    }

    #[test]
    fn available_platforms_includes_cpu() {
        let d = make_dispatcher();
        let platforms = d.available_platforms();
        assert!(platforms.contains(&HardwarePlatform::GenericCpu));
    }

    #[test]
    fn fallback_uses_cpu_when_platform_unavailable() {
        let d = make_dispatcher();
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::NVIDIACuda);
        let hw = det.detect();
        let result = d.fallback("test inference", &hw);
        // Should succeed via CPU fallback
        assert!(result.is_ok());
        assert!(result.unwrap().contains("cpu fallback"));
    }

    #[test]
    fn fallback_succeeds_with_cpu_platform() {
        let d = make_dispatcher();
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::GenericCpu);
        let hw = det.detect();
        let result = d.fallback("direct path", &hw);
        assert!(result.is_ok());
    }

    #[test]
    fn dispatch_unregistered_platform() {
        let d = make_dispatcher();
        let result = d.dispatch(HardwarePlatform::ArmNeon, "test");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            JitError::BackendUnavailable(_)
        ));
    }

    #[test]
    fn register_custom_backend() {
        let mut d = make_dispatcher();
        d.register(HardwarePlatform::ArmNeon, Box::new(CpuFallbackBackend));
        let result = d.dispatch(HardwarePlatform::ArmNeon, "custom");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("custom"));
    }

    #[test]
    fn dispatcher_default_is_same_as_new() {
        let a = JitDispatcher::new();
        let b = JitDispatcher::default();
        assert_eq!(a.backends.len(), b.backends.len());
    }
}
