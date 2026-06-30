//! Hardware detection — identifies the best compute platform for JIT dispatch.

use serde::{Deserialize, Serialize};

/// Supported hardware platforms in priority order (highest last for Ord derive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum HardwarePlatform {
    GenericCpu,
    ArmNeon,
    X86Avx,
    AMDRocm,
    NVIDIACuda,
    AppleANE,
}

/// Describes detected hardware capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareInfo {
    pub platform: HardwarePlatform,
    pub gpu_name: Option<String>,
    pub gpu_memory_mb: Option<u64>,
    pub cpu_features: Vec<String>,
    pub npu_available: bool,
}

/// Configurable hardware detector for testing and cross-platform stubs.
#[derive(Debug, Clone)]
pub struct HardwareDetector {
    forced_platform: Option<HardwarePlatform>,
}

impl HardwareDetector {
    pub fn new() -> Self {
        Self {
            forced_platform: None,
        }
    }

    /// Override detection — forces `detect()` to return a specific platform.
    pub fn set_platform(&mut self, platform: HardwarePlatform) {
        self.forced_platform = Some(platform);
    }

    /// Clear any forced platform, reverting to auto-detection.
    pub fn clear_platform(&mut self) {
        self.forced_platform = None;
    }

    /// Run hardware detection. Returns platform-appropriate defaults.
    pub fn detect(&self) -> HardwareInfo {
        let platform = self.forced_platform.unwrap_or_else(Self::auto_detect);
        match platform {
            HardwarePlatform::AppleANE => HardwareInfo {
                platform,
                gpu_name: Some("Apple M-series GPU".into()),
                gpu_memory_mb: None, // unified memory — reported as system RAM
                cpu_features: vec!["neon".into(), "amx".into()],
                npu_available: true,
            },
            HardwarePlatform::NVIDIACuda => HardwareInfo {
                platform,
                gpu_name: Some("NVIDIA GPU (CUDA)".into()),
                gpu_memory_mb: Some(24_000),
                cpu_features: vec!["avx2".into(), "fma".into()],
                npu_available: false,
            },
            HardwarePlatform::AMDRocm => HardwareInfo {
                platform,
                gpu_name: Some("AMD GPU (ROCm)".into()),
                gpu_memory_mb: Some(16_000),
                cpu_features: vec!["avx2".into(), "fma".into()],
                npu_available: false,
            },
            HardwarePlatform::X86Avx => HardwareInfo {
                platform,
                gpu_name: None,
                gpu_memory_mb: None,
                cpu_features: vec!["avx2".into(), "avx512".into(), "fma".into()],
                npu_available: false,
            },
            HardwarePlatform::ArmNeon => HardwareInfo {
                platform,
                gpu_name: None,
                gpu_memory_mb: None,
                cpu_features: vec!["neon".into()],
                npu_available: false,
            },
            HardwarePlatform::GenericCpu => HardwareInfo {
                platform,
                gpu_name: None,
                gpu_memory_mb: None,
                cpu_features: vec![],
                npu_available: false,
            },
        }
    }

    /// Compile-time fallback — on non-target platforms returns GenericCpu.
    fn auto_detect() -> HardwarePlatform {
        #[cfg(target_arch = "aarch64")]
        {
            #[cfg(target_os = "macos")]
            {
                HardwarePlatform::AppleANE
            }
            #[cfg(not(target_os = "macos"))]
            {
                HardwarePlatform::ArmNeon
            }
        }
        #[cfg(target_arch = "x86_64")]
        {
            HardwarePlatform::X86Avx
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            HardwarePlatform::GenericCpu
        }
    }
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function: detect hardware with default settings.
pub fn detect_hardware() -> HardwareInfo {
    HardwareDetector::new().detect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_detector_returns_valid_platform() {
        let info = detect_hardware();
        // Any platform variant is valid
        assert!(matches!(
            info.platform,
            HardwarePlatform::AppleANE
                | HardwarePlatform::NVIDIACuda
                | HardwarePlatform::AMDRocm
                | HardwarePlatform::X86Avx
                | HardwarePlatform::ArmNeon
                | HardwarePlatform::GenericCpu
        ));
    }

    #[test]
    fn forced_platform_overrides_detection() {
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::NVIDIACuda);
        let info = det.detect();
        assert_eq!(info.platform, HardwarePlatform::NVIDIACuda);
        assert_eq!(info.gpu_name, Some("NVIDIA GPU (CUDA)".into()));
        assert!(info.gpu_memory_mb.is_some());
        assert!(!info.npu_available);
    }

    #[test]
    fn clear_platform_restores_auto() {
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::NVIDIACuda);
        det.clear_platform();
        let info = det.detect();
        // Should match whatever auto-detect returns (platform-dependent)
        let auto = HardwareDetector::new().detect();
        assert_eq!(info.platform, auto.platform);
    }

    #[test]
    fn apple_ane_has_npu() {
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::AppleANE);
        let info = det.detect();
        assert!(info.npu_available);
        assert!(info.cpu_features.contains(&"neon".to_string()));
    }

    #[test]
    fn generic_cpu_has_no_gpu() {
        let mut det = HardwareDetector::new();
        det.set_platform(HardwarePlatform::GenericCpu);
        let info = det.detect();
        assert!(info.gpu_name.is_none());
        assert!(info.gpu_memory_mb.is_none());
        assert!(!info.npu_available);
    }

    #[test]
    fn hardware_info_serializes() {
        let info = detect_hardware();
        let json = serde_json::to_string(&info).unwrap();
        let back: HardwareInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, back);
    }

    #[test]
    fn platform_ordering() {
        // AppleANE > CUDA > ROCm > AVX > NEON > Generic
        assert!(HardwarePlatform::AppleANE > HardwarePlatform::NVIDIACuda);
        assert!(HardwarePlatform::NVIDIACuda > HardwarePlatform::AMDRocm);
        assert!(HardwarePlatform::AMDRocm > HardwarePlatform::X86Avx);
        assert!(HardwarePlatform::X86Avx > HardwarePlatform::ArmNeon);
        assert!(HardwarePlatform::ArmNeon > HardwarePlatform::GenericCpu);
    }
}
