//! Full-stack quickstart — one agent's request touching all four layers.
//!
//! Walks the golden path: store a memory in SSE, negotiate an intent in INP,
//! open an ephemeral ZKPA session for the winning provider, then check CK's
//! thermal budget before dispatching the inference. This is the compiling
//! version of the walkthrough described in docs/ONBOARDING.md.
//!
//! Run with: `cargo run -p tpt-anima-observatory --example full_stack_quickstart`

use std::time::Duration;

use uuid::Uuid;

use tpt_anima_ck::budgeting::{BudgetManager, ComputeBudget};
use tpt_anima_ck::types::ThermalConstraints;
use tpt_anima_inp::{
    Capability, CapabilityBid, CapabilityRegistry, EconomicTerms, Intent, NegotiationSession,
    PerformanceProfile, QoSConstraints,
};
use tpt_anima_sse::{InMemoryStorage, MemoryNode, SensoryModality, VectorEmbedding};
use tpt_anima_zkpa::SessionManager;

fn main() {
    // ── Layer 1 — SSE: store the fact this request will reason about ────────
    let mut storage = InMemoryStorage::new();
    let sse_agent_id = [9u8; 32];
    let memory = MemoryNode::new(
        [1u8; 32],
        VectorEmbedding::new(vec![0.12, 0.44, -0.08, 0.91]),
        b"Q3 financial report: revenue up 12%.".to_vec(),
        "text/plain".into(),
        SensoryModality::Textual,
        sse_agent_id,
    );
    storage.write_node(memory);
    println!("1. [SSE]  Stored 1 memory node ({} total in substrate)", storage.node_count());

    // ── Layer 2 — INP: negotiate who analyzes it ─────────────────────────────
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
        serde_json::json!({ "task": "Analyze Q3 financial report" }),
        QoSConstraints::default(),
        EconomicTerms::FixedPrice { credits: 100.0 },
        requester,
    );
    let intent_id = intent.id;

    let mut session = NegotiationSession::new(intent);
    session.start_broadcasting();
    session.start_collecting();
    session.add_bid(CapabilityBid::new(
        intent_id,
        provider,
        capability_id,
        100.0,
        Duration::from_millis(200),
        0.92,
        0.9,
    ));
    let winner = session.bids.first().expect("one bid was submitted").clone();
    println!(
        "2. [INP]  Negotiated intent {intent_id}: provider {} won (accuracy {:.0}%)",
        winner.provider_id,
        winner.estimated_accuracy * 100.0
    );

    // ── Layer 3 — ZKPA: give the winning provider an ephemeral session ───────
    let mut sessions = SessionManager::new();
    let zkpa_session = sessions.create_session(
        winner.provider_id,
        "q3-report-analysis".into(),
        winner.estimated_cost,
        Duration::from_secs(300),
        None,
    );
    println!(
        "3. [ZKPA] Issued task-scoped session {} for provider {} (expires in 300s, no persistent API key)",
        zkpa_session.id, winner.provider_id
    );

    // ── Layer 4 — CK: check thermal budget before dispatching the inference ──
    let budget = ComputeBudget {
        flops_budget: 5_000_000_000_000,
        thermal_constraints: ThermalConstraints { max_temp_celsius: 80.0, max_power_watts: 65.0 },
    };
    let mut budget_manager = BudgetManager::new(budget);
    budget_manager.update_temperature(58.0);
    println!(
        "4. [CK]   Thermal status before dispatch: {:?} (throttle factor {:.2})",
        budget_manager.thermal_status(),
        budget_manager.thermal_throttle_factor()
    );

    println!("\nGolden path complete — one request touched SSE, INP, ZKPA, and CK.");
}
