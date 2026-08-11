# tpt-anima-ck

**Cognitive Kernel (CK)** — the brainstem of
[TPT Anima](https://github.com/tpt-solutions/tpt-anima).

> *The unit of compute is an inference.*

The CK manages attention, context, and tensor execution natively. The fundamental
compute unit is an **inference**, not a thread. It blurs VRAM, RAM, and NVMe into a
single addressable context space, schedules inferences by attention weight, and halts
unsafe work via **Cognitive Circuit Breakers**.

## Key concepts

- **Inference scheduling** — `InferenceScheduler` dispatches jobs by attention weight
  (`InferencePriority`), preempting lower-weight work.
- **Circuit breakers** — `circuit_breaker` halts inferences that violate causal
  integrity or enter repetition loops.
- **Unified memory fabric** — `memory_fabric` addresses VRAM/RAM/NVMe as one space;
  `kv_swap_optimization` manages KV-cache swapping.
- **Thermal budgeting** — `budgeting` enforces `ComputeBudget` / `ThermalStatus`
  limits across the silicon.
- **Thought bubbles** — `thought_bubble` sandboxes untrusted inferences.
- **JIT hardware detection** — `jit` reshapes scheduling to the available silicon
  (CUDA, ROCm, NEON, AVX).

## Installation

```toml
[dependencies]
tpt-anima-ck = { git = "https://github.com/tpt-solutions/tpt-anima" }
```

## Quick start

```rust
use uuid::Uuid;
use tpt_anima_ck::scheduler::InferenceScheduler;
use tpt_anima_ck::types::{
    CircuitBreakerConfig, EvictionPolicy, Inference, InferencePriority, KVCacheConfig,
    ResourceRequest,
};

fn make_inference(model_id: &str) -> Inference {
    Inference {
        id: Uuid::new_v4(),
        model_id: model_id.into(),
        resource_requirements: ResourceRequest {
            cpu_cores: 4, gpu_count: 1, vram_mb: 8192, ram_mb: 4096, nvme_gb: 0,
        },
        kv_cache_config: KVCacheConfig {
            cache_size_tokens: 4096,
            eviction_policy: EvictionPolicy::AttentionBased,
            prefetch_depth: 2,
        },
        circuit_breaker_settings: CircuitBreakerConfig {
            entropy_threshold: 0.85, max_repetition_count: 5, causal_violation_tolerance: 0,
        },
        priority: InferencePriority::Normal,
    }
}

let mut scheduler = InferenceScheduler::new();
scheduler.submit(make_inference("summarizer-small"), 0.3);
scheduler.submit(make_inference("reasoner-large"), 0.95);

// Attention-weight preemption: the 0.95-weight job dispatches first.
let first = scheduler.schedule_next().unwrap();
assert_eq!(first.inference.model_id, "reasoner-large");
```

## Modules

| Module | Purpose |
|--------|---------|
| `scheduler` | `InferenceScheduler` attention-weighted dispatch |
| `types` | `Inference`, `ResourceRequest`, `KVCacheConfig`, circuit-breaker config |
| `circuit_breaker` | causal/repetition safety halts |
| `memory_fabric` | unified VRAM/RAM/NVMe address space |
| `kv_swap_optimization` | KV-cache swap planning |
| `budgeting` | compute/thermal budgeting |
| `thought_bubble` | sandboxed inference execution |
| `jit` | hardware-fluid detection & tuning |
| `mcp` | `mcp://` scheduler/tools adapters |

## Runnable example

```bash
cargo run -p tpt-anima-ck --example inference_scheduling
```

Demonstrates attention-weight preemption plus a `ThoughtBubble` sandbox allocation.

## Related crates

- `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-zkpa` — the other three Anima layers.

## License

Dual-licensed under the [MIT License](../../LICENSE-MIT) or the [Apache License 2.0](../../LICENSE-APACHE).
