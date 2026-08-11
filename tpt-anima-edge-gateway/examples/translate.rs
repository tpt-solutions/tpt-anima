//! Example: translate a native INP intent into MCP and A2A, and back.
//!
//! Run with: `cargo run -p tpt-anima-edge-gateway --example translate`

use serde_json::json;
use tpt_anima_edge_gateway::{
    A2aToInpTranslator, InpIntent, InpToA2aTranslator, InpToMcpTranslator, McpToInpTranslator,
};

fn main() {
    let intent = InpIntent {
        id: "intent-42".into(),
        action: "summarize".into(),
        payload: json!({ "message": "hello world", "doc": "report.pdf" }),
        max_cost: Some(0.25),
        max_latency_ms: Some(1500),
    };

    // Native INP → MCP
    let mcp = InpToMcpTranslator::translate(&intent);
    println!(
        "MCP payload:\n{}\n",
        serde_json::to_string_pretty(&mcp).unwrap()
    );

    // MCP → INP (round-trips back to a native intent)
    let back_from_mcp = McpToInpTranslator::translate(&mcp).unwrap();
    assert_eq!(back_from_mcp.id, intent.id);
    assert_eq!(back_from_mcp.action, intent.action);

    // Native INP → A2A
    let a2a = InpToA2aTranslator::translate(&intent);
    println!(
        "A2A payload:\n{}\n",
        serde_json::to_string_pretty(&a2a).unwrap()
    );

    // A2A → INP (round-trips back to a native intent)
    let back_from_a2a = A2aToInpTranslator::translate(&a2a).unwrap();
    assert_eq!(back_from_a2a.id, intent.id);
    assert_eq!(back_from_a2a.action, intent.action);

    println!("All four translations round-tripped successfully.");
}
