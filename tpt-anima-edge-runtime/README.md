# tpt-anima-edge-runtime

**Tier 3 Edge Runtime** — lightweight SSE + INP for phones, IoT, and browsers, part of
[TPT Anima](https://github.com/PhillipC05/tpt-anima).

The Edge Runtime brings the Anima stack to resource-constrained devices. It provides a
minimal memory store and intent handler coordinated by a small runtime, plus the
**cross-tier communication protocol** used to route messages between Core Hypervisor,
Co-Processor, and Edge Runtime nodes.

## Key concepts

- **EdgeMemory** — a minimal, embedded-friendly memory store (`memory`).
- **EdgeIntentHandler** — handles intents on-device (`intent`).
- **EdgeRuntime** — coordinates memory + intent handling and exposes `RuntimeStats`.
- **Cross-tier protocol** — `cross_tier` defines envelopes, handshakes, heartbeats, and
  routing (`Tier`, `TierRole`, `TierRoute`) for inter-tier messaging. Suitable for
  WebAssembly or a small native binary.

## Installation

```toml
[dependencies]
tpt-anima-edge-runtime = { git = "https://github.com/PhillipC05/tpt-anima" }
```

## Quick start

```rust
use tpt_anima_edge_runtime::{
    EdgeMemory, EdgeIntent, EdgeIntentHandler, EdgeRuntime, RuntimeStats,
};

let mut runtime = EdgeRuntime::new(EdgeMemory::new());
let intent = EdgeIntent::new("sense".into(), serde_json::json!({ "pin": 4 }));
let result = runtime.handle(intent);
assert!(result.is_ok());
```

## Modules

| Module | Purpose |
|--------|---------|
| `memory` | `EdgeMemory`, `MemoryError` minimal store |
| `intent` | `EdgeIntentHandler`, `EdgeIntent`, `EdgeError` |
| `runtime` | `EdgeRuntime`, `RuntimeStats` coordinator |
| `cross_tier` | `CrossTierEnvelope`, `Handshake`, `TierRoute`, heartbeats, routing |

## Related crates

- `tpt-anima-sse` / `tpt-anima-inp` — the full-strength layers this runtime condenses.
- `tpt-anima-core-hypervisor` / `tpt-anima-co-processor` — tiers it talks to via
  `cross_tier`.

## License

Apache License 2.0 — see [LICENSE](../../LICENSE).
