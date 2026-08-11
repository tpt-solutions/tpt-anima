//! INP — Multi-Agent Intent Negotiation
//!
//! Mirrors `DEMO_SCENARIOS.md` Demo 2: an agent broadcasts an intent, two
//! capability providers bid, the best bid wins, and compute credits move
//! from requester to winner over a real `ComputeLedger`.
//!
//! Run with: `cargo run -p tpt-anima-inp --example negotiation_swarm`

use std::time::Duration;

use uuid::Uuid;

use tpt_anima_inp::{
    Capability, CapabilityBid, CapabilityRegistry, ComputeCredits, ComputeLedger, EconomicTerms,
    ExecutionResult, ExecutionStatus, Intent, NegotiationSession, PerformanceProfile,
    QoSConstraints,
};

fn main() {
    let requester = Uuid::new_v4();
    let beta = Uuid::new_v4();
    let gamma = Uuid::new_v4();

    // ── Register two competing capability providers ─────────────────────
    let mut registry = CapabilityRegistry::new();

    let beta_cap = Capability::new(
        vec!["analyze".into()],
        PerformanceProfile {
            typical_latency: Duration::from_millis(200),
            max_latency: Duration::from_millis(600),
            typical_accuracy: 0.92,
            throughput_per_second: 8.0,
        },
        EconomicTerms::FixedPrice { credits: 100.0 },
        beta,
    );
    let gamma_cap = Capability::new(
        vec!["analyze".into()],
        PerformanceProfile {
            typical_latency: Duration::from_millis(400),
            max_latency: Duration::from_millis(800),
            typical_accuracy: 0.98,
            throughput_per_second: 4.0,
        },
        EconomicTerms::FixedPrice { credits: 150.0 },
        gamma,
    );
    let beta_cap_id = registry.register(beta_cap);
    let gamma_cap_id = registry.register(gamma_cap);
    println!(
        "Registered {} capabilities for action 'analyze'",
        registry.count()
    );

    // ── Broadcast an intent requiring accuracy > 95% ─────────────────────
    let qos = QoSConstraints {
        min_accuracy: Some(0.95),
        ..QoSConstraints::default()
    };

    let intent = Intent::new(
        "analyze".into(),
        serde_json::json!({ "task": "Analyze Q3 financial report" }),
        qos,
        EconomicTerms::PerUnit {
            credits_per_unit: 1.0,
            unit: "report".into(),
        },
        requester,
    );
    let intent_id = intent.id;

    let mut session = NegotiationSession::new(intent);
    session.start_broadcasting();
    println!("1. Intent {intent_id} broadcast: 'Analyze Q3 financial report'");

    session.start_collecting();
    session.add_bid(CapabilityBid::new(
        intent_id,
        beta,
        beta_cap_id,
        100.0,
        Duration::from_millis(200),
        0.92,
        0.9,
    ));
    session.add_bid(CapabilityBid::new(
        intent_id,
        gamma,
        gamma_cap_id,
        150.0,
        Duration::from_millis(400),
        0.98,
        0.95,
    ));
    println!("2. Collected {} bids", session.bids.len());

    // Pick the best bid that meets the QoS accuracy floor.
    let winner = session
        .bids
        .iter()
        .filter(|b| b.estimated_accuracy >= 0.95)
        .min_by(|a, b| a.estimated_cost.total_cmp(&b.estimated_cost))
        .expect("gamma meets the accuracy requirement")
        .clone();
    println!(
        "3. Winner: provider {} (accuracy {:.0}%, cost {})",
        winner.provider_id,
        winner.estimated_accuracy * 100.0,
        winner.estimated_cost
    );

    session.start_executing(winner.provider_id);
    session.complete(ExecutionResult {
        intent_id,
        provider_id: winner.provider_id,
        actual_cost: winner.estimated_cost,
        actual_latency: winner.estimated_latency,
        actual_accuracy: winner.estimated_accuracy,
        proofs: vec![],
        status: ExecutionStatus::Completed,
    });
    println!("4. Negotiation state: {:?}", session.state);

    // ── Real compute-credit transfer for the winning execution ──────────
    let mut ledger = ComputeLedger::new();
    ledger
        .mint(requester, ComputeCredits::new(500.0))
        .expect("mint starting balance");
    let tx = ledger
        .transfer(
            requester,
            winner.provider_id,
            ComputeCredits::new(winner.estimated_cost),
            intent_id,
            "Q3 financial report analysis".into(),
        )
        .expect("requester has sufficient balance");
    println!(
        "5. Transferred {} credits from {} to {} (tx {})",
        tx.amount.amount(),
        tx.from_agent,
        tx.to_agent,
        tx.id
    );
}
