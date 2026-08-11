# tpt-anima-inp

**Intent & Negotiation Protocol (INP)** — the nervous system of
[TPT Anima](https://github.com/tpt-solutions/tpt-anima).

> *Agents pursue outcomes, not endpoints.*

Agents broadcast **goal descriptors**; capability providers **bid**; a state-machine
protocol routes intents to the optimal provider with automatic fallback chains. A
native micro-economy (the Compute Ledger) tracks credits between agents, and the
protocol speaks **A2A** for cross-network collaboration and exposes **MCP** tools for
agent attendance.

## Key concepts

- **Intent** — a declarative goal (`action`, payload, `QoSConstraints`, `EconomicTerms`)
  rather than a concrete endpoint call.
- **Capability & Bid** — providers register `Capability` descriptors and submit
  `CapabilityBid`s; the `NegotiationManager` selects winners.
- **Negotiation state machine** — `NegotiationSession` progresses through
  `NegotiationState`s with `FallbackStrategy`/`FallbackTrigger` chains.
- **Compute Ledger** — `ComputeLedger` moves `ComputeCredits` between agents via
  `Transaction`s, with optional `ExternalRails` settlement.
- **Consensus & reputation** — `ConsensusManager`, `RateLimiter`, `ReputationManager`,
  and `ProofOfWork`/`EconomicStake` guards against bad actors.
- **Contracts** — `SemanticContract` validation via `ContractValidator`.

## Installation

```toml
[dependencies]
tpt-anima-inp = { git = "https://github.com/tpt-solutions/tpt-anima" }
```

## Quick start

```rust
use std::time::Duration;
use uuid::Uuid;
use tpt_anima_inp::{
    Capability, CapabilityBid, CapabilityRegistry, EconomicTerms, Intent,
    NegotiationSession, PerformanceProfile, QoSConstraints,
};

let requester = Uuid::new_v4();
let provider = Uuid::new_v4();

let mut registry = CapabilityRegistry::new();
let capability_id = registry.register(Capability::new(
    vec!["analyze".into()],
    PerformanceProfile {
        typical_latency: Duration::from_millis(200),
        max_latency: Duration::from_millis(600),
        typical_accuracy: 0.92,
        throughput_per_second: 8.0,
    },
    EconomicTerms::FixedPrice { credits: 100.0 },
    provider,
));

let intent = Intent::new(
    "analyze".into(),
    serde_json::json!({ "task": "Analyze Q3 report" }),
    QoSConstraints::default(),
    EconomicTerms::FixedPrice { credits: 100.0 },
    requester,
);
let intent_id = intent.id;

let mut session = NegotiationSession::new(intent);
session.start_broadcasting();
session.start_collecting();
session.add_bid(CapabilityBid::new(
    intent_id, provider, capability_id, 100.0, Duration::from_millis(200), 0.92, 0.9,
));
assert_eq!(session.bids.len(), 1);
```

## Modules

| Module | Purpose |
|--------|---------|
| `intent` | `Intent`, `Capability`, `CapabilityBid`, QoS/economics types |
| `registry` | `CapabilityRegistry` of provider capabilities |
| `negotiation` | `NegotiationSession` / `NegotiationManager` state machine |
| `broadcast` | `IntentBroadcaster` fan-out |
| `ledger` | `ComputeLedger`, `ComputeCredits`, `Transaction` |
| `consensus` | reputation, rate limiting, PoW/stake |
| `contracts` | `SemanticContract` validation |
| `qos` | adaptive QoS negotiation & recommendations |
| `manifest` | `ManifestParser` for capability manifests |
| `mcp` | `mcp://` broadcast/negotiation/result tools |

## Runnable example

```bash
cargo run -p tpt-anima-inp --example negotiation_swarm
```

Broadcasts an intent, collects competing bids, and performs a real `ComputeLedger`
credit transfer.

## Related crates

- `tpt-anima-sse`, `tpt-anima-ck`, `tpt-anima-zkpa` — the other three Anima layers.

## License

Dual-licensed under the [MIT License](../../LICENSE-MIT) or the [Apache License 2.0](../../LICENSE-APACHE).
