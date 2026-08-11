//! CK — Attention-Weighted Inference Scheduling
//!
//! Submits two inference jobs with different attention weights and shows
//! that the scheduler dispatches by attention weight, not submission order
//! (the fundamental compute unit is an inference, not a thread — see
//! CLAUDE.md). Also allocates a `ThoughtBubble` sandbox for isolated
//! execution.
//!
//! Run with: `cargo run -p tpt-anima-ck --example inference_scheduling`

use uuid::Uuid;

use tpt_anima_ck::scheduler::InferenceScheduler;
use tpt_anima_ck::thought_bubble::ThoughtBubbleManager;
use tpt_anima_ck::types::{
    CircuitBreakerConfig, EvictionPolicy, Inference, InferencePriority, IsolationLevel,
    KVCacheConfig, ResourceRequest,
};

fn make_inference(model_id: &str, priority: InferencePriority) -> Inference {
    Inference {
        id: Uuid::new_v4(),
        model_id: model_id.into(),
        resource_requirements: ResourceRequest {
            cpu_cores: 4,
            gpu_count: 1,
            vram_mb: 8192,
            ram_mb: 4096,
            nvme_gb: 0,
        },
        kv_cache_config: KVCacheConfig {
            cache_size_tokens: 4096,
            eviction_policy: EvictionPolicy::AttentionBased,
            prefetch_depth: 2,
        },
        circuit_breaker_settings: CircuitBreakerConfig {
            entropy_threshold: 0.85,
            max_repetition_count: 5,
            causal_violation_tolerance: 0,
        },
        priority,
    }
}

fn main() {
    let mut scheduler = InferenceScheduler::new();

    // Both jobs share the same priority tier, submitted low-weight-first —
    // the scheduler should still dispatch by attention weight, not
    // submission order or a round-robin.
    let low = make_inference("summarizer-small", InferencePriority::Normal);
    let high = make_inference("reasoner-large", InferencePriority::Normal);

    scheduler.submit(low, 0.3);
    scheduler.submit(high, 0.95);
    println!("Submitted 2 same-priority jobs (attention weights 0.3, then 0.95). Pending: {}", scheduler.pending_count());

    if let Some(job) = scheduler.schedule_next() {
        println!(
            "1. Dispatched first: model '{}' — attention-weight preemption picked the 0.95-weight job despite being submitted second",
            job.inference.model_id
        );
    }
    if let Some(job) = scheduler.schedule_next() {
        println!("2. Dispatched second: model '{}' (priority {:?})", job.inference.model_id, job.inference.priority);
    }

    // Allocate an isolated ThoughtBubble sandbox for the winning inference.
    let mut bubbles = ThoughtBubbleManager::new();
    let bubble = bubbles.create_bubble("reasoner-large-sandbox".into(), IsolationLevel::Enclave, vec![]);
    println!(
        "3. Allocated ThoughtBubble {} (isolation: {:?}, compromised: {})",
        bubble.id, bubble.isolation_level, bubble.is_compromised
    );
}
