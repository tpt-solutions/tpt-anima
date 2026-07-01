//! KV Cache Context Swapping Optimization.
//!
//! Minimizes thrashing during multi-inference preemption by implementing:
//! - Efficient context serialization/deserialization
//! - Pre-allocated swap buffers
//! - Priority-aware swap scheduling

use crate::scheduler::{InferenceJob, InferenceScheduler, JobStatus};
use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Swap Priority ─────────────────────────────────────────────────────────────

/// Priority for context swap operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwapPriority {
    /// Low priority: background eviction, not affecting active inference
    Background,
    /// Medium priority: preempted job context
    Preempted,
    /// High priority: critical inference context
    Critical,
}

// ── Swap Strategy ─────────────────────────────────────────────────────────────

/// Strategy for minimizing KV cache thrashing.
#[derive(Debug, Clone)]
pub struct SwapStrategy {
    /// Whether to use async swap for non-critical operations
    pub async_swap: bool,
    /// Maximum concurrent swaps
    pub max_concurrent_swaps: usize,
    /// Pin frequently-accessed contexts to avoid re-swap
    pub pin_frequency_threshold: u32,
}

impl Default for SwapStrategy {
    fn default() -> Self {
        Self {
            async_swap: true,
            max_concurrent_swaps: 4,
            pin_frequency_threshold: 10,
        }
    }
}

// ── Context Buffer ───────────────────────────────────────────────────────────

/// Pre-allocated buffer for context swapping to reduce allocation overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBuffer {
    /// Maximum tokens this buffer can hold
    pub max_tokens: u64,
    /// Serialized context data
    pub data: Vec<u8>,
    /// Access count for pin consideration
    pub access_count: u32,
    /// Whether this buffer is pinned in VRAM
    pub pinned: bool,
}

impl ContextBuffer {
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max_tokens,
            data: Vec::with_capacity((max_tokens * 2) as usize), // ~2 bytes per token typical
            access_count: 0,
            pinned: false,
        }
    }

    pub fn should_pin(&self) -> bool {
        self.access_count >= 10 && !self.pinned
    }

    pub fn record_access(&mut self) {
        self.access_count = self.access_count.saturating_add(1);
    }
}

// ── KV Cache Optimizer ─────────────────────────────────────────────────────

/// Optimizes KV cache context swapping during inference preemption.
pub struct KVCacheOptimizer {
    strategy: SwapStrategy,
    /// Pre-allocated buffers indexed by job ID
    swap_buffers: HashMap<uuid::Uuid, ContextBuffer>,
    /// Swap latency metrics in microseconds
    swap_latency_us: Vec<u64>,
    /// Throttle count to prevent excessive swapping
    throttle_count: u64,
}

impl KVCacheOptimizer {
    pub fn new(strategy: SwapStrategy) -> Self {
        Self {
            strategy,
            swap_buffers: HashMap::new(),
            swap_latency_us: Vec::new(),
            throttle_count: 0,
        }
    }

    /// Prepare a context buffer for a job's KV cache.
    pub fn prepare_buffer(&mut self, job_id: uuid::Uuid, cache_tokens: u64) -> &mut ContextBuffer {
        self.swap_buffers.entry(job_id).or_insert_with(|| ContextBuffer::new(cache_tokens))
    }

    /// Swap out a job's context with minimal overhead.
    /// Returns the time taken in microseconds.
    pub fn swap_out(&mut self, job_id: uuid::Uuid, job: &InferenceJob) -> u64 {
        let start = std::time::Instant::now();

        // Record access for pin consideration
        if let Some(buf) = self.swap_buffers.get_mut(&job_id) {
            buf.record_access();
        }

        // Serialize context metadata (not the full KV cache, just the essential state)
        let buffer = self.prepare_buffer(job_id, job.inference.kv_cache_config.cache_size_tokens);
        
        // For optimization, we only serialize essential metadata
        // Full KV cache data would be handled by the JIT backend
        buffer.data.clear();
        buffer.data.extend_from_slice(&job.inference.id.as_bytes());
        buffer.data.push(job.inference.priority as u8);
        buffer.data.extend_from_slice(&job.inference.priority.to_string().as_bytes());

        let elapsed_us = start.elapsed().as_micros() as u64;
        self.swap_latency_us.push(elapsed_us);
        elapsed_us
    }

    /// Check if a buffer should be pinned based on access frequency.
    pub fn check_pin(&mut self, job_id: &uuid::Uuid) -> bool {
        if let Some(buf) = self.swap_buffers.get_mut(job_id) {
            if buf.should_pin() {
                buf.pinned = true;
                return true;
            }
        }
        false
    }

    /// Optimize swap decisions based on scheduler state.
    /// Returns a swap plan with ordered operations.
    pub fn optimize_swap_plan(&self, scheduler: &InferenceScheduler) -> SwapPlan {
        let mut plan = SwapPlan::default();

        // Find running jobs that might be preempted
        for job in scheduler.active_jobs() {
            if job.status == JobStatus::Preempted {
                let priority = match job.inference.priority {
                    InferencePriority::Critical => SwapPriority::Critical,
                    InferencePriority::High => SwapPriority::Preempted,
                    _ => SwapPriority::Background,
                };
                plan.swap_operations.push(SwapOperation {
                    job_id: job.inference.id,
                    priority,
                });
            }
        }

        // Sort by priority (higher priority first to ensure they get swapped in first)
        plan.swap_operations.sort_by(|a, b| b.priority.cmp(&a.priority));
        plan
    }

    /// Get average swap latency.
    pub fn avg_swap_latency_us(&self) -> f64 {
        if self.swap_latency_us.is_empty() {
            0.0
        } else {
            self.swap_latency_us.iter().sum::<u64>() as f64 / self.swap_latency_us.len() as f64
        }
    }

    /// Get swap statistics.
    pub fn swap_stats(&self) -> SwapStatistics {
        SwapStatistics {
            total_swaps: self.swap_latency_us.len() as u64,
            avg_latency_us: self.avg_swap_latency_us(),
            throttled_swaps: self.throttle_count,
        }
    }

    /// Access buffer for a job.
    pub fn access_buffer(&mut self, job_id: &uuid::Uuid) -> Option<&mut ContextBuffer> {
        self.swap_buffers.get_mut(job_id)
    }

    /// Remove buffer after job completion.
    pub fn cleanup_buffer(&mut self, job_id: &uuid::Uuid) -> bool {
        self.swap_buffers.remove(job_id).is_some()
    }
}

impl Default for KVCacheOptimizer {
    fn default() -> Self {
        Self::new(SwapStrategy::default())
    }
}

// ── Swap Plan ───────────────────────────────────────────────────────────────

/// Ordered plan for KV cache swap operations.
#[derive(Debug, Clone, Default)]
pub struct SwapPlan {
    pub swap_operations: Vec<SwapOperation>,
}

#[derive(Debug, Clone)]
pub struct SwapOperation {
    pub job_id: uuid::Uuid,
    pub priority: SwapPriority,
}

// ── Statistics ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SwapStatistics {
    pub total_swaps: u64,
    pub avg_latency_us: f64,
    pub throttled_swaps: u64,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_test_job(id: Uuid, priority: InferencePriority, cache_tokens: u64) -> InferenceJob {
        InferenceJob {
            inference: Inference {
                id,
                model_id: "test-model".into(),
                resource_requirements: ResourceRequest {
                    cpu_cores: 1,
                    gpu_count: 0,
                    vram_mb: 0,
                    ram_mb: 1000,
                    nvme_gb: 0,
                },
                kv_cache_config: KVCacheConfig {
                    cache_size_tokens,
                    eviction_policy: EvictionPolicy::LRU,
                    prefetch_depth: 0,
                },
                circuit_breaker_settings: CircuitBreakerConfig {
                    entropy_threshold: 0.5,
                    max_repetition_count: 3,
                    causal_violation_tolerance: 1,
                },
                priority,
            },
            status: JobStatus::Preempted,
            submitted_at: std::time::SystemTime::now(),
            started_at: None,
            attention_weight: 0.5,
        }
    }

    #[test]
    fn context_buffer_creation() {
        let buf = ContextBuffer::new(1000);
        assert_eq!(buf.max_tokens, 1000);
        assert!(!buf.pinned);
    }

    #[test]
    fn context_buffer_pin_threshold() {
        let mut optimizer = KVCacheOptimizer::default();
        let job_id = Uuid::new_v4();
        
        optimizer.prepare_buffer(job_id, 1000);
        
        // Access 9 times - should not pin
        for _ in 0..9 {
            optimizer.access_buffer(&job_id).unwrap().record_access();
        }
        assert!(!optimizer.access_buffer(&job_id).unwrap().pinned);
        
        // Access 1 more time - should pin
        optimizer.access_buffer(&job_id).unwrap().record_access();
        assert!(optimizer.check_pin(&job_id));
    }

    #[test]
    fn swap_stats_empty() {
        let optimizer = KVCacheOptimizer::default();
        let stats = optimizer.swap_stats();
        assert_eq!(stats.total_swaps, 0);
        assert_eq!(stats.avg_latency_us, 0.0);
    }
}