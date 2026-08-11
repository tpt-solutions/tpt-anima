# TPT Anima

**Codename:** Anima (Latin: soul, breath, life)
**License:** Apache 2.0 · Copyright 2026 TPT Solutions

TPT Anima is a zero-legacy, four-layer computational stack rebuilt from first principles for the era of autonomous AI. It abandons deterministic, human-centric paradigms (REST APIs, SQL databases, OAuth, thread-based OSes) in favour of a living, probabilistic, and self-defending software environment.

Anima natively implements the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol/specification) and [Agent-to-Agent (A2A)](https://github.com/google-deepmind/a2a) standards as its primary nervous system, while keeping its internal substrate philosophy-free of HTTP and JSON.

---

## Use it in your project

Not yet published to crates.io — add it as a git dependency:

```toml
[dependencies]
tpt-anima-sse  = { git = "https://github.com/PhillipC05/tpt-anima" }
tpt-anima-inp  = { git = "https://github.com/PhillipC05/tpt-anima" }
tpt-anima-ck   = { git = "https://github.com/PhillipC05/tpt-anima" }
tpt-anima-zkpa = { git = "https://github.com/PhillipC05/tpt-anima" }
```

The smallest possible example — ingest a memory into the Semantic State Engine and watch it decay and reinforce:

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

node.apply_decay_for(Duration::from_secs(24 * 3600)); // strength decays exponentially
node.record_access([9u8; 32]);                        // ...and reinforces on access

storage.write_node(node);
```

### Runnable examples

Each core crate ships a self-contained, working example:

| Crate | Command | What it shows |
|-------|---------|----------------|
| `tpt-anima-sse` | `cargo run -p tpt-anima-sse --example memory_lifecycle` | Ingest → decay → reinforce → prune a memory node |
| `tpt-anima-inp` | `cargo run -p tpt-anima-inp --example negotiation_swarm` | Broadcast an intent, collect competing bids, transfer compute credits |
| `tpt-anima-ck` | `cargo run -p tpt-anima-ck --example inference_scheduling` | Attention-weight preemption in the inference scheduler + a `ThoughtBubble` sandbox |
| `tpt-anima-zkpa` | `cargo run -p tpt-anima-zkpa --example session_and_attestation` | Ephemeral session, real ZK proof generation/verification, hash-chained liveness |
| `tpt-anima-observatory` | `cargo run -p tpt-anima-observatory --example full_stack_quickstart` | Golden path across all four layers in one request: memory → negotiation → session → thermal budget check |

---

## Getting Started

**Is there a GUI?** Yes — the **Anima Observatory** (see below), a real-time dashboard that visualizes what the four layers are doing.

The fastest way to see Anima in action:

```bash
# Terminal 1: start the Observatory telemetry backend
cd tpt-anima-observatory
cargo run

# Terminal 2: start the Observatory UI
cd observatory-ui
npm install
npm run dev
```

Then open `http://localhost:5173`. No agents to configure — the demo backend emits sample telemetry across all five panels.

To build and test the rest of the workspace: `cargo build --workspace` / `cargo test --workspace`.

For the full walkthrough — deployment tiers, embedding Anima's crates in your own Rust project, and a golden-path tour of memory, negotiation, and observability — see [docs/ONBOARDING.md](docs/ONBOARDING.md).

---

## The Four Layers

| Layer | Crate | Replaces | Tagline |
|-------|-------|----------|---------|
| **Semantic State Engine (SSE)** | `tpt-anima-sse` | Relational DBs, Vector DBs, Key-Value Stores | *Memory is attended, not queried* |
| **Intent & Negotiation Protocol (INP)** | `tpt-anima-inp` | REST, gRPC, GraphQL, Message Queues | *Agents pursue outcomes, not endpoints* |
| **Zero-Knowledge Proof of Agency (ZKPA)** | `tpt-anima-zkpa` | OAuth, API Keys, IAM, Certificate Authorities | *Identity is ephemeral and provable* |
| **Cognitive Kernel (CK)** | `tpt-anima-ck` | Linux, Windows, thread schedulers, container runtimes | *The unit of compute is an inference* |

### Supporting Crates

| Crate | Role |
|-------|------|
| `tpt-anima-observatory` | Real-time human telemetry window — translates machine-speed cognition into observable, auditable visualisations |
| `tpt-anima-edge-gateway` | Lightweight translation layer: native INP ↔ MCP/A2A JSON, preserving internal philosophical purity |

---

## Layer Summaries

### Layer 1 — Semantic State Engine (SSE)
The memory substrate. Stores data as unified semantic tensors with native temporal dynamics, causal topology, and multi-modal sensory ingestion. Memories decay organically, reinforce on access, and are linked by causal chains — not flat indices. Exposed to AI agents as `mcp://resources` so agents *attend* rather than query.

### Layer 2 — Intent & Negotiation Protocol (INP)
The nervous system. Agents broadcast goal descriptors; capability providers bid; a state-machine protocol routes intents to the optimal provider with automatic fallback chains. A native micro-economy (Compute Ledger) tracks credits between agents. Natively speaks A2A for cross-network agent collaboration.

### Layer 3 — Zero-Knowledge Proof of Agency (ZKPA)
The immune system. Agents receive cryptographically bound, time-limited identities tied to specific tasks. ZK proofs demonstrate code integrity, policy compliance, and hardware attestation without exposing weights or source. Parent agents spawn child agents with mathematically scoped permissions. Continuous liveness heartbeats form a tamper-evident hash chain.

### Layer 4 — Cognitive Kernel (CK)
The brainstem. The fundamental compute unit is an *inference*, not a thread. The CK manages attention, context, and tensor execution natively — blurring VRAM, RAM, and NVMe into a single addressable context space. Cognitive Circuit Breakers halt inferences that violate causal integrity or enter repetition loops, providing safety at the silicon level.

---

## The Anima Observatory

This is Anima's GUI — see [Getting Started](#getting-started) above to run it locally. It's a React/Vite frontend (`observatory-ui/`) fed by a WebSocket telemetry server (`tpt-anima-observatory/`), and translates machine-speed AI cognition into human-speed telemetry:

- **Swarm Map** — real-time node-graph of INP intent pulses and negotiation outcomes
- **Memory Topography** — 2D/3D cluster view of SSE nodes glowing and fading with temporal decay
- **Inference Pulse** — hardware telemetry dashboard for CK scheduling (GPU/CPU/NPU load, VRAM, FLOPs)
- **Agency Ledger** — immutable ZK audit log for SOC2/HIPAA/FedRAMP compliance
- **Economic Flow** — real-time visualisation of compute-credit transfers between agents

---

## Hardware Deployment Tiers

| Tier | Target | Mechanism |
|------|--------|-----------|
| **Anima Core** | Servers, power users | Type-1 bare-metal hypervisor; legacy OSes run as isolated VMs |
| **Anima Co-Processor** | Everyday laptops/desktops | eBPF kernel module (Linux) or driver (Windows/macOS); intercepts AI workloads |
| **Anima Edge** | Phones, IoT, browsers | WebAssembly or native Rust binary; full SSE + INP without hardware control |

A **JIT Hardware-Fluid Engine** detects the available silicon (Apple ANE, NVIDIA CUDA, AMD ROCm, ARM NEON, x86 AVX) and reshapes scheduling logic at install time.

---

## Development Roadmap

| Phase | Months | Focus |
|-------|--------|-------|
| 0 | — | Project scaffold (this milestone) |
| 1 | 1–3 | SSE: memory nodes, temporal decay, causal graph, sensory ingestion |
| 2 | 4–6 | INP: intent broadcasting, negotiation state machine, Compute Ledger, Edge Gateway |
| 3 | 7–9 | CK: inference scheduler, circuit breakers, unified memory fabric; ZKPA |
| 4 | 10–12 | Observatory UI, three-tier deployment, beta release |
| 5 | 13–16 | Production hardening, compliance certification, v1.0 |

---

## Documentation

| Doc | Purpose |
|-----|---------|
| [docs/API.md](docs/API.md) | API reference across all four layers |
| [docs/ONBOARDING.md](docs/ONBOARDING.md) | Beta onboarding guide |
| [docs/INCIDENT_RESPONSE.md](docs/INCIDENT_RESPONSE.md) | Detection signals, severity classification, and response procedure per layer |
| [docs/compliance/SOC2.md](docs/compliance/SOC2.md) | SOC 2 Type II control mapping and certification prep |
| [docs/compliance/HIPAA.md](docs/compliance/HIPAA.md) | HIPAA technical-safeguards mapping and deployer responsibilities |
| [docs/compliance/FEDRAMP.md](docs/compliance/FEDRAMP.md) | FedRAMP baseline/control mapping and authorization prep notes |

None of the compliance documents constitute an actual certification —
SOC 2, HIPAA, and FedRAMP all require independent third-party
assessment/audit of a live deployment, which is outside what a codebase can
self-certify.

---

## License & Attribution

Apache License 2.0 — see [LICENSE](LICENSE).

This project implements the *semantics* of MCP (MIT, Anthropic) and A2A (Apache 2.0, Google). No source code from those projects is copied; only their conceptual interfaces are implemented from scratch in Rust. Full attribution is included in the LICENSE file.
