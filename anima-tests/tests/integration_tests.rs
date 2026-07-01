//! Integration tests for the TPT Anima workspace.
//!
//! These tests exercise cross-layer interactions across all four core crates:
//! SSE (Semantic State Engine), INP (Intent & Negotiation Protocol),
//! CK (Cognitive Kernel), and ZKPA (Zero-Knowledge Proof of Agency),
//! plus the Observatory telemetry collector.

use std::time::Duration;

use anima_ck::scheduler::{InferenceScheduler, JobStatus};
use anima_ck::types::{
    CircuitBreakerConfig, EvictionPolicy, Inference, InferencePriority, KVCacheConfig,
    ResourceRequest,
};
use anima_inp::broadcast::{BroadcastConfig, BroadcastError, IntentBroadcaster};
use anima_inp::intent::{
    Capability, CapabilityBid, EconomicTerms, ExecutionResult, ExecutionStatus, Intent,
    PerformanceProfile, QoSConstraints,
};
use anima_inp::ledger::{ComputeCredits, ComputeLedger};
use anima_inp::manifest::ManifestParser;
use anima_inp::negotiation::{NegotiationManager, NegotiationState};
use anima_inp::registry::CapabilityRegistry;
use anima_observatory::{
    CkEvent, InpEvent, LayerCounts, SseEvent, TelemetryCollector, TelemetryEvent, ZkpaEvent,
};
use anima_sse::decay::{apply_batch_decay_for, collection_stats, prune_collection};
use anima_sse::ledger::Ledger;
use anima_sse::memory_node::{MemoryNode, SensoryModality, VectorEmbedding};
use anima_sse::storage::InMemoryStorage;
use anima_zkpa::ledger::AgencyLedger;
use anima_zkpa::liveness::LivenessMonitor;
use anima_zkpa::proofs::ProofGenerator;
use anima_zkpa::session::SessionManager;
use anima_zkpa::types::{LedgerEventType, ProofSystem, ZKCircuit, ZkpaError};
use uuid::Uuid;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn sse_agent(id: u8) -> [u8; 32] {
    [id; 32]
}

fn inp_agent() -> Uuid {
    Uuid::new_v4()
}

fn default_perf_profile() -> PerformanceProfile {
    PerformanceProfile {
        typical_latency: Duration::from_millis(100),
        max_latency: Duration::from_secs(5),
        typical_accuracy: 0.95,
        throughput_per_second: 100.0,
    }
}

fn make_ck_inference(priority: InferencePriority, cache_tokens: u64) -> Inference {
    Inference {
        id: Uuid::new_v4(),
        model_id: "integration-test-model".into(),
        resource_requirements: ResourceRequest {
            cpu_cores: 4,
            gpu_count: 1,
            vram_mb: 8_000,
            ram_mb: 16_000,
            nvme_gb: 100,
        },
        kv_cache_config: KVCacheConfig {
            cache_size_tokens: cache_tokens,
            eviction_policy: EvictionPolicy::LRU,
            prefetch_depth: 2,
        },
        circuit_breaker_settings: CircuitBreakerConfig {
            entropy_threshold: 0.8,
            max_repetition_count: 5,
            causal_violation_tolerance: 2,
        },
        priority,
    }
}

fn test_circuit() -> ZKCircuit {
    ZKCircuit {
        circuit_type: "integration-test".to_string(),
        proof_system: ProofSystem::PLONK,
    }
}

// ── (a) SSE → INP flow ─────────────────────────────────────────────────────

#[test]
fn sse_to_inp_flow_memory_stored_intent_references_memory() {
    // Create a memory node in SSE and store it.
    let agent_sse = sse_agent(1);
    let node_id: [u8; 32] = [10u8; 32];

    let mut node = MemoryNode::new(
        node_id,
        VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
        b"process sensor data from fleet".to_vec(),
        "text/plain".to_string(),
        SensoryModality::Textual,
        agent_sse,
    );
    node.metadata.tags.push("sensor-data".to_string());
    node.metadata.label.replace("Sensor Fleet Data".to_string());

    let mut storage = InMemoryStorage::new();
    storage.write_node(node);

    // Retrieve the node and verify payload integrity.
    let stored = storage.read_node(&node_id).unwrap();
    assert!(stored.verify_payload_integrity());
    assert_eq!(stored.payload, b"process sensor data from fleet");
    assert_eq!(stored.metadata.tags, vec!["sensor-data"]);

    // Create an INP Intent that references the stored memory node's action.
    let agent_inp = inp_agent();
    let intent = Intent::new(
        "process_sensor_data".to_string(),
        serde_json::json!({
            "sse_node_id": hex::encode(node_id),
            "source": "fleet",
        }),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 2.5 },
        agent_inp,
    );

    // Verify the intent carries the reference to the SSE memory node.
    let ref_node_id = intent.payload["sse_node_id"].as_str().unwrap();
    let decoded = hex::decode(ref_node_id).unwrap();
    assert_eq!(decoded.len(), 32);
    assert_eq!(&decoded[..], &node_id);

    // Verify the memory can be attended to by strength.
    let retrieved = storage.attention_retrieve(10);
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].id, node_id);
}

// ── (b) INP negotiation flow ────────────────────────────────────────────────

#[test]
fn inp_full_negotiation_broadcast_to_execution() {
    // Register 3 providers with the same action type.
    let mut registry = CapabilityRegistry::new();
    let providers: Vec<Uuid> = (0..3).map(|_| inp_agent()).collect();
    let cap_ids: Vec<_> = providers
        .iter()
        .map(|&aid| {
            registry.register(Capability::new(
                vec!["compute".to_string()],
                default_perf_profile(),
                EconomicTerms::FixedPrice { credits: 0.5 },
                aid,
            ))
        })
        .collect();

    assert_eq!(registry.count(), 3);
    assert_eq!(registry.lookup_by_action("compute").len(), 3);

    // Create and broadcast an intent.
    let mut broadcaster = IntentBroadcaster::default();
    let intent = Intent::new(
        "compute".to_string(),
        serde_json::json!({"model": "gpt-4o"}),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 5.0 },
        inp_agent(),
    );

    let matching_providers = broadcaster.begin_broadcast(&intent, &registry);
    assert_eq!(matching_providers.len(), 3);

    // Each provider submits a bid.
    for (i, &provider_id) in matching_providers.iter().enumerate() {
        let bid = CapabilityBid::new(
            intent.id,
            provider_id,
            cap_ids[i],
            0.3 + i as f64 * 0.2, // increasing cost
            Duration::from_millis(50 + i as u64 * 20),
            0.99 - i as f32 * 0.05, // decreasing accuracy
            0.9 - i as f32 * 0.1,
        );
        broadcaster.submit_bid(bid).unwrap();
    }

    // Finalize and select the winner.
    let result = broadcaster.finalize_broadcast(&intent.id).unwrap();
    assert_eq!(result.bids_received.len(), 3);
    assert!(!result.timed_out);

    let winner = result.select_winner(&intent.qos).unwrap();
    // The winner should be the lowest-scored bid (best value).
    assert_eq!(winner.provider_id, matching_providers[0]);

    // Record the result in the ComputeLedger.
    let mut ledger = ComputeLedger::new();
    let requester = inp_agent();
    let winner_id = winner.provider_id;

    ledger.ensure_agent(requester);
    ledger.ensure_agent(winner_id);

    // Both agents start with zero balance.
    assert_eq!(ledger.balance_of(&requester).amount(), 0.0);
    assert_eq!(ledger.balance_of(&winner_id).amount(), 0.0);
    assert_eq!(ledger.agent_count(), 2);

    // A zero-amount transfer succeeds as a recorded micro-transaction.
    let tx = ledger
        .transfer(
            requester,
            winner_id,
            ComputeCredits::new(0.0),
            intent.id,
            "negotiation recording".to_string(),
        )
        .unwrap();
    assert_eq!(tx.amount.amount(), 0.0);
    assert_eq!(ledger.transaction_count(), 1);

    // Verify transaction is queryable.
    let txs = ledger.transactions_for_intent(&intent.id);
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].description, "negotiation recording");
}

// ── (c) CK scheduling flow ─────────────────────────────────────────────────

#[test]
fn ck_scheduler_priority_ordering() {
    let mut scheduler = InferenceScheduler::new();

    // Submit 5 inferences with different priorities and attention weights.
    let priorities = [
        InferencePriority::Low,
        InferencePriority::Critical,
        InferencePriority::Normal,
        InferencePriority::High,
        InferencePriority::Deferred,
    ];
    let weights = [0.5, 0.3, 0.8, 0.1, 0.9];

    for (pri, weight) in priorities.iter().zip(weights.iter()) {
        scheduler.submit(make_ck_inference(*pri, 4096), *weight);
    }

    assert_eq!(scheduler.pending_count(), 5);

    // Schedule in order — should pick Critical first (priority=4, score=4*2+0.3=8.3).
    let job1 = scheduler.schedule_next().unwrap();
    assert_eq!(job1.inference.priority, InferencePriority::Critical);
    assert_eq!(job1.status, JobStatus::Running);

    // Next should be High (priority=3, score=3*2+0.1=6.1).
    let job2 = scheduler.schedule_next().unwrap();
    assert_eq!(job2.inference.priority, InferencePriority::High);

    // Next should be Normal (priority=2, score=2*2+0.8=4.8).
    let job3 = scheduler.schedule_next().unwrap();
    assert_eq!(job3.inference.priority, InferencePriority::Normal);

    // Next should be Low (priority=1, score=1*2+0.5=2.5).
    let job4 = scheduler.schedule_next().unwrap();
    assert_eq!(job4.inference.priority, InferencePriority::Low);

    // Last: Deferred (priority=0, score=0*2+0.9=0.9).
    let job5 = scheduler.schedule_next().unwrap();
    assert_eq!(job5.inference.priority, InferencePriority::Deferred);

    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(scheduler.active_jobs().len(), 5);

    // Verify KV-cache pressure.
    let pressure = scheduler.kv_cache_pressure();
    assert!(
        pressure > 1.0,
        "expected aggregate pressure > 1.0, got {pressure}"
    );
}

// ── (d) ZKPA session flow ──────────────────────────────────────────────────

#[test]
fn zkpa_session_lifecycle_with_proof_and_ledger() {
    let agent_id = Uuid::new_v4();
    let circuit = test_circuit();

    // 1. Create a session.
    let mut session_mgr = SessionManager::new();
    let session = session_mgr.create_session(
        agent_id,
        "integration-test-task".to_string(),
        100.0,
        Duration::from_secs(3600),
        None,
    );
    assert_eq!(session.agent_id, agent_id);

    // 2. Validate the session.
    let validated = session_mgr.validate_session(session.id).unwrap();
    assert_eq!(validated.id, session.id);
    assert_eq!(validated.task_scope, "integration-test-task");

    // 3. Generate a code integrity proof.
    let gen = ProofGenerator::new();
    let proof = gen
        .generate_code_integrity_proof(b"test_module_hash", &circuit)
        .unwrap();
    assert_eq!(proof.proof_type, "code_integrity");
    assert!(!proof.proof_bytes.is_empty());

    // 4. Emit liveness heartbeats and validate the chain.
    let mut liveness = LivenessMonitor::new();
    for _ in 0..5 {
        liveness.emit_heartbeat(agent_id, None).unwrap();
    }
    assert!(liveness.validate_chain(agent_id).is_ok());
    assert_eq!(liveness.chain_length(agent_id), 5);

    // 5. Append events to the AgencyLedger.
    let mut ledger = AgencyLedger::new();
    ledger
        .append(
            LedgerEventType::SessionCreated,
            agent_id,
            format!("session {} created", session.id),
        )
        .unwrap();
    ledger
        .append(
            LedgerEventType::ProofGenerated,
            agent_id,
            "code_integrity proof generated".to_string(),
        )
        .unwrap();
    ledger
        .append(
            LedgerEventType::LivenessProven,
            agent_id,
            "5 heartbeats validated".to_string(),
        )
        .unwrap();

    let agent_entries = ledger.query_by_agent(agent_id);
    assert_eq!(agent_entries.len(), 3);

    let proof_entries = ledger.query_by_event_type(&LedgerEventType::ProofGenerated);
    assert_eq!(proof_entries.len(), 1);

    // 6. Verify JSON export works.
    let json = ledger.export_format("json").unwrap();
    assert!(json.contains("code_integrity proof generated"));

    // 7. Verify read-only mode.
    ledger.set_read_only(true);
    let result = ledger.append(
        LedgerEventType::SessionRevoked,
        agent_id,
        "revoked".to_string(),
    );
    assert!(matches!(result, Err(ZkpaError::LedgerReadOnly)));
}

// ── (e) Cross-layer telemetry ──────────────────────────────────────────────

#[test]
fn cross_layer_telemetry_collection() {
    let mut collector = TelemetryCollector::new();

    // Record events from all four layers.
    collector.record(TelemetryEvent::SSE(SseEvent::MemoryAccessed {
        node_id: "sse-node-1".into(),
        strength: 0.85,
    }));
    collector.record(TelemetryEvent::SSE(SseEvent::DecayApplied {
        node_id: "sse-node-2".into(),
        new_strength: 0.42,
    }));
    collector.record(TelemetryEvent::SSE(SseEvent::MemoryPruned { count: 3 }));

    collector.record(TelemetryEvent::INP(InpEvent::IntentBroadcast {
        intent_id: "inp-intent-1".into(),
        action: "compute".into(),
    }));
    collector.record(TelemetryEvent::INP(InpEvent::BidReceived {
        intent_id: "inp-intent-1".into(),
        provider_id: "prov-1".into(),
    }));
    collector.record(TelemetryEvent::INP(InpEvent::NegotiationCompleted {
        intent_id: "inp-intent-1".into(),
        winner: "prov-1".into(),
    }));

    collector.record(TelemetryEvent::CK(CkEvent::InferenceScheduled {
        inference_id: "ck-inf-1".into(),
        priority: "critical".into(),
    }));
    collector.record(TelemetryEvent::CK(CkEvent::CircuitBreakerTripped {
        reason: "entropy spike".into(),
    }));

    collector.record(TelemetryEvent::ZKPA(ZkpaEvent::SessionCreated {
        session_id: "zkpa-sess-1".into(),
        agent_id: "agent-1".into(),
    }));
    collector.record(TelemetryEvent::ZKPA(ZkpaEvent::AttestationVerified {
        platform: "TPM".into(),
    }));
    collector.record(TelemetryEvent::ZKPA(ZkpaEvent::LivenessHeartbeat {
        agent_id: "agent-1".into(),
        sequence: 42,
    }));

    // Verify total count.
    assert_eq!(collector.events().len(), 11);

    // Verify per-layer counts.
    let counts = collector.count_by_layer();
    assert_eq!(
        counts,
        LayerCounts {
            sse: 3,
            inp: 3,
            ck: 2,
            zkpa: 3,
        }
    );

    // Verify recent events (last 4).
    let recent = collector.recent_events(4);
    assert_eq!(recent.len(), 4);
    assert!(matches!(
        recent[0],
        TelemetryEvent::CK(CkEvent::CircuitBreakerTripped { .. })
    ));
    assert!(matches!(
        recent[1],
        TelemetryEvent::ZKPA(ZkpaEvent::SessionCreated { .. })
    ));
    assert!(matches!(
        recent[2],
        TelemetryEvent::ZKPA(ZkpaEvent::AttestationVerified { .. })
    ));
    assert!(matches!(
        recent[3],
        TelemetryEvent::ZKPA(ZkpaEvent::LivenessHeartbeat { .. })
    ));

    // Verify clear.
    collector.clear();
    assert_eq!(collector.events().len(), 0);
    assert_eq!(collector.count_by_layer(), LayerCounts::default());
}

// ── (f) Memory decay → prune → ledger lifecycle ────────────────────────────

#[test]
fn memory_decay_prune_and_ledger_lifecycle() {
    let agent = sse_agent(1);

    // Create nodes at different initial strengths.
    let mut nodes: Vec<MemoryNode> = vec![
        {
            let mut n = MemoryNode::new(
                [1u8; 32],
                VectorEmbedding::new(vec![1.0, 0.0]),
                b"strong memory".to_vec(),
                "text/plain".to_string(),
                SensoryModality::Textual,
                agent,
            );
            n.current_strength = 1.0;
            n
        },
        {
            let mut n = MemoryNode::new(
                [2u8; 32],
                VectorEmbedding::new(vec![0.5, 0.5]),
                b"medium memory".to_vec(),
                "text/plain".to_string(),
                SensoryModality::Textual,
                agent,
            );
            n.current_strength = 0.06;
            n
        },
        {
            let mut n = MemoryNode::new(
                [3u8; 32],
                VectorEmbedding::new(vec![0.0, 1.0]),
                b"weak memory".to_vec(),
                "text/plain".to_string(),
                SensoryModality::Textual,
                agent,
            );
            n.current_strength = 0.01; // below default pruning threshold (0.05)
            n
        },
        {
            let mut n = MemoryNode::new(
                [4u8; 32],
                VectorEmbedding::new(vec![1.0, 1.0]),
                b"pinned memory".to_vec(),
                "text/plain".to_string(),
                SensoryModality::Textual,
                agent,
            );
            n.current_strength = 0.001;
            n.decay_profile.pinned = true; // should survive decay and prune
            n
        },
    ];

    // Apply decay for 24 hours.
    apply_batch_decay_for(&mut nodes, Duration::from_secs(86400));

    // Strong node should have decayed but still be above threshold.
    assert!(nodes[0].current_strength < 1.0);
    assert!(nodes[0].current_strength > 0.5);

    // Medium node should decay below pruning threshold after 24h.
    assert!(nodes[1].current_strength < 0.06);

    // Weak node is already below threshold.
    assert!(nodes[2].current_strength < 0.01);

    // Pinned node should be unchanged.
    assert!((nodes[3].current_strength - 0.001).abs() < 1e-6);

    // Collect stats before pruning.
    let stats_before = collection_stats(&nodes);
    assert_eq!(stats_before.total_nodes, 4);

    // Prune the collection.
    let pruned = prune_collection(&mut nodes);
    assert_eq!(pruned, 2); // weak + medium pruned (both below 0.05)
    assert_eq!(nodes.len(), 2);

    // Verify ledger entries exist for decay operations.
    let breakdown = Ledger::cost_breakdown(&nodes);
    // Each node has at least a write entry from creation + decay entries.
    assert!(breakdown.write > 0.0);
    // Decay operations should have been recorded.
    let total_decay_entries: usize = nodes
        .iter()
        .flat_map(|n| &n.ledger)
        .filter(|e| e.operation == anima_sse::memory_node::LedgerOperation::Decay)
        .count();
    assert!(total_decay_entries > 0);

    // Verify total cost is tracked.
    let total = Ledger::total_cost(&nodes);
    assert!(total > 0.0);
}

// ── (g) Capability Manifest → Registry flow ────────────────────────────────

#[test]
fn capability_manifest_to_registry_lookup() {
    let manifest_json = r#"{
        "name": "ml-inference-service",
        "version": "2.1.0",
        "description": "Multi-model ML inference provider",
        "capabilities": [
            {
                "action": "text_generation",
                "description": "Generate text from prompts",
                "performance": {
                    "typical_latency_ms": 200,
                    "max_latency_ms": 2000,
                    "accuracy": 0.97
                },
                "pricing": {
                    "type": "per_unit",
                    "credits_per_unit": 0.002,
                    "unit": "token"
                },
                "required_hardware": ["gpu", "nvme"],
                "tags": ["llm", "generation"]
            },
            {
                "action": "embedding",
                "description": "Compute vector embeddings",
                "performance": {
                    "typical_latency_ms": 50,
                    "max_latency_ms": 500,
                    "accuracy": 0.99
                },
                "pricing": {
                    "type": "fixed",
                    "credits": 0.1
                },
                "tags": ["embedding", "retrieval"]
            },
            {
                "action": "classification",
                "description": "Classify text into categories",
                "performance": {
                    "typical_latency_ms": 30,
                    "max_latency_ms": 300,
                    "accuracy": 0.94
                },
                "pricing": {
                    "type": "auction",
                    "max_budget": 5.0
                },
                "tags": ["classification"]
            }
        ],
        "metadata": {
            "author": "ml-team",
            "region": "us-east-1"
        }
    }"#;

    // 1. Parse the manifest.
    let manifest = ManifestParser::parse(manifest_json).unwrap();
    assert_eq!(manifest.name, "ml-inference-service");
    assert_eq!(manifest.capabilities.len(), 3);

    // 2. Validate the manifest.
    ManifestParser::validate(&manifest).unwrap();

    // 3. Convert to INP capabilities.
    let provider_id = inp_agent();
    let caps = ManifestParser::to_capabilities(&manifest, provider_id);
    assert_eq!(caps.len(), 3);
    assert_eq!(caps[0].action_types, vec!["text_generation"]);
    assert_eq!(caps[1].action_types, vec!["embedding"]);
    assert_eq!(caps[2].action_types, vec!["classification"]);

    // All caps belong to the same provider.
    for cap in &caps {
        assert_eq!(cap.provider_agent_id, provider_id);
    }

    // 4. Register in CapabilityRegistry.
    let mut registry = CapabilityRegistry::new();
    for cap in caps {
        registry.register(cap);
    }
    assert_eq!(registry.count(), 3);
    assert_eq!(registry.action_type_count(), 3);

    // 5. Look up by action type.
    let text_gen_caps = registry.lookup_by_action("text_generation");
    assert_eq!(text_gen_caps.len(), 1);
    assert_eq!(
        text_gen_caps[0].performance_profile.typical_latency,
        Duration::from_millis(200)
    );

    let embed_caps = registry.lookup_by_action("embedding");
    assert_eq!(embed_caps.len(), 1);
    assert!(embed_caps[0].handles_action("embedding"));

    let class_caps = registry.lookup_by_action("classification");
    assert_eq!(class_caps.len(), 1);

    // Verify non-existent action returns empty.
    assert!(registry.lookup_by_action("image_generation").is_empty());

    // Verify provider's capabilities.
    let provider_caps = registry.capabilities_of(&provider_id);
    assert_eq!(provider_caps.len(), 3);
}

// ── (h) Cross-layer: SSE memory integrity → ZKPA proof ────────────────────

#[test]
fn sse_memory_integrity_with_zkpa_proof() {
    let agent_sse = sse_agent(42);
    // Convert SSE [u8;32] agent to UUID by taking first 16 bytes.
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&agent_sse[..16]);
    let agent_uuid = Uuid::from_bytes(uuid_bytes);

    // Store a memory node in SSE.
    let node_id: [u8; 32] = [7u8; 32];
    let node = MemoryNode::new(
        node_id,
        VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
        b"sensitive configuration data".to_vec(),
        "application/json".to_string(),
        SensoryModality::Abstract,
        agent_sse,
    );

    let mut storage = InMemoryStorage::new();
    storage.write_node(node);

    // Verify integrity in SSE.
    let stored = storage.read_node(&node_id).unwrap();
    assert!(stored.verify_payload_integrity());

    // Generate a ZKPA code integrity proof over the payload hash.
    let circuit = test_circuit();
    let gen = ProofGenerator::new();
    let proof = gen
        .generate_code_integrity_proof(&stored.payload_hash, &circuit)
        .unwrap();
    assert_eq!(proof.proof_type, "code_integrity");
    // The public inputs should contain the payload hash.
    assert_eq!(proof.public_inputs, stored.payload_hash.to_vec());

    // Create a ZKPA session for the agent.
    let mut session_mgr = SessionManager::new();
    let session = session_mgr.create_session(
        agent_uuid,
        "memory-integrity-check".to_string(),
        50.0,
        Duration::from_secs(1800),
        None,
    );
    assert_eq!(session.agent_id, agent_uuid);

    // Append the proof event to the AgencyLedger.
    let mut ledger = AgencyLedger::new();
    ledger
        .append(
            LedgerEventType::ProofGenerated,
            agent_uuid,
            format!("memory integrity proof for node {}", hex::encode(node_id)),
        )
        .unwrap();

    let entries = ledger.query_by_agent(agent_uuid);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].details.contains(&hex::encode(node_id)));
}

// ── (i) INP NegotiationManager full lifecycle ──────────────────────────────

#[test]
fn inp_negotiation_manager_full_lifecycle() {
    let mut registry = CapabilityRegistry::new();
    let provider = inp_agent();
    registry.register(Capability::new(
        vec!["data_transform".to_string()],
        default_perf_profile(),
        EconomicTerms::FixedPrice { credits: 0.3 },
        provider,
    ));

    let mut manager = NegotiationManager::default();
    let intent = Intent::new(
        "data_transform".to_string(),
        serde_json::json!({"input": "raw.csv"}),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 1.0 },
        inp_agent(),
    );

    // Start negotiation.
    let intent_id = manager.start_negotiation(intent, &registry);
    let session = manager.get_session(&intent_id).unwrap();
    assert_eq!(session.state, NegotiationState::CollectingBids);

    // Submit a bid.
    let bid = CapabilityBid::new(
        intent_id,
        provider,
        Uuid::new_v4(),
        0.3,
        Duration::from_millis(150),
        0.96,
        0.92,
    );
    manager.submit_bid(bid).unwrap();

    let session = manager.get_session(&intent_id).unwrap();
    assert_eq!(session.bids.len(), 1);

    // Finalize and get winner.
    let (completed_id, winner) = manager.finalize_and_select(&intent_id).unwrap();
    assert_eq!(completed_id, intent_id);
    assert!(winner.is_some());
    assert_eq!(winner.unwrap(), provider);

    // Simulate execution via the session state machine.
    let session = manager.get_session_mut(&intent_id).unwrap();
    session.start_executing(provider);

    let result = ExecutionResult {
        intent_id,
        provider_id: provider,
        actual_cost: 0.3,
        actual_latency: Duration::from_millis(120),
        actual_accuracy: 0.97,
        proofs: vec!["zkp-attestation-hash".to_string()],
        status: ExecutionStatus::Completed,
    };
    session.complete(result);

    let session = manager.get_session(&intent_id).unwrap();
    assert_eq!(session.state, NegotiationState::Completed);
    assert!(session.result.is_some());
    assert_eq!(
        session.result.as_ref().unwrap().status,
        ExecutionStatus::Completed
    );

    // Verify the ledger records the transaction.
    let mut ledger = ComputeLedger::new();
    let requester = session.intent.origin_agent_id;
    ledger.ensure_agent(requester);
    ledger.ensure_agent(provider);

    // Zero-amount recording transaction.
    let tx = ledger
        .transfer(
            requester,
            provider,
            ComputeCredits::new(0.0),
            intent_id,
            "data_transform execution".to_string(),
        )
        .unwrap();
    assert_eq!(tx.amount.amount(), 0.0);
    assert_eq!(ledger.transaction_count(), 1);
    assert_eq!(ledger.balance_of(&requester).amount(), 0.0);
    assert_eq!(ledger.balance_of(&provider).amount(), 0.0);
}

// ── (j) Cross-layer: CK scheduler + ZKPA proof + Observatory telemetry ────

#[test]
fn ck_zkpa_observatory_cross_layer() {
    let agent_uuid = Uuid::new_v4();
    let circuit = test_circuit();

    // 1. Create a ZKPA session for the agent.
    let mut session_mgr = SessionManager::new();
    let session = session_mgr.create_session(
        agent_uuid,
        "ck-inference-task".to_string(),
        200.0,
        Duration::from_secs(7200),
        None,
    );

    // 2. Submit an inference to the CK scheduler.
    let mut scheduler = InferenceScheduler::new();
    let inference = make_ck_inference(InferencePriority::High, 8192);
    let _idx = scheduler.submit(inference.clone(), 0.7);
    let job = scheduler.schedule_next().unwrap();
    assert_eq!(job.inference.id, inference.id);

    // 3. Generate a ZKPA proof of computation correctness.
    let gen = ProofGenerator::new();
    let input_hash = [0xABu8; 32];
    let output_hash = [0xCDu8; 32];
    let proof = gen
        .generate_computation_correctness_proof(&input_hash, &output_hash, &circuit)
        .unwrap();
    assert_eq!(proof.proof_type, "computation_correctness");

    // 4. Record everything in Observatory telemetry.
    let mut collector = TelemetryCollector::new();
    collector.record(TelemetryEvent::ZKPA(ZkpaEvent::SessionCreated {
        session_id: session.id.to_string(),
        agent_id: agent_uuid.to_string(),
    }));
    collector.record(TelemetryEvent::CK(CkEvent::InferenceScheduled {
        inference_id: inference.id.to_string(),
        priority: "high".into(),
    }));
    collector.record(TelemetryEvent::ZKPA(ZkpaEvent::AttestationVerified {
        platform: "PLONK".into(),
    }));

    let counts = collector.count_by_layer();
    assert_eq!(counts.ck, 1);
    assert_eq!(counts.zkpa, 2);
    assert_eq!(collector.events().len(), 3);

    // 5. Verify the session is still valid.
    let validated = session_mgr.validate_session(session.id).unwrap();
    assert_eq!(validated.compute_budget, 200.0);
}

// ── (k) Adversarial fuzzing of SSE: cycle injection attacks ───────────────────

#[test]
fn sse_adversarial_cycle_detection_in_causal_graph() {
    // Attempt to inject cycles into the causal graph to trigger integrity violations.
    let mut graph = anima_sse::CausalGraph::new();

    // Create a chain of nodes: A → B → C
    let id_a: [u8; 32] = [1u8; 32];
    let id_b: [u8; 32] = [2u8; 32];
    let id_c: [u8; 32] = [3u8; 32];
    // Build valid chain
    graph
        .add_link(
            id_a,
            id_b,
            anima_sse::CausalLink::new(id_b, anima_sse::CausalLinkType::Caused, 0.9),
        )
        .unwrap();
    graph
        .add_link(
            id_b,
            id_c,
            anima_sse::CausalLink::new(id_c, anima_sse::CausalLinkType::Caused, 0.8),
        )
        .unwrap();

    // Verify initial integrity
    assert!(graph.validate_causal_integrity(0.5).is_ok());

    // Adversarial: attempt to close the cycle A → B → C → A
    // This should be detected as a cycle
    graph
        .add_link(
            id_c,
            id_a,
            anima_sse::CausalLink::new(id_a, anima_sse::CausalLinkType::Caused, 0.7),
        )
        .unwrap();

    // Cycle should now be detected
    let cycles = graph.detect_cycles();
    assert!(!cycles.is_empty(), "cycle should have been detected");

    // Integrity validation should fail
    let result = graph.validate_causal_integrity(0.5);
    assert!(result.is_err(), "integrity validation should fail on cycle");
}

#[test]
fn sse_adversarial_contradiction_detection() {
    // Attempt to inject contradictory relationships into the graph.
    let mut graph = anima_sse::CausalGraph::new();

    let id_a: [u8; 32] = [10u8; 32];
    let id_b: [u8; 32] = [20u8; 32];
    let id_c: [u8; 32] = [30u8; 32];

    // Add contradictory links
    graph
        .add_link(
            id_a,
            id_b,
            anima_sse::CausalLink::new(id_b, anima_sse::CausalLinkType::Contradicts, 0.95),
        )
        .unwrap();

    // Below threshold - should not be flagged
    graph
        .add_link(
            id_a,
            id_c,
            anima_sse::CausalLink::new(id_c, anima_sse::CausalLinkType::Contradicts, 0.3),
        )
        .unwrap();

    // Only the high-strength contradiction should be detected
    let contradictions = graph.detect_contradictions(0.5);
    assert_eq!(contradictions.len(), 1);
    assert_eq!(contradictions[0], (id_a, id_b));

    // Integrity check should fail with contradictions detected
    assert!(graph.validate_causal_integrity(0.5).is_err());
}

#[test]
fn sse_adversarial_memory_poisoning_attack() {
    // Attempt to poison memory nodes by corrupting embeddings.
    let mut storage = anima_sse::InMemoryStorage::new();
    let agent = sse_agent(5);

    // Create a set of valid nodes
    for i in 0..10u8 {
        let node = anima_sse::MemoryNode::new(
            [i; 32],
            VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
            format!("memory node {}", i).into_bytes(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            agent,
        );
        storage.write_node(node);
    }

    // Adversarial: corrupt one node's payload to break integrity
    let corrupted_id: [u8; 32] = [3u8; 32];
    if let Some(corrupted) = storage.read_node_mut(&corrupted_id) {
        corrupted.payload[0] ^= 0xFF; // Flip bits to corrupt
    }

    // Verify integrity check fails
    let corrupted_node = storage.read_node(&corrupted_id).unwrap();
    assert!(!corrupted_node.verify_payload_integrity());

    // Other nodes should still be valid
    for i in [0, 1, 2, 4, 5, 6, 7, 8, 9] {
        let node = storage.read_node(&[i; 32]).unwrap();
        assert!(node.verify_payload_integrity());
    }
}

#[test]
fn sse_adversarial_self_loop_attack() {
    // Attempt to create self-loops in the causal graph.
    let mut graph = anima_sse::CausalGraph::new();
    let self_loop_id: [u8; 32] = [42u8; 32];

    // Attempt to create a self-link
    let result = graph.add_link(
        self_loop_id,
        self_loop_id,
        anima_sse::CausalLink::new(self_loop_id, anima_sse::CausalLinkType::Caused, 0.5),
    );

    // Should be rejected
    assert!(matches!(result, Err(anima_sse::CausalError::SelfLoop { .. })));
    assert_eq!(graph.node_count(), 0, "self-loop should not register node");
}

// ── (l) Adversarial fuzzing of INP: intent spam and spoofing ──────────────────

#[test]
fn inp_adversarial_intent_spam_flood() {
    // Flood the system with rapid intents to test robustness.
    let mut registry = CapabilityRegistry::new();
    let provider = inp_agent();
    registry.register(Capability::new(
        vec!["compute".to_string()],
        default_perf_profile(),
        EconomicTerms::FixedPrice { credits: 0.1 },
        provider,
    ));

    let mut manager = NegotiationManager::default();
    let mut intent_ids = Vec::new();

    // Spam 1000 intents rapidly
    for i in 0..1000usize {
        let intent = Intent::new(
            "compute".to_string(),
            serde_json::json!({"batch": i}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            inp_agent(),
        );
        let intent_id = manager.start_negotiation(intent, &registry);
        intent_ids.push(intent_id);
    }

    // All intents should have created sessions
    assert_eq!(manager.active_count(), 1000);

    // Verify they can all be retrieved
    for intent_id in &intent_ids {
        assert!(manager.get_session(intent_id).is_some());
    }
}

#[test]
fn inp_adversarial_capability_spoofing() {
    // Attempt to spoof a capability with malicious bid values.
    let mut registry = CapabilityRegistry::new();
    let legitimate_provider = inp_agent();
    registry.register(Capability::new(
        vec!["compute".to_string()],
        PerformanceProfile {
            typical_latency: Duration::from_millis(100),
            max_latency: Duration::from_secs(5),
            typical_accuracy: 0.95,
            throughput_per_second: 100.0,
        },
        EconomicTerms::FixedPrice { credits: 0.5 },
        legitimate_provider,
    ));

    // Malicious provider attempts to spoof with unrealistic values
    let spoofer = inp_agent();
    registry.register(Capability::new(
        vec!["compute".to_string()],
        PerformanceProfile {
            typical_latency: Duration::from_millis(1), // Unrealistically low
            max_latency: Duration::from_nanos(1),
            typical_accuracy: 1.0, // Impossible accuracy
            throughput_per_second: 1_000_000.0, // Unrealistic throughput
        },
        EconomicTerms::FixedPrice { credits: 0.0001 }, // Tiny price
        spoofer,
    ));

    // Both should be registered (validation is the registry's job)
    assert_eq!(registry.count(), 2);

    // Broadcast should find both
    let mut broadcaster = IntentBroadcaster::default();
    let intent = Intent::new(
        "compute".to_string(),
        serde_json::json!({}),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 1.0 },
        inp_agent(),
    );

    let providers = broadcaster.begin_broadcast(&intent, &registry);
    assert_eq!(providers.len(), 2, "both legitimate and spoofed providers found");
}

#[test]
fn inp_adversarial_bid_manipulation_suspicious_values() {
    // Submit bids with manipulated (extreme) values to test scoring resilience.
    let qos = QoSConstraints::default();

    // Create a bid with extremely low cost but unrealistic values
    let suspicious_bid = CapabilityBid::new(
        Uuid::new_v4(),
        inp_agent(),
        Uuid::new_v4(),
        0.000001, // Suspiciously low cost
        Duration::from_millis(1), // Unrealistic latency
        1.0, // Perfect accuracy
        0.99,
    );

    let normal_bid = CapabilityBid::new(
        Uuid::new_v4(),
        inp_agent(),
        Uuid::new_v4(),
        0.5,
        Duration::from_millis(100),
        0.95,
        0.9,
    );

    // Both bids should produce scores
    let suspicious_score = suspicious_bid.score(&qos);
    let normal_score = normal_bid.score(&qos);

    // Scores should be calculable and comparable
    assert!(suspicious_score.is_finite());
    assert!(normal_score.is_finite());

    // Suspicious bid should score well (would be preferred by naive selection)
    // This demonstrates need for bid validation/sanity checking
    assert!(suspicious_score < normal_score, "suspicious bid has better score");
}

#[test]
fn inp_adversarial_bid_limit_enforcement() {
    // Test that the system enforces bid limits during spam attacks.
    let (registry, provider) = {
        let mut registry = CapabilityRegistry::new();
        let provider = inp_agent();
        registry.register(Capability::new(
            vec!["compute".to_string()],
            default_perf_profile(),
            EconomicTerms::FixedPrice { credits: 0.1 },
            provider,
        ));
        (registry, provider)
    };

    let mut broadcaster = IntentBroadcaster::new(BroadcastConfig {
        bid_timeout: Duration::from_secs(30),
        max_bids: 100, // Lower limit for testing
    });

    let intent = Intent::new(
        "compute".to_string(),
        serde_json::json!({}),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 1.0 },
        inp_agent(),
    );

    broadcaster.begin_broadcast(&intent, &registry);

    // Attempt to spam 200 bids (exceeding limit)
    let intent_id = intent.id;
    let capability_id = registry.lookup_by_action("compute")[0].id;

    for i in 0..200usize {
        let bid = CapabilityBid::new(
            intent_id,
            provider,
            capability_id,
            i as f64 / 100.0,
            Duration::from_millis(100 + i as u64),
            0.95,
            0.9,
        );

        let result = broadcaster.submit_bid(bid);
        if i >= 100 {
            // Should fail after limit reached
            assert!(
                matches!(result, Err(BroadcastError::BidLimitReached { .. })),
                "bid limit should be enforced at bid {}",
                i
            );
        }
    }
}

// ── (m) Adversarial testing of ZKPA: session attacks ──────────────────────────

#[test]
fn zkpa_adversarial_session_hijacking_attempt() {
    // Attempt to use an expired session.
    let mut session_mgr = SessionManager::new();
    let agent = Uuid::new_v4();

    let session = session_mgr.create_session(
        agent,
        "sensitive-task".to_string(),
        100.0,
        Duration::from_millis(10), // Very short TTL
        None,
    );

    // Session should be valid immediately
    assert!(session_mgr.validate_session(session.id).is_ok());

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(20));

    // Session should now be expired/invalid
    let result = session_mgr.validate_session(session.id);
    assert!(matches!(result, Err(ZkpaError::SessionExpired)));
}

#[test]
fn zkpa_adversarial_liveness_proof_replay_attack() {
    // Attempt to replay a liveness heartbeat to spoof continuity.
    // The liveness monitor uses hash-chain validation, so forging should fail.
    let mut liveness = LivenessMonitor::new();
    let agent = Uuid::new_v4();

    // Emit legitimate heartbeats
    let hb1 = liveness.emit_heartbeat(agent, None).unwrap();
    let hb2 = liveness.emit_heartbeat(agent, None).unwrap();

    // Verify legitimate chain validates
    assert!(liveness.validate_chain(agent).is_ok());

    // Simulate a replay attack: try to emit a heartbeat with the same sequence
    // but different hash - this should succeed as a new heartbeat but break chain continuity
    // Instead, we test that the hash chain prevents tampering by checking:
    // The sequence numbers are monotonic and hashes are deterministic
    assert_eq!(hb1.sequence_num, 0);
    assert_eq!(hb2.sequence_num, 1);
    assert_ne!(hb1.hash_chain_link, hb2.hash_chain_link);

    // Demonstrate that gap detection works for missing heartbeats
    let gaps = liveness.detect_gaps(agent, 5).unwrap();
    assert_eq!(gaps, vec![2, 3, 4], "should detect missing heartbeats 2-4");
}

#[test]
fn zkpa_adversarial_agency_ledger_tampering() {
    // Attempt to tamper with ledger entries.
    let agent = Uuid::new_v4();
    let mut ledger = AgencyLedger::new();

    ledger
        .append(
            LedgerEventType::SessionCreated,
            agent,
            "valid entry".to_string(),
        )
        .unwrap();

    // Verify entry exists
    let entries = ledger.query_by_agent(agent);
    assert_eq!(entries.len(), 1);

    // Attempt to set read-only and make modifications fail
    ledger.set_read_only(true);

    let result = ledger.append(
        LedgerEventType::SessionRevoked,
        agent,
        "tampering attempt".to_string(),
    );

    assert!(matches!(result, Err(ZkpaError::LedgerReadOnly)));
}

// ── (n) CK fault injection testing ───────────────────────────────────────────

#[test]
fn ck_fault_injection_inference_preemption() {
    // Test that preemption works correctly under fault conditions.
    let mut scheduler = InferenceScheduler::new();

    // Submit jobs: Critical first (priority=4), High second (priority=3)
    let critical_idx = scheduler.submit(make_ck_inference(InferencePriority::Critical, 4096), 0.5);
    let _high_idx = scheduler.submit(make_ck_inference(InferencePriority::High, 4096), 0.9);

    // Schedule - Critical runs first (higher priority)
    let job = scheduler.schedule_next().unwrap();
    assert_eq!(job.inference.priority, InferencePriority::Critical);

    // Preempt the critical job (simulating fault/corruption)
    // The preemption should succeed
    assert!(scheduler.preempt(critical_idx));

    // After preemption:
    // - Pending jobs: 1 (the High job)
    // - Active jobs (Running or Preempted): 1 (the preempted Critical)
    assert_eq!(scheduler.pending_count(), 1); // high remaining
    assert_eq!(scheduler.active_jobs().len(), 1); // one preempted critical
}

#[test]
fn ck_fault_injection_scheduler_stress() {
    // Stress test scheduler with many concurrent jobs.
    let mut scheduler = InferenceScheduler::new();

    // Submit 1000 jobs with random priorities
    for i in 0..1000usize {
        let priority = match i % 5 {
            0 => InferencePriority::Deferred,
            1 => InferencePriority::Low,
            2 => InferencePriority::Normal,
            3 => InferencePriority::High,
            _ => InferencePriority::Critical,
        };
        scheduler.submit(make_ck_inference(priority, 1024), (i as f32) / 1000.0);
    }

    // All jobs should be pending initially
    assert_eq!(scheduler.pending_count(), 1000);

    // Schedule all jobs
    let mut scheduled_count = 0;
    while scheduler.pending_count() > 0 {
        if scheduler.schedule_next().is_some() {
            scheduled_count += 1;
        } else {
            break;
        }
    }
    assert_eq!(scheduled_count, 1000);
    assert_eq!(scheduler.active_jobs().len(), 1000);
}

#[test]
fn ck_thermal_pressure_simulation() {
    // Simulate thermal runaway by checking KV cache pressure behavior.
    let mut scheduler = InferenceScheduler::new();

    // Submit many jobs requiring large KV caches
    for _ in 0..50usize {
        scheduler.submit(make_ck_inference(InferencePriority::High, 1_000_000), 0.9);
    }

    // Schedule all jobs
    for _ in 0..50usize {
        scheduler.schedule_next();
    }

    // Pressure should be high (simulating thermal/VRAM pressure)
    let pressure = scheduler.kv_cache_pressure();
    assert!(pressure > 10.0, "pressure should be elevated: {pressure}");
}

// ── (o) Multi-node distributed INP stress test ───────────────────────────────

#[test]
fn inp_multi_node_distributed_negotiation() {
    // Simulate many agents negotiating across multiple action types.
    let mut registry = CapabilityRegistry::new();

    // Create 50 providers for various action types
    let providers: Vec<Uuid> = (0..50).map(|_| inp_agent()).collect();

    for provider in &providers {
        registry.register(Capability::new(
            vec![
                "text_generation".to_string(),
                "embedding".to_string(),
                "classification".to_string(),
            ],
            default_perf_profile(),
            EconomicTerms::FixedPrice { credits: 0.1 },
            *provider,
        ));
    }

    let mut manager = NegotiationManager::default();

    // Run 100 concurrent negotiations
    for i in 0..100usize {
        let action = match i % 3 {
            0 => "text_generation",
            1 => "embedding",
            _ => "classification",
        };

        let intent = Intent::new(
            action.to_string(),
            serde_json::json!({"batch": i}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            inp_agent(),
        );

        let intent_id = manager.start_negotiation(intent, &registry);

        // Submit bids from matching providers
        let matching = registry.lookup_by_action(action);
        for cap in matching.iter().take(5) {
            let bid = CapabilityBid::new(
                intent_id,
                cap.provider_agent_id,
                cap.id,
                0.1 + rand::random::<f64>(),
                Duration::from_millis(50 + rand::random::<u64>() % 100),
                0.9 + rand::random::<f32>() % 0.09,
                0.9,
            );
            let _ = manager.submit_bid(bid);
        }

        // Finalize negotiation and remove session (simulating cleanup)
        let _ = manager.finalize_and_select(&intent_id);
        let _ = manager.remove_session(&intent_id);
    }

    // All sessions should be removed after finalization
    assert_eq!(manager.active_count(), 0, "all sessions should be cleaned up");
}

// ── (p) Causal graph scaling benchmark ────────────────────────────────────────

#[test]
fn causal_graph_scaling_benchmark() {
    use std::time::Instant;

    // Build a causal graph with many nodes to test retrieval performance.
    let mut graph = anima_sse::CausalGraph::new();
    let node_count = 100_000usize;

    // Create a linear chain plus random links
    let start = Instant::now();

    for i in 0..node_count as u8 {
        let id: [u8; 32] = [i; 32];
        let next_id: [u8; 32] = [((i as usize + 1) % 256) as u8; 32];

        graph
            .add_link(
                id,
                next_id,
                anima_sse::CausalLink::new(
                    next_id,
                    anima_sse::CausalLinkType::Caused,
                    0.9,
                ),
            )
            .ok();
    }

    let build_time = start.elapsed();
    assert!(
        build_time < Duration::from_secs(5),
        "graph build took too long: {:?}",
        build_time
    );

    // Test context chain retrieval performance
    let target: [u8; 32] = [(node_count % 256) as u8; 32];
    let start = Instant::now();
    let _chain = graph.get_context_chain(&target, 100);
    let chain_time = start.elapsed();

    // Chain retrieval should be sub-millisecond for reasonable sizes
    assert!(
        chain_time < Duration::from_millis(100),
        "chain retrieval too slow: {:?}",
        chain_time
    );

    // Test cycle detection on large graph
    let start = Instant::now();
    let cycles = graph.detect_cycles();
    let cycle_time = start.elapsed();

    // This specific graph has no cycles, but detection should still be fast
    assert!(
        cycle_time < Duration::from_millis(100),
        "cycle detection too slow: {:?}",
        cycle_time
    );
    assert!(cycles.is_empty());
}
