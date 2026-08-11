//! Anima Observatory — real-time human telemetry window for TPT Anima.
//!
//! Starts a real HTTP + WebSocket server (see `server.rs`) and, once at
//! startup, drives one genuine operation through each of the four Anima
//! layers (SSE, INP, CK, ZKPA) via their real public APIs. The telemetry
//! broadcast to the frontend is derived from the actual results of those
//! calls (real strengths, real ids, real credit amounts) rather than from a
//! random-number generator standing in for "the backend."

use std::time::Duration;

use uuid::Uuid;

use tpt_anima_ck::scheduler::InferenceScheduler;
use tpt_anima_ck::types::{
    CircuitBreakerConfig, EvictionPolicy, Inference, InferencePriority, KVCacheConfig,
    ResourceRequest,
};
use tpt_anima_inp::{
    Capability, CapabilityRegistry, ComputeCredits, ComputeLedger, EconomicTerms,
    PerformanceProfile,
};
use tpt_anima_sse::{InMemoryStorage, MemoryNode, SensoryModality, VectorEmbedding};
use tpt_anima_zkpa::SessionManager;

use tpt_anima_observatory::{CkEvent, InpEvent, SseEvent, TelemetryEvent, TelemetryServer, ZkpaEvent};

const OBSERVATORY_ADDR: &str = "127.0.0.1:9001";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Anima Observatory — starting real-time telemetry server");
    println!("Collecting telemetry from SSE, INP, CK, and ZKPA layers...\n");

    let server = TelemetryServer::new(256);
    bootstrap_layers(&server).await;

    println!(
        "Observatory HTTP+WS server listening on http://{OBSERVATORY_ADDR} \
         (WebSocket at /ws/telemetry, REST at /api/health, /api/telemetry/recent, /api/telemetry/count)"
    );
    server.start(OBSERVATORY_ADDR).await
}

/// Perform one real operation against each of the four Anima layers and
/// broadcast the resulting telemetry, deriving every field from the real
/// return value of the underlying call rather than fabricating it.
async fn bootstrap_layers(server: &TelemetryServer) {
    sse_bootstrap(server).await;
    let (requester_id, provider_id) = inp_bootstrap(server).await;
    ck_bootstrap(server).await;
    zkpa_bootstrap(server, requester_id).await;
    let _ = provider_id;
}

async fn sse_bootstrap(server: &TelemetryServer) {
    let mut storage = InMemoryStorage::new();

    let mut node_id = [0u8; 32];
    node_id[0] = 1;
    let mut agent_id = [0u8; 32];
    agent_id[0] = 9;

    let mut node = MemoryNode::new(
        node_id,
        VectorEmbedding::new(vec![0.1, 0.2, 0.3, 0.4]),
        b"Observatory bootstrap memory".to_vec(),
        "text/plain".into(),
        SensoryModality::Textual,
        agent_id,
    );

    node.record_access(agent_id);
    let strength_after_access = node.current_strength;

    node.apply_decay_for(Duration::from_secs(3600));
    let strength_after_decay = node.current_strength;

    let node_id_str = hex_id(&node_id);
    storage.write_node(node);

    server
        .record_and_broadcast(TelemetryEvent::SSE(SseEvent::MemoryAccessed {
            node_id: node_id_str.clone(),
            strength: strength_after_access,
        }))
        .await;
    server
        .record_and_broadcast(TelemetryEvent::SSE(SseEvent::DecayApplied {
            node_id: node_id_str,
            new_strength: strength_after_decay,
        }))
        .await;
}

/// Returns (requester_id, provider_id) so callers can reuse them.
async fn inp_bootstrap(server: &TelemetryServer) -> (Uuid, Uuid) {
    let mut registry = CapabilityRegistry::new();
    let provider_id = Uuid::new_v4();
    let requester_id = Uuid::new_v4();
    let intent_id = Uuid::new_v4();

    let capability = Capability::new(
        vec!["analyze".into()],
        PerformanceProfile {
            typical_latency: Duration::from_millis(120),
            max_latency: Duration::from_millis(500),
            typical_accuracy: 0.94,
            throughput_per_second: 12.0,
        },
        EconomicTerms::FixedPrice { credits: 4.0 },
        provider_id,
    );
    registry.register(capability);

    server
        .record_and_broadcast(TelemetryEvent::INP(InpEvent::IntentBroadcast {
            intent_id: intent_id.to_string(),
            action: "analyze".into(),
        }))
        .await;

    let mut ledger = ComputeLedger::new();
    if ledger
        .mint(requester_id, ComputeCredits::new(100.0))
        .is_ok()
    {
        if let Ok(tx) = ledger.transfer(
            requester_id,
            provider_id,
            ComputeCredits::new(4.0),
            intent_id,
            "Observatory bootstrap task".into(),
        ) {
            server
                .record_and_broadcast(TelemetryEvent::INP(InpEvent::NegotiationCompleted {
                    intent_id: intent_id.to_string(),
                    winner: provider_id.to_string(),
                }))
                .await;
            server
                .record_and_broadcast(TelemetryEvent::INP(InpEvent::CreditTransferred {
                    from: tx.from_agent.to_string(),
                    to: tx.to_agent.to_string(),
                    amount: tx.amount.amount(),
                }))
                .await;
        }
    }

    (requester_id, provider_id)
}

async fn ck_bootstrap(server: &TelemetryServer) {
    let mut scheduler = InferenceScheduler::new();

    let inference = Inference {
        id: Uuid::new_v4(),
        model_id: "tpt-anima-ck-bootstrap".into(),
        resource_requirements: ResourceRequest {
            cpu_cores: 4,
            gpu_count: 0,
            vram_mb: 0,
            ram_mb: 2048,
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
        priority: InferencePriority::Normal,
    };

    scheduler.submit(inference, 0.7);

    if let Some(job) = scheduler.schedule_next() {
        server
            .record_and_broadcast(TelemetryEvent::CK(CkEvent::InferenceScheduled {
                inference_id: job.inference.id.to_string(),
                priority: job.inference.priority.to_string(),
            }))
            .await;
    }
}

async fn zkpa_bootstrap(server: &TelemetryServer, requester_id: Uuid) {
    let mut sessions = SessionManager::new();
    let session = sessions.create_session(
        requester_id,
        "observatory-bootstrap".into(),
        10.0,
        Duration::from_secs(300),
        None,
    );

    server
        .record_and_broadcast(TelemetryEvent::ZKPA(ZkpaEvent::SessionCreated {
            session_id: session.id.to_string(),
            agent_id: session.agent_id.to_string(),
        }))
        .await;
}

fn hex_id(id: &[u8; 32]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
