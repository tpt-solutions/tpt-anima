//! JIT Hardware-Fluid Engine — compile-time platform detection with runtime dispatch.
//!
//! Maps inference workloads to the best available hardware backend,
//! falling back gracefully through a prioritized chain.

pub mod backend;
pub mod detection;
pub mod dispatch;

pub use backend::{Backend, CudaBackend, CpuFallbackBackend, JitError, RocmBackend, AnEBackend};
pub use detection::{detect_hardware, HardwareDetector, HardwareInfo, HardwarePlatform};
pub use dispatch::JitDispatcher;
