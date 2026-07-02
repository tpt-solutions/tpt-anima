# TPT Anima Beta Onboarding Guide

> Copyright TPT Solutions · Apache License 2.0

Welcome to the TPT Anima beta program! This guide will help you get started with the living, probabilistic software stack for autonomous AI.

---

## What is Anima?

Anima is a four-layer computational stack rebuilt from first principles:

1. **SSE (Semantic State Engine)** — Memory is attended, not queried
2. **INP (Intent & Negotiation Protocol)** — Agents pursue outcomes, not endpoints
3. **CK (Cognitive Kernel)** — The unit of compute is an inference
4. **ZKPA (Zero-Knowledge Proof of Agency)** — Identity is ephemeral and provable

---

## Quick Start: Observatory UI

The fastest way to explore Anima is through the Observatory UI.

### Prerequisites

- Node.js 18+ or npm/pnpm
- Rust 1.80+ with Cargo
- WebSocket-compatible browser (Chrome, Firefox, Edge, Safari)

### Running the Demo

```bash
# Terminal 1: Start the backend
cd anima-observatory
cargo run

# Terminal 2: Start the UI
cd observatory-ui
npm install
npm run dev
```

Open `http://localhost:5173` to view the Observatory.

### UI Components

| Panel | Description |
|-------|-------------|
| **Swarm Map** | Real-time visualization of agent network and intent flows |
| **Memory Topography** | 2D/3D view of memory nodes with decay visualization |
| **Inference Pulse** | Hardware telemetry for GPU/CPU/NPU utilization |
| **Agency Ledger** | Immutable ZK audit log for compliance |
| **Economic Flow** | Real-time compute credit transfers between agents |

---

## Deployment Tiers

Anima supports three deployment tiers:

### Tier 1: Anima Core Hypervisor (Servers, Workstations)

Full hardware control with legacy OS virtualization.

```bash
# Build from source (requires signed binaries for production)
cargo build --release -p anima-core-hypervisor

# Install on Windows
./install-scripts/install-core.ps1

# Install on Linux (bare-metal boot image)
sudo ./install-scripts/install-core.sh
```

### Tier 2: Anima Co-Processor (Laptops, Desktops)

eBPF kernel module on Linux, kernel driver on Windows.

```bash
# Install eBPF co-processor on Linux
sudo ./install-scripts/install-coproc.sh

# The co-processor intercepts AI workloads automatically
# No code changes required in your applications
```

### Tier 3: Anima Edge Runtime (Phones, IoT, Browsers)

WebAssembly and native Rust binaries for resource-constrained devices.

```bash
# Build WASM module
cargo build --target wasm32-wasi -p anima-edge-runtime

# Build native binary
cargo build --release -p anima-edge-runtime

# Install
./install-scripts/install-edge.sh
```

---

## Programming with Anima

### Rust Crate Integration

Add to your `Cargo.toml`:

```toml
[dependencies]
anima-sse = "0.1"
anima-inp = "0.1"
anima-ck = "0.1"
anima-zkpa = "0.1"
uuid = { version = "1", features = ["v4"] }
rand = "0.8"
```

### Example: Creating an AI Agent

Note: SSE's `AgentId`/`MemoryId` are raw `[u8; 32]`, while INP's and ZKPA's `AgentId` is a `Uuid` — this is intentional per-crate type isolation (see `CLAUDE.md`), not an oversight, so the two id kinds below are deliberately separate values.

```rust
use anima_sse::{MemoryNode, VectorEmbedding, SensoryModality};
use anima_inp::{
    Intent, Capability, CapabilityRegistry, QoSConstraints, EconomicTerms, PerformanceProfile,
};
use anima_zkpa::session::SessionManager;
use std::time::Duration;
use uuid::Uuid;

// INP and ZKPA agent identity (Uuid-based)
let agent_id: Uuid = Uuid::new_v4();

// 1. Create a ZKPA session (ephemeral identity)
let mut sessions = SessionManager::default();
let session = sessions.create_session(
    agent_id,
    "my-agent-task".to_string(),
    1000.0, // Compute budget
    Duration::from_secs(3600), // 1 hour TTL
    None, // no hardware attestation binding
);

// 2. Store a memory in SSE (uses SSE's own [u8; 32] id space)
let node_id: anima_sse::MemoryId = rand::random();
let sse_agent_id: anima_sse::AgentId = rand::random();
let memory = MemoryNode::new(
    node_id,
    VectorEmbedding::new(vec![0.5, 0.3, 0.2]),
    b"Important knowledge".to_vec(),
    "text/plain".to_string(),
    SensoryModality::Textual,
    sse_agent_id,
);

// 3. Register a capability with INP
let mut registry = CapabilityRegistry::new();
registry.register(Capability::new(
    vec!["analyze".to_string()],
    PerformanceProfile {
        typical_latency: Duration::from_millis(200),
        max_latency: Duration::from_secs(1),
        typical_accuracy: 0.95,
        throughput_per_second: 10.0,
    },
    EconomicTerms::PerUnit { credits_per_unit: 0.05, unit: "token".to_string() },
    agent_id,
));

// 4. Describe an intent to negotiate over (see docs/API.md for the full
// negotiation flow: BroadcastEngine::start_negotiation -> submit_bid -> finalize_and_select)
let intent = Intent::new(
    "analyze".to_string(),
    serde_json::json!({"data": "input"}),
    QoSConstraints::default(),
    EconomicTerms::FixedPrice { credits: 10.0 },
    agent_id,
);
```

---

## Golden Path Walkthrough

### Step 1: Single-Agent Memory Lifecycle

1. Create a memory node in SSE
2. Query the node (triggers decay reinforcement)
3. Watch the Memory Topography visualization update

### Step 2: Multi-Agent Intent Negotiation

1. Register multiple providers with different capabilities
2. Broadcast an intent from a requester agent
3. Observe bid collection and winner selection in Swarm Map

### Step 3: Observability

1. Watch hardware metrics in Inference Pulse
2. Check audit log in Agency Ledger
3. Monitor credit flows in Economic Flow

---

## Common Patterns

### Memory Attention Pattern

```rust
// Instead of: SELECT * FROM memory WHERE id = ?
// Use: Attend to the most relevant memories
let relevant = storage.attention_retrieve(count: 10);
```

### Intent Negotiation Pattern

```rust
// Instead of: POST /api/endpoint
// Use: Broadcast an intent and let providers compete
let intent = Intent::new(action, payload, qos, economic_terms, agent);
broadcast.send(intent).await?;
let result = await negotiation_result().await?;
```

### Compute-Aware Pattern

```rust
// Check hardware before scheduling large inference
let hw_status = ck.hardware_status();
if hw_status.temperature > 80.0 {
    // Defer or reduce precision
}
```

---

## Beta Testing Checklist

- [ ] Run all three demo scenarios
- [ ] Test cross-tier communication (Edge → Co-Processor → Core)
- [ ] Verify ZKPA session lifecycle and proofs
- [ ] Monitor memory decay in Observatory
- [ ] Test intent negotiation with multiple agents
- [ ] Verify circuit breaker triggers on simulated anomalies
- [ ] Check compliance report generation (SOC2/HIPAA)

---

## Feedback Channels

- **GitHub Issues**: Report bugs and feature requests
- **Discord**: Join `#beta-testing` for real-time support
- **Email**: beta@tptsolutions.io for private feedback

Thank you for participating in the Anima beta program!