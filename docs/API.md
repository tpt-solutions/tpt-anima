# TPT Anima API Documentation

> Copyright TPT Solutions · Apache License 2.0

## Overview

TPT Anima provides four core crates that implement a living, probabilistic software stack:

| Crate | Layer | Description |
|-------|-------|-------------|
| `anima-sse` | Semantic State Engine | Memory as semantic tensors with temporal dynamics |
| `anima-inp` | Intent & Negotiation Protocol | Agent-to-agent goal coordination |
| `anima-ck` | Cognitive Kernel | Inference-based compute scheduling |
| `anima-zkpa` | Zero-Knowledge Proof of Agency | Ephemeral cryptographic identity |

---

## SSE API — Semantic State Engine

### MemoryNode

```rust
use anima_sse::memory_node::{MemoryNode, SensoryModality, VectorEmbedding};

// Create a memory node with embedding
let node = MemoryNode::new(
    node_id: [u8; 32],
    embedding: VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
    payload: b"Important fact".to_vec(),
    content_type: "text/plain".to_string(),
    modality: SensoryModality::Textual,
    owner_agent: agent_id,
);
```

### Storage

```rust
use anima_sse::storage::InMemoryStorage;

let mut storage = InMemoryStorage::new();
storage.write_node(node);
let retrieved = storage.read_node(&node_id)?;
let nodes = storage.attention_retrieve(10); // Returns top 10 by strength
storage.prune_pass(); // Remove nodes below pruning threshold
```

### Causal Graph

```rust
use anima_sse::causal_graph::CausalGraph;

let mut graph = CausalGraph::new();
graph.add_link(node_a_id, node_b_id, CausalLinkType::Caused)?;
let chain = graph.get_context_chain(node_id)?;
```

### Decay Engine

```rust
use anima_sse::decay::{apply_decay, prune_collection};
use std::time::Duration;

apply_decay(&mut node, Duration::from_secs(3600));
let pruned_count = prune_collection(&mut nodes);
```

---

## INP API — Intent & Negotiation Protocol

### Intent

```rust
use anima_inp::intent::{Intent, QoSConstraints, EconomicTerms};

let intent = Intent::new(
    action: "analyze".to_string(),
    payload: json!({"data": "input"}),
    qos: QoSConstraints {
        max_latency: Duration::from_millis(500),
        min_accuracy: 0.95,
        ..Default::default()
    },
    economic: EconomicTerms::FixedPrice { credits: 10.0 },
    origin_agent: agent_id,
);
```

### Capability Registry

```rust
use anima_inp::registry::CapabilityRegistry;

let mut registry = CapabilityRegistry::new();
let cap_id = registry.register(Capability::new(
    action_types: vec!["compute".to_string()],
    performance: PerformanceProfile::default(),
    pricing: EconomicTerms::PerUnit { credits_per_unit: 0.01 },
    provider: agent_id,
));

let providers = registry.lookup_by_action("compute");
```

### Negotiation

```rust
use anima_inp::negotiation::NegotiationManager;

let mut manager = NegotiationManager::default();
let intent_id = manager.start_negotiation(intent, &registry);
// Bids are collected automatically via broadcast
let (completed_id, winner) = manager.finalize_and_select(&intent_id)?;
```

### Compute Ledger

```rust
use anima_inp::ledger::ComputeLedger;

let mut ledger = ComputeLedger::new();
ledger.ensure_agent(agent_a);
ledger.ensure_agent(agent_b);

let transaction = ledger.transfer(
    agent_a,
    agent_b,
    ComputeCredits::new(50.0),
    intent_id,
    "inference execution".to_string(),
)?;
```

---

## CK API — Cognitive Kernel

### Inference

```rust
use anima_ck::{InferenceScheduler, InferencePriority, make_ck_inference};

let mut scheduler = InferenceScheduler::new();
let inference = make_ck_inference(
    priority: InferencePriority::High,
    cache_tokens: 8192,
);
scheduler.submit(inference, attention_weight: 0.7);
let job = scheduler.schedule_next()?;
```

### Circuit Breaker

```rust
use anima_ck::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

let breaker = CircuitBreaker::new(CircuitBreakerConfig {
    entropy_threshold: 0.8,
    max_repetition_count: 5,
    ..Default::default()
});

breaker.check_entropy(&inference_output)?;
breaker.detect_repetition(inference_id, current_path)?;
```

### Memory Fabric

```rust
use anima_ck::memory_fabric::MemoryFabric;

let fabric = MemoryFabric::new();
fabric.page_in(token_sequence)?;
fabric.prefetch_context(prediction: &AccessPattern)?;
```

---

## ZKPA API — Zero-Knowledge Proof of Agency

### Session Management

```rust
use anima_zkpa::session::SessionManager;
use std::time::Duration;

let mut sessions = SessionManager::new();
let session = sessions.create_session(
    agent_id,
    "task-scope".to_string(),
    compute_budget: 1000.0,
    Duration::from_secs(3600),
    hardware_binding: None,
);

sessions.validate_session(session.id)?;
```

### Proof Generation

```rust
use anima_zkpa::proofs::ProofGenerator;
use anima_zkpa::types::ZKCircuit;

let gen = ProofGenerator::new();
let circuit = ZKCircuit::new("code_integrity", ProofSystem::PLONK);

let proof = gen.generate_code_integrity_proof(
    &code_hash,
    &circuit
)?;
```

### Attestation

```rust
use anima_zkpa::attestation;

// Platform-specific attestation (TPM, TrustZone, SGX, etc.)
let attestation = attestation::attest_platform(Platform::TPM)?;
```

### Agency Ledger

```rust
use anima_zkpa::ledger::AgencyLedger;
use anima_zkpa::types::LedgerEventType;

let mut ledger = AgencyLedger::new();
ledger.append(
    LedgerEventType::SessionCreated,
    agent_id,
    "session created".to_string()
)?;

let json = ledger.export_format("json")?;
let soc2_report = ledger.generate_compliance_report("SOC2")?;
```

---

## Cross-Tier Communication

### Tier Enumeration

```rust
use anima_edge_runtime::cross_tier::{Tier, TierCapabilities, TierRole};

// Tier::CoreHypervisor    - Bare-metal full stack
// Tier::CoProcessor       - eBPF/kernel driver interception
// Tier::EdgeRuntime       - Wasm/native for phones/IoT
```

### Cross-Tier Envelope

```rust
use anima_edge_runtime::cross_tier::{
    EnvelopeBuilder, CrossTierPayload, serialize_envelope, deserialize_envelope,
};

let envelope = EnvelopeBuilder::new(
    node_id: Uuid::new_v4(),
    origin_tier: Tier::EdgeRuntime,
    destination_tier: Tier::CoreHypervisor,
    payload: CrossTierPayload::IntentForward(IntentMessage { ... }),
).ttl(5).build();

let bytes = serialize_envelope(&envelope)?;
let recovered = deserialize_envelope(&bytes)?;
```

---

## Observatory API

### Telemetry Collection

```rust
use anima_observatory::{TelemetryCollector, TelemetryEvent, SwarmMapData, MemoryTopographyData};

let mut collector = TelemetryCollector::new();
collector.record(TelemetryEvent::SSE(SseEvent::MemoryAccessed {
    node_id: "node-1".into(),
    strength: 0.85,
}));

let counts = collector.count_by_layer();
let recent = collector.recent_events(100);
```

---

## MCP Integration

All four layers expose MCP resources:

| Layer | MCP URI | Access Method |
|-------|---------|---------------|
| SSE | `mcp://resources` | Agents attend to memory nodes |
| INP | `mcp://intent_tools` | Intent broadcasting and bid query |
| CK | `mcp://compute_resources` | Compute allocation as tools |
| ZKPA | `mcp://security_contexts` | Background authentication |

---</

## Error Handling

All crates use `thiserror` for error types. Common patterns:

```rust
match result {
    Ok(value) => value,
    Err(e) => {
        eprintln!("Error: {}", e);
        // Handle specific error variants
        match e.downcast_ref::<SseError>() {
            Some(SseError::NodeNotFound(id)) => { /* handle missing node */ }
            _ => { /* generic error handling */ }
        }
    }
}
```

---

## Security Considerations

- **ThoughtBubbles** provide cryptographic isolation
- **SessionCredentials** are time-limited and task-scoped
- **ZK proofs** demonstrate compliance without exposure
- **Circuit breakers** halt compromised inferences

See `docs/ZKPA.md` for detailed security architecture.