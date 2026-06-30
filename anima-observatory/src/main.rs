//! Anima Observatory — real-time human telemetry window for TPT Anima.
//!
//! Translates machine-speed AI cognition into observable, auditable visualisations:
//! Swarm Map, Memory Topography, Inference Pulse, Agency Ledger, Economic Flow.

use anima_observatory::{
    LayerCounts, MemoryTopographyData, PulseEvent, SwarmMapData, SwarmNode, TelemetryCollector,
    TelemetryEvent, TopoNode,
};

fn main() {
    println!("Anima Observatory — starting (Phase 4 implementation)");
    println!("Observatory server starting on ws://localhost:9001");
    println!("HTTP API available at http://localhost:9001/api/...\n");
    println!("Collecting telemetry from SSE, INP, CK, and ZKPA layers...\n");

    // ── Build a small Swarm Map ───────────────────────────────────────────────

    let mut swarm = SwarmMapData::new();
    swarm.add_node(SwarmNode {
        id: "cog-1".into(),
        label: "Cognitive Core".into(),
        status: "active".into(),
        agent_type: "cognitive".into(),
    });
    swarm.add_node(SwarmNode {
        id: "relay-1".into(),
        label: "Relay Agent".into(),
        status: "active".into(),
        agent_type: "relay".into(),
    });
    swarm.add_node(SwarmNode {
        id: "zk-1".into(),
        label: "Attestation Agent".into(),
        status: "idle".into(),
        agent_type: "zkpa".into(),
    });

    swarm.add_edge(anima_observatory::SwarmEdge {
        source: "cog-1".into(),
        target: "relay-1".into(),
        weight: 0.9,
    });
    swarm.add_edge(anima_observatory::SwarmEdge {
        source: "relay-1".into(),
        target: "zk-1".into(),
        weight: 0.6,
    });

    swarm.record_pulse(PulseEvent {
        source: "cog-1".into(),
        target: "relay-1".into(),
        intent_id: "intent-001".into(),
        timestamp: 1719763200,
        won: true,
    });

    println!(
        "Swarm Map: {} nodes, {} edges, {} pulses",
        swarm.nodes.len(),
        swarm.edges.len(),
        swarm.pulse_events.len()
    );

    // ── Build Memory Topography ───────────────────────────────────────────────

    let mut topo = MemoryTopographyData::new();
    topo.add_node(TopoNode {
        id: "mem-1".into(),
        strength: 0.95,
        modality: "semantic".into(),
        label: "Project Architecture".into(),
    });
    topo.add_node(TopoNode {
        id: "mem-2".into(),
        strength: 0.42,
        modality: "procedural".into(),
        label: "Build Commands".into(),
    });

    let (n_nodes, n_chains) = topo.summary();
    println!(
        "Memory Topography: {} nodes, {} chains\n",
        n_nodes, n_chains
    );

    // ── Record sample telemetry events ────────────────────────────────────────

    let mut collector = TelemetryCollector::new();

    collector.record(TelemetryEvent::SSE(
        anima_observatory::SseEvent::MemoryAccessed {
            node_id: "mem-1".into(),
            strength: 0.95,
        },
    ));
    collector.record(TelemetryEvent::SSE(
        anima_observatory::SseEvent::MemoryPruned { count: 3 },
    ));
    collector.record(TelemetryEvent::SSE(
        anima_observatory::SseEvent::DecayApplied {
            node_id: "mem-2".into(),
            new_strength: 0.38,
        },
    ));

    collector.record(TelemetryEvent::INP(
        anima_observatory::InpEvent::IntentBroadcast {
            intent_id: "intent-001".into(),
            action: "analyze".into(),
        },
    ));
    collector.record(TelemetryEvent::INP(
        anima_observatory::InpEvent::BidReceived {
            intent_id: "intent-001".into(),
            provider_id: "cog-1".into(),
        },
    ));
    collector.record(TelemetryEvent::INP(
        anima_observatory::InpEvent::NegotiationCompleted {
            intent_id: "intent-001".into(),
            winner: "cog-1".into(),
        },
    ));

    collector.record(TelemetryEvent::CK(
        anima_observatory::CkEvent::InferenceScheduled {
            inference_id: "inf-001".into(),
            priority: "high".into(),
        },
    ));
    collector.record(TelemetryEvent::CK(
        anima_observatory::CkEvent::ThoughtBubbleCreated {
            bubble_id: "bubble-001".into(),
        },
    ));

    collector.record(TelemetryEvent::ZKPA(
        anima_observatory::ZkpaEvent::SessionCreated {
            session_id: "sess-001".into(),
            agent_id: "cog-1".into(),
        },
    ));
    collector.record(TelemetryEvent::ZKPA(
        anima_observatory::ZkpaEvent::AttestationVerified {
            platform: "tee".into(),
        },
    ));
    collector.record(TelemetryEvent::ZKPA(
        anima_observatory::ZkpaEvent::LivenessHeartbeat {
            agent_id: "cog-1".into(),
            sequence: 42,
        },
    ));

    // ── Print summary ─────────────────────────────────────────────────────────

    let LayerCounts { sse, inp, ck, zkpa } = collector.count_by_layer();
    println!("Telemetry Summary:");
    println!("  Total events : {}", collector.events().len());
    println!("  SSE events   : {}", sse);
    println!("  INP events   : {}", inp);
    println!("  CK events    : {}", ck);
    println!("  ZKPA events  : {}", zkpa);
    println!();

    println!("Recent events (last 3):");
    for ev in collector.recent_events(3) {
        println!("  {:?}", ev);
    }

    // ── Show HTTP API response ────────────────────────────────────────────────

    if let Some(health) = anima_observatory::handle_request("/api/health", &collector) {
        println!("\nHTTP API /api/health: {}", health);
    }
    if let Some(counts) = anima_observatory::handle_request("/api/telemetry/count", &collector) {
        println!("HTTP API /api/telemetry/count: {}", counts);
    }

    println!("\nObservatory ready.");
}
