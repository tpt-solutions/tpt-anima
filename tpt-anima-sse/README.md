# tpt-anima-sse

**Semantic State Engine (SSE)** — the memory substrate of [TPT Anima](https://github.com/tpt-solutions/tpt-anima).

> *Memory is attended, not queried.*

The SSE stores data as unified **semantic tensors** with native temporal dynamics,
causal topology, and multi-modal sensory ingestion. Memories decay organically,
reinforce on access, and are linked by causal chains — not flat indices. Agents
attend to memory through `mcp://` resources rather than issuing SQL-style queries.

## Key concepts

- **MemoryNode** — a semantic tensor carrying a vector embedding, payload, sensory
  modality, and causal links to other nodes.
- **Decay & Reinforcement** — node strength decays exponentially over time and is
  reinforced each time the node is accessed (`record_access`).
- **Causal Graph** — explicit `CausalLink` edges between nodes model why a memory
  exists, enabling causal reasoning and pruning.
- **Sensory Ingestion** — multi-modal inputs (text, image, audio, abstract) enter
  through the `SensoryManager` and become normalized nodes.
- **Compute Ledger** — an economic record of the cost of storing/accessing memory.

## Installation

Not yet published to crates.io — add it as a git or path dependency:

```toml
[dependencies]
tpt-anima-sse = { git = "https://github.com/tpt-solutions/tpt-anima" }
```

## Quick start

```rust
use std::time::Duration;
use tpt_anima_sse::{InMemoryStorage, MemoryNode, SensoryModality, VectorEmbedding};

let mut storage = InMemoryStorage::new();
let mut node = MemoryNode::new(
    [1u8; 32],
    VectorEmbedding::new(vec![0.12, 0.44, -0.08, 0.91]),
    b"The capital of France is Paris.".to_vec(),
    "text/plain".into(),
    SensoryModality::Textual,
    [9u8; 32],
);

// Strength decays exponentially...
node.apply_decay_for(Duration::from_secs(24 * 3600));
// ...and is reinforced when accessed again.
node.record_access([9u8; 32]);

storage.write_node(node);
assert_eq!(storage.node_count(), 1);
```

## Modules

| Module | Purpose |
|--------|---------|
| `memory_node` | `MemoryNode`, `VectorEmbedding`, `SensoryModality`, IDs, errors |
| `storage` | `InMemoryStorage` — the in-process memory store |
| `decay` | Temporal decay, `CollectionStats`, `MemoryType` |
| `causal_graph` | `CausalGraph` + `CausalLink` topology |
| `sensory` | `SensoryManager` multi-modal ingestion |
| `ledger` | `Ledger`, `CostRates`, `CostBreakdown` |
| `mcp` | `mcp://` resource + tool adapters for agent attendance |

## Runnable example

```bash
cargo run -p tpt-anima-sse --example memory_lifecycle
```

Walks through ingest → decay → reinforce → prune of a single memory node and prints
the resulting strength curve.

## Related crates

- `tpt-anima-inp` — Intent & Negotiation Protocol (the nervous system)
- `tpt-anima-ck` — Cognitive Kernel (the brainstem)
- `tpt-anima-zkpa` — Zero-Knowledge Proof of Agency (the immune system)

## License

Dual-licensed under the [MIT License](../../LICENSE-MIT) or the [Apache License 2.0](../../LICENSE-APACHE).
