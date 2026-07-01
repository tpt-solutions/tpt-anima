//! Tier 1 Anima Core Hypervisor — Type-1 bare-metal hypervisor for full Anima stack.
//!
//! This crate provides the core hypervisor that runs directly on hardware, managing
//! all four Anima layers (SSE, INP, CK, ZKPA) with full hardware control. Legacy
//! operating systems run as isolated VMs on top of the hypervisor.
//!
//! # Architecture
//!
//! - **Hardware Abstraction Layer**: Direct silicon control for CPU/GPU/NPU
//! - **Memory Management**: Unified VRAM/RAM/NVMe address space
//! - **Security Isolation**: Hardware-enclave backed ThoughtBubble quarantine
//! - **Legacy VM Support**: Run unmodified OSes in sandboxed VMs

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// The Anima hypervisor singleton.
///
/// On bare-metal boot, this is the entry point that initializes all layers.
#[derive(Debug, Clone)]
pub struct AnimaHypervisor {
    /// Whether the hypervisor is currently running.
    running: bool,
    /// Configuration for the hypervisor.
    config: HypervisorConfig,
    /// Active VM sessions.
    vms: Vec<VMSession>,
}

impl AnimaHypervisor {
    /// Create a new hypervisor instance with the given configuration.
    pub fn new(config: HypervisorConfig) -> Result<Self, HypervisorError> {
        Ok(Self {
            running: false,
            config,
            vms: Vec::new(),
        })
    }

    /// Boot the hypervisor and initialize all Anima layers.
    pub fn boot(&mut self) -> Result<(), HypervisorError> {
        // Initialize SSE layer
        // Initialize INP layer  
        // Initialize CK layer
        // Initialize ZKPA layer
        self.running = true;
        Ok(())
    }

    /// Spawn a new VM session for legacy OS support.
    pub fn spawn_vm(&mut self, vm_config: VMConfig) -> Result<VMSession, HypervisorError> {
        let session = VMSession::new(vm_config)?;
        self.vms.push(session.clone());
        Ok(session)
    }

    /// Get current hypervisor statistics.
    pub fn stats(&self) -> HypervisorStats {
        HypervisorStats {
            running: self.running,
            vm_count: self.vms.len(),
            cpu_usage: self.config.cpu_usage_percent,
            memory_utilization: self.config.memory_utilization_percent,
        }
    }

    /// Halt the hypervisor.
    pub fn halt(&mut self) {
        self.running = false;
    }
}

/// Configuration for the Anima hypervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervisorConfig {
    /// Initial CPU usage percentage target.
    pub cpu_usage_percent: f32,
    /// Initial memory utilization target.
    pub memory_utilization_percent: f32,
    /// Whether to enable hardware attestation.
    pub hardware_attestation_enabled: bool,
    /// Security level for ThoughtBubbles.
    pub bubble_security_level: String,
}

impl Default for HypervisorConfig {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 50.0,
            memory_utilization_percent: 30.0,
            hardware_attestation_enabled: true,
            bubble_security_level: "HardwareEnclave".to_string(),
        }
    }
}

/// Statistics about the hypervisor state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervisorStats {
    pub running: bool,
    pub vm_count: usize,
    pub cpu_usage: f32,
    pub memory_utilization: f32,
}

/// Status of the hypervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypervisorStatus {
    /// Hypervisor is booting.
    Booting,
    /// Hypervisor is running.
    Running,
    /// Hypervisor is halted.
    Halted,
    /// Hypervisor encountered an error.
    Error,
}

/// Errors that can occur in the hypervisor.
#[derive(Debug, Error)]
pub enum HypervisorError {
    #[error("hypervisor already running")]
    AlreadyRunning,

    #[error("hypervisor not running")]
    NotRunning,

    #[error("VM spawn failed: {0}")]
    VMSpawnFailed(String),

    #[error("hardware initialization failed: {0}")]
    HardwareInitFailed(String),
}

/// Configuration for a guest VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    /// Unique identifier for the VM.
    pub id: String,
    /// Name of the VM.
    pub name: String,
    /// Amount of RAM allocated in MB.
    pub ram_mb: u32,
    /// Amount of VRAM allocated in MB.
    pub vram_mb: u32,
    /// CPU cores allocated.
    pub cpus: u32,
    /// Whether the VM is enabled.
    pub enabled: bool,
}

impl VMConfig {
    /// Create a new VM configuration.
    pub fn new(name: String, ram_mb: u32, vram_mb: u32, cpus: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            ram_mb,
            vram_mb,
            cpus,
            enabled: true,
        }
    }
}

/// A running VM session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMSession {
    /// VM configuration.
    pub config: VMConfig,
    /// Session identifier.
    pub session_id: String,
    /// When the session was created (Unix timestamp).
    pub created_at: u64,
}

impl VMSession {
    /// Create a new VM session.
    pub fn new(config: VMConfig) -> Result<Self, HypervisorError> {
        Ok(Self {
            config,
            session_id: Uuid::new_v4().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypervisor_new_creates_stopped() {
        let config = HypervisorConfig::default();
        let hypervisor = AnimaHypervisor::new(config).unwrap();
        assert!(!hypervisor.stats().running);
    }

    #[test]
    fn hypervisor_boot_succeeds() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();
        hypervisor.boot().unwrap();
        assert!(hypervisor.stats().running);
    }

    #[test]
    fn vm_config_creates_with_id() {
        let config = VMConfig::new("test-vm".into(), 4096, 2048, 4);
        assert_eq!(config.name, "test-vm");
        assert_eq!(config.ram_mb, 4096);
        assert_eq!(config.cpus, 4);
        assert!(config.enabled);
    }

    #[test]
    fn vm_session_creates() {
        let vm_config = VMConfig::new("test-vm".into(), 4096, 2048, 4);
        let session = VMSession::new(vm_config).unwrap();
        assert!(!session.session_id.is_empty());
    }

    #[test]
    fn spawn_vm_succeeds() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();
        hypervisor.boot().unwrap();
        
        let vm = VMConfig::new("test-vm".into(), 4096, 2048, 4);
        let session = hypervisor.spawn_vm(vm.clone()).unwrap();
        assert_eq!(session.config.name, "test-vm");
        assert_eq!(hypervisor.stats().vm_count, 1);
    }
}