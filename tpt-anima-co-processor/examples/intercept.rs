//! Example: record an intercepted AI workload and translate it into an INP intent.
//!
//! Run with: `cargo run -p tpt-anima-co-processor --example intercept`

use tpt_anima_co_processor::{CoProcessor, CoProcessorConfig, WorkloadIntercept, WorkloadType};

fn main() {
    let mut co_proc = CoProcessor::new(CoProcessorConfig::default()).expect("config valid");

    // Start intercepting (no-op off Linux; safe on every platform).
    co_proc.start().expect("start succeeds");

    // Record an intercepted inference workload.
    let intercept = WorkloadIntercept::new(
        WorkloadType::Inference,
        std::process::id() as u32,
        "edge-tiny".into(),
        serde_json::json!({ "model_id": "edge-tiny", "prompt": "translate hello" }),
    );
    co_proc.record_intercept(intercept);

    // Translate the most recent intercept into a native INP intent.
    let last = co_proc
        .recent_intercepts(1)
        .first()
        .expect("has an intercept");
    let intent = last
        .to_intent()
        .expect("known workload translates to an intent");
    println!("Translated intercepted workload into intent: {intent}");

    co_proc.stop();
}
