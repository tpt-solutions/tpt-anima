//! Cognitive Kernel — the brainstem of TPT Anima.
//!
//! The fundamental compute unit is an *inference*, not a thread.
//! The CK manages attention, context, and tensor execution natively.
//! Phase 3 implementation lives in sub-modules added during Months 7–9.
//!
//! ```
//! use uuid::Uuid;
//! use tpt_anima_ck::scheduler::InferenceScheduler;
//! use tpt_anima_ck::types::{
//!     CircuitBreakerConfig, EvictionPolicy, Inference, InferencePriority, KVCacheConfig,
//!     ResourceRequest,
//! };
//!
//! fn make_inference(model_id: &str) -> Inference {
//!     Inference {
//!         id: Uuid::new_v4(),
//!         model_id: model_id.into(),
//!         resource_requirements: ResourceRequest {
//!             cpu_cores: 4,
//!             gpu_count: 1,
//!             vram_mb: 8192,
//!             ram_mb: 4096,
//!             nvme_gb: 0,
//!         },
//!         kv_cache_config: KVCacheConfig {
//!             cache_size_tokens: 4096,
//!             eviction_policy: EvictionPolicy::AttentionBased,
//!             prefetch_depth: 2,
//!         },
//!         circuit_breaker_settings: CircuitBreakerConfig {
//!             entropy_threshold: 0.85,
//!             max_repetition_count: 5,
//!             causal_violation_tolerance: 0,
//!         },
//!         priority: InferencePriority::Normal,
//!     }
//! }
//!
//! let mut scheduler = InferenceScheduler::new();
//! scheduler.submit(make_inference("summarizer-small"), 0.3);
//! scheduler.submit(make_inference("reasoner-large"), 0.95);
//!
//! // Attention-weight preemption: the 0.95-weight job dispatches first,
//! // even though it was submitted second.
//! let first = scheduler.schedule_next().unwrap();
//! assert_eq!(first.inference.model_id, "reasoner-large");
//! ```
//!
//! See `examples/inference_scheduling.rs` for the full walkthrough including
//! `ThoughtBubble` sandbox allocation.

pub mod budgeting;
pub mod circuit_breaker;
pub mod jit;
pub mod kv_swap_optimization;
pub mod mcp;
pub mod memory_fabric;
pub mod scheduler;
pub mod thought_bubble;
pub mod types;

pub use budgeting::{BudgetError, BudgetManager, ComputeBudget, ComputeUnit, ThermalStatus};
pub use kv_swap_optimization::{
    ContextBuffer, KVCacheOptimizer, SwapOperation, SwapPlan, SwapPriority, SwapStatistics,
    SwapStrategy,
};
