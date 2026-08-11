# tpt-anima-co-processor

**Tier 2 Co-Processor** — eBPF kernel module (Linux) + Windows driver for AI workload
interception, part of [TPT Anima](https://github.com/PhillipC05/tpt-anima).

The Co-Processor runs *alongside* an existing operating system and intercepts AI
workloads as they happen — converting them into native Anima intents and forwarding
them to the Tier 1 hypervisor or Tier 3 edge runtime. It hooks the Linux kernel via
**eBPF** and Windows/macOS via a **kernel driver**, without requiring a bare-metal
install.

## Key concepts

- **Workload interception** — `WorkloadIntercept` captures an in-flight AI workload
  (`WorkloadType`) and converts it to an Anima intent via `to_intent()`.
- **CoProcessor** — top-level coordinator that records and manages intercepts.
- **Config** — `CoProcessorConfig` controls enabled patterns and behavior.
- **Cross-tier forwarding** — intercepted work is translated and routed to the
  hypervisor or edge runtime.

> The eBPF bytecode generators in `bpf` are pure data/encoding logic and build on every
> platform; only the actual kernel *load* path (in `driver`) is gated to Linux.

## Installation

```toml
[dependencies]
tpt-anima-co-processor = { git = "https://github.com/PhillipC05/tpt-anima" }
```

## Quick start

```rust
use tpt_anima_co_processor::{CoProcessor, CoProcessorConfig, WorkloadIntercept, WorkloadType};

let mut co = CoProcessor::new(CoProcessorConfig::default())?;
let intercept = WorkloadIntercept::new(
    WorkloadType::Inference,
    1234,
    "test-model".into(),
    serde_json::json!({ "model_id": "gpt-4" }),
);
co.record_intercept(intercept);
assert!(!co.recent_intercepts(10).is_empty());
```

## Modules

| Module | Purpose |
|--------|---------|
| `driver` | `CoProcessorConfig`, `CoProcessorError`, `WorkloadIntercept`, `WorkloadType` |
| `bpf` | pure eBPF bytecode generation/encoding (platform-independent) |

## Related crates

- `tpt-anima-sse` / `tpt-anima-inp` / `tpt-anima-edge-runtime` — targets the
  Co-Processor forwards intercepted work to.

## License

Apache License 2.0 — see [LICENSE](../../LICENSE).
