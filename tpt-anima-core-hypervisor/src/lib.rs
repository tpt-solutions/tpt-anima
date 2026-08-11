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

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// The Anima hypervisor singleton.
///
/// # What this actually is
///
/// This performs **real software-level orchestration** of the four Anima layer
/// subsystems (SSE, INP, CK, ZKPA) within a single hosted process: it
/// constructs one real instance of each subsystem and drives one genuine
/// operation through each subsystem's real API during [`AnimaHypervisor::boot`]
/// as a health check, failing boot if any subsystem does not come up cleanly.
///
/// It explicitly does **not** perform bare-metal hardware virtualization —
/// there is no VMX/SVM setup, no custom bootloader, and no ring transitions
/// here. Literal bare-metal hypervisor concerns (hardware-enclave backed
/// isolation, direct silicon control, legacy-OS VM support) are a separate,
/// out-of-scope OS-dev project undertaken elsewhere. `spawn_vm`/`VMSession`
/// below model VM *bookkeeping* only, not real virtualization.
///
/// Note: unlike the earlier stub, this struct no longer derives
/// `Debug`/`Clone` — the real subsystem types it now owns (SSE storage, INP
/// registry, CK scheduler, ZKPA sessions) do not implement those traits.
pub struct AnimaHypervisor {
    /// Whether the hypervisor is currently running.
    running: bool,
    /// Configuration for the hypervisor.
    config: HypervisorConfig,
    /// Active VM sessions.
    vms: Vec<VMSession>,
    /// SSE (Semantic State Engine) memory substrate.
    sse_storage: tpt_anima_sse::InMemoryStorage,
    /// INP (Intent & Negotiation Protocol) capability registry.
    inp_registry: tpt_anima_inp::registry::CapabilityRegistry,
    /// CK (Cognitive Kernel) inference scheduler.
    ck_scheduler: tpt_anima_ck::scheduler::InferenceScheduler,
    /// ZKPA (Zero-Knowledge Proof of Agency) session manager.
    zkpa_sessions: tpt_anima_zkpa::session::SessionManager,
}

impl AnimaHypervisor {
    /// Create a new hypervisor instance with the given configuration.
    pub fn new(config: HypervisorConfig) -> Result<Self, HypervisorError> {
        Ok(Self {
            running: false,
            config,
            vms: Vec::new(),
            sse_storage: tpt_anima_sse::InMemoryStorage::new(),
            inp_registry: tpt_anima_inp::registry::CapabilityRegistry::new(),
            ck_scheduler: tpt_anima_ck::scheduler::InferenceScheduler::new(),
            zkpa_sessions: tpt_anima_zkpa::session::SessionManager::new(),
        })
    }

    /// Boot the hypervisor: construct and health-check the four Anima layer
    /// subsystems.
    ///
    /// This is real software-level orchestration in a hosted process, not
    /// bare-metal hardware virtualization (see the struct-level doc comment).
    /// For each layer this performs one genuine, meaningful operation against
    /// its real API:
    ///
    /// - **SSE**: writes a real [`tpt_anima_sse::MemoryNode`] into
    ///   [`tpt_anima_sse::InMemoryStorage`] and confirms it round-trips via
    ///   `read_node`.
    /// - **INP**: registers a real [`tpt_anima_inp::intent::Capability`] with the
    ///   [`tpt_anima_inp::registry::CapabilityRegistry`] and confirms it is
    ///   retrievable by ID.
    /// - **CK**: submits a real [`tpt_anima_ck::types::Inference`] job to the
    ///   [`tpt_anima_ck::scheduler::InferenceScheduler`] and confirms
    ///   `schedule_next` actually selects and runs it.
    /// - **ZKPA**: creates a real, task-scoped, time-limited
    ///   [`tpt_anima_zkpa::types::SessionCredential`] via
    ///   [`tpt_anima_zkpa::session::SessionManager`] and confirms it validates.
    ///
    /// If any of these four checks fails, `boot()` returns
    /// `Err(HypervisorError::SubsystemHealthCheckFailed)` and the hypervisor
    /// is left in the not-running state.
    pub fn boot(&mut self) -> Result<(), HypervisorError> {
        self.boot_check_sse()?;
        self.boot_check_inp()?;
        self.boot_check_ck()?;
        self.boot_check_zkpa()?;

        self.running = true;
        Ok(())
    }

    /// SSE health check: write one real memory node and confirm it round-trips.
    fn boot_check_sse(&mut self) -> Result<(), HypervisorError> {
        let creator_agent_id: tpt_anima_sse::memory_node::AgentId = [1u8; 32];
        let node_id: tpt_anima_sse::memory_node::MemoryId = [2u8; 32];
        let embedding = tpt_anima_sse::memory_node::VectorEmbedding::new(vec![0.0, 1.0, 0.0]);
        let node = tpt_anima_sse::memory_node::MemoryNode::new(
            node_id,
            embedding,
            b"hypervisor boot health check".to_vec(),
            "text/plain".to_string(),
            tpt_anima_sse::memory_node::SensoryModality::Abstract,
            creator_agent_id,
        );

        self.sse_storage.write_node(node);

        self.sse_storage
            .read_node(&node_id)
            .ok_or(HypervisorError::SubsystemHealthCheckFailed(
                "SSE: written memory node did not round-trip through InMemoryStorage".to_string(),
            ))?;

        Ok(())
    }

    /// INP health check: register one real capability and confirm lookup.
    fn boot_check_inp(&mut self) -> Result<(), HypervisorError> {
        let provider_agent_id = Uuid::new_v4();
        let capability = tpt_anima_inp::intent::Capability::new(
            vec!["hypervisor.boot_health_check".to_string()],
            tpt_anima_inp::intent::PerformanceProfile {
                typical_latency: Duration::from_millis(10),
                max_latency: Duration::from_millis(100),
                typical_accuracy: 1.0,
                throughput_per_second: 1.0,
            },
            tpt_anima_inp::intent::EconomicTerms::FixedPrice { credits: 0.0 },
            provider_agent_id,
        );
        let capability_id = self.inp_registry.register(capability);

        self.inp_registry.get(&capability_id).ok_or(
            HypervisorError::SubsystemHealthCheckFailed(
                "INP: registered capability was not retrievable from CapabilityRegistry"
                    .to_string(),
            ),
        )?;

        Ok(())
    }

    /// CK health check: submit one real inference job and confirm the
    /// scheduler picks it up and runs it.
    fn boot_check_ck(&mut self) -> Result<(), HypervisorError> {
        let inference = tpt_anima_ck::types::Inference {
            id: Uuid::new_v4(),
            model_id: "hypervisor-boot-health-check".to_string(),
            resource_requirements: tpt_anima_ck::types::ResourceRequest {
                cpu_cores: 1,
                gpu_count: 0,
                vram_mb: 0,
                ram_mb: 64,
                nvme_gb: 0,
            },
            kv_cache_config: tpt_anima_ck::types::KVCacheConfig {
                cache_size_tokens: 0,
                eviction_policy: tpt_anima_ck::types::EvictionPolicy::LRU,
                prefetch_depth: 0,
            },
            circuit_breaker_settings: tpt_anima_ck::types::CircuitBreakerConfig {
                entropy_threshold: 1.0,
                max_repetition_count: 8,
                causal_violation_tolerance: 0,
            },
            priority: tpt_anima_ck::types::InferencePriority::Normal,
        };

        let job_index = self.ck_scheduler.submit(inference, 1.0);

        let scheduled = self.ck_scheduler.schedule_next().ok_or(
            HypervisorError::SubsystemHealthCheckFailed(
                "CK: submitted inference job was not selected by InferenceScheduler".to_string(),
            ),
        )?;

        if scheduled.status != tpt_anima_ck::scheduler::JobStatus::Running {
            return Err(HypervisorError::SubsystemHealthCheckFailed(format!(
                "CK: expected job {job_index} to be Running after schedule_next, got {:?}",
                scheduled.status
            )));
        }

        Ok(())
    }

    /// ZKPA health check: create one real ephemeral session and confirm it
    /// validates.
    fn boot_check_zkpa(&mut self) -> Result<(), HypervisorError> {
        let agent_id = Uuid::new_v4();
        let credential = self.zkpa_sessions.create_session(
            agent_id,
            "hypervisor.boot_health_check".to_string(),
            0.0,
            Duration::from_secs(60),
            None,
        );

        self.zkpa_sessions
            .validate_session(credential.id)
            .map_err(|e| {
                HypervisorError::SubsystemHealthCheckFailed(format!(
                    "ZKPA: freshly created session failed validation: {e}"
                ))
            })?;

        Ok(())
    }

    /// Read-only access to the SSE storage, e.g. for diagnostics.
    pub fn sse_storage(&self) -> &tpt_anima_sse::InMemoryStorage {
        &self.sse_storage
    }

    /// Read-only access to the INP capability registry, e.g. for diagnostics.
    pub fn inp_registry(&self) -> &tpt_anima_inp::registry::CapabilityRegistry {
        &self.inp_registry
    }

    /// Read-only access to the ZKPA session manager, e.g. for diagnostics.
    pub fn zkpa_sessions(&self) -> &tpt_anima_zkpa::session::SessionManager {
        &self.zkpa_sessions
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

    #[error("subsystem health check failed during boot: {0}")]
    SubsystemHealthCheckFailed(String),
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
    fn hypervisor_boot_writes_real_sse_node() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();
        hypervisor.boot().unwrap();

        // The boot health check must have actually written a node into SSE
        // storage — not just flipped a boolean.
        let node_id: tpt_anima_sse::memory_node::MemoryId = [2u8; 32];
        let node = hypervisor
            .sse_storage()
            .read_node(&node_id)
            .expect("boot() must leave a real SSE memory node in storage");
        assert_eq!(node.payload, b"hypervisor boot health check");
    }

    #[test]
    fn hypervisor_boot_registers_real_inp_capability() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();
        hypervisor.boot().unwrap();

        // The boot health check must have actually registered a capability.
        let matches = hypervisor
            .inp_registry()
            .lookup_by_action("hypervisor.boot_health_check");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn hypervisor_boot_creates_real_zkpa_session() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();
        hypervisor.boot().unwrap();

        // The boot health check must have actually created a live session.
        let active = hypervisor.zkpa_sessions().active_sessions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].task_scope, "hypervisor.boot_health_check");
    }

    #[test]
    fn hypervisor_boot_runs_real_ck_job() {
        let config = HypervisorConfig::default();
        let mut hypervisor = AnimaHypervisor::new(config).unwrap();

        // Before boot, the scheduler has no jobs to select.
        assert!(hypervisor.ck_scheduler.schedule_next().is_none());

        hypervisor.boot().unwrap();

        // The boot health check job was already moved to Running and consumed
        // by schedule_next() inside boot(); scheduling again must find no
        // further ready jobs, proving real state changed rather than nothing
        // having been submitted at all.
        assert!(hypervisor.ck_scheduler.schedule_next().is_none());
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
