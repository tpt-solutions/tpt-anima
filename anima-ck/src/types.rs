//! Core data structures for the Cognitive Kernel.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Serde helper for `SystemTime`.
pub(crate) mod serde_system_time {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        let millis = u128::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_millis(millis as u64))
    }
}

// ── Priority ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[repr(u8)]
pub enum InferencePriority {
    Deferred = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

// ── Resource requests & hardware ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRequest {
    pub cpu_cores: u32,
    pub gpu_count: u32,
    pub vram_mb: u64,
    pub ram_mb: u64,
    pub nvme_gb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThermalConstraints {
    pub max_temp_celsius: f32,
    pub max_power_watts: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareStatus {
    pub temperature_celsius: f32,
    pub power_watts: f32,
    pub gpu_utilization: f32,
    pub cpu_utilization: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
}

// ── KV cache ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvictionPolicy {
    LRU,
    LFU,
    Random,
    AttentionBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KVCacheConfig {
    pub cache_size_tokens: u64,
    pub eviction_policy: EvictionPolicy,
    pub prefetch_depth: u32,
}

// ── Circuit breaker ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CircuitBreakerConfig {
    pub entropy_threshold: f32,
    pub max_repetition_count: u32,
    pub causal_violation_tolerance: u32,
}

// ── Inference ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Inference {
    pub id: Uuid,
    pub model_id: String,
    pub resource_requirements: ResourceRequest,
    pub kv_cache_config: KVCacheConfig,
    pub circuit_breaker_settings: CircuitBreakerConfig,
    pub priority: InferencePriority,
}

// ── Memory & isolation ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryRegionType {
    VRAM,
    RAM,
    NVMe,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRegion {
    pub id: Uuid,
    pub size_bytes: u64,
    pub region_type: MemoryRegionType,
    pub data_hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IsolationLevel {
    Process,
    Thread,
    Enclave,
    Hypervisor,
}

// ── ThoughtBubble ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThoughtBubble {
    pub id: Uuid,
    pub sandbox_id: String,
    pub isolation_level: IsolationLevel,
    pub memory_regions: Vec<MemoryRegion>,
    #[serde(with = "serde_system_time")]
    pub created_at: SystemTime,
    pub is_compromised: bool,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn priority_ordering() {
        assert!(InferencePriority::Critical > InferencePriority::High);
        assert!(InferencePriority::High > InferencePriority::Normal);
        assert!(InferencePriority::Normal > InferencePriority::Low);
        assert!(InferencePriority::Low > InferencePriority::Deferred);
    }

    #[test]
    fn priority_repr() {
        assert_eq!(InferencePriority::Deferred as u8, 0);
        assert_eq!(InferencePriority::Low as u8, 1);
        assert_eq!(InferencePriority::Normal as u8, 2);
        assert_eq!(InferencePriority::High as u8, 3);
        assert_eq!(InferencePriority::Critical as u8, 4);
    }

    #[test]
    fn inference_roundtrip() {
        let inf = Inference {
            id: Uuid::new_v4(),
            model_id: "gpt-4o".into(),
            resource_requirements: ResourceRequest {
                cpu_cores: 8,
                gpu_count: 2,
                vram_mb: 24_000,
                ram_mb: 64_000,
                nvme_gb: 1_000,
            },
            kv_cache_config: KVCacheConfig {
                cache_size_tokens: 128_000,
                eviction_policy: EvictionPolicy::AttentionBased,
                prefetch_depth: 4,
            },
            circuit_breaker_settings: CircuitBreakerConfig {
                entropy_threshold: 0.85,
                max_repetition_count: 5,
                causal_violation_tolerance: 2,
            },
            priority: InferencePriority::High,
        };
        let json = serde_json::to_string(&inf).unwrap();
        let back: Inference = serde_json::from_str(&json).unwrap();
        assert_eq!(inf, back);
    }

    #[test]
    fn thought_bubble_roundtrip() {
        let bubble = ThoughtBubble {
            id: Uuid::new_v4(),
            sandbox_id: "sb-001".into(),
            isolation_level: IsolationLevel::Enclave,
            memory_regions: vec![MemoryRegion {
                id: Uuid::new_v4(),
                size_bytes: 1_073_741_824,
                region_type: MemoryRegionType::VRAM,
                data_hash: [0xab; 32],
            }],
            created_at: SystemTime::now(),
            is_compromised: false,
        };
        let json = serde_json::to_string(&bubble).unwrap();
        let back: ThoughtBubble = serde_json::from_str(&json).unwrap();
        assert_eq!(bubble.id, back.id);
        assert_eq!(bubble.sandbox_id, back.sandbox_id);
        assert_eq!(bubble.memory_regions[0].data_hash, [0xab; 32]);
    }

    #[test]
    fn system_time_serde_precision() {
        let now = SystemTime::now();
        let json = serde_system_time::serialize(&now, serde_json::value::Serializer).unwrap();
        let restored = serde_system_time::deserialize(&mut serde_json::Deserializer::from_str(
            &json.to_string(),
        ))
        .unwrap();
        let diff = now
            .duration_since(restored)
            .unwrap_or_else(|e| e.duration());
        assert!(diff < Duration::from_millis(2));
    }

    #[test]
    fn hardware_status_defaults() {
        let hw = HardwareStatus {
            temperature_celsius: 45.0,
            power_watts: 250.0,
            gpu_utilization: 0.75,
            cpu_utilization: 0.40,
            memory_used_mb: 16_000,
            memory_total_mb: 24_000,
        };
        assert!(hw.temperature_celsius > 0.0);
        assert!(hw.gpu_utilization <= 1.0);
    }

    #[test]
    fn eviction_policy_variants() {
        let policies = [
            EvictionPolicy::LRU,
            EvictionPolicy::LFU,
            EvictionPolicy::Random,
            EvictionPolicy::AttentionBased,
        ];
        for p in &policies {
            let json = serde_json::to_string(p).unwrap();
            let back: EvictionPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(*p, back);
        }
    }

    #[test]
    fn isolation_level_variants() {
        let levels = [
            IsolationLevel::Process,
            IsolationLevel::Thread,
            IsolationLevel::Enclave,
            IsolationLevel::Hypervisor,
        ];
        for l in &levels {
            let json = serde_json::to_string(l).unwrap();
            let back: IsolationLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(*l, back);
        }
    }
}
