//! Example: run the lightweight Tier 3 Edge Runtime — store a memory, process
//! an intent, and build a cross-tier envelope to forward to the Core Hypervisor.
//!
//! Run with: `cargo run -p tpt-anima-edge-runtime --example run`

use tpt_anima_edge_runtime::{
    CrossTierPayload, EdgeIntent, EdgeRuntime, EnvelopeBuilder, IntentMessage, Tier,
};

fn main() {
    let mut runtime = EdgeRuntime::new();

    // Register a handler for the inference action before processing it.
    runtime.register_handler("inference.run", |intent| {
        let model = intent
            .payload
            .get("model_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        Ok(serde_json::json!({ "status": "ok", "model": model }))
    });

    // Process an intent through the edge runtime.
    let intent = EdgeIntent::new(
        "inference.run".into(),
        serde_json::json!({ "model_id": "edge-tiny", "prompt": "translate hello" }),
    );
    let result = runtime
        .process(&intent)
        .expect("intent processing succeeds");
    println!("Intent result: {result}");

    // Store a memory entry on the edge device.
    runtime
        .memory()
        .store(b"recent translation".to_vec(), vec!["nlp".into()]);

    // Inspect runtime statistics.
    let stats = runtime.stats();
    println!(
        "Runtime stats: total_processed={}, memory_entries={}, memory_bytes={}",
        stats.total_processed, stats.memory_entries, stats.memory_bytes
    );

    // Build a cross-tier envelope forwarding an intent upward to the hypervisor.
    let source = uuid::Uuid::new_v4();
    let target = uuid::Uuid::new_v4();
    let envelope = EnvelopeBuilder::new(
        source,
        Tier::EdgeRuntime,
        Tier::CoreHypervisor,
        CrossTierPayload::IntentForward(IntentMessage {
            intent_id: uuid::Uuid::new_v4(),
            action: "inference.run".into(),
            payload: serde_json::json!({ "model_id": "edge-tiny" }),
            qos: tpt_anima_edge_runtime::IntentQoS {
                max_latency_ms: 1500,
                min_accuracy: 0.9,
                cost_budget: 100,
            },
            origin_tier: Tier::EdgeRuntime,
        }),
    )
    .target(target)
    .ttl(8)
    .build();

    let bytes = tpt_anima_edge_runtime::serialize_envelope(&envelope).expect("serializes");
    let restored = tpt_anima_edge_runtime::deserialize_envelope(&bytes).expect("round-trips");
    assert_eq!(restored.envelope_id, envelope.envelope_id);
    println!(
        "Cross-tier envelope {} (Edge→Core) serialized to {} bytes.",
        restored.envelope_id,
        bytes.len()
    );
}
