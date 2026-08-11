//! SSE — Single-Agent Memory Lifecycle
//!
//! Mirrors `DEMO_SCENARIOS.md` Demo 1: ingest a memory, reinforce it via
//! `record_access()`, then watch it decay via `apply_decay_for()`.
//!
//! Run with: `cargo run -p tpt-anima-sse --example memory_lifecycle`

use std::time::Duration;

use tpt_anima_sse::{InMemoryStorage, MemoryNode, SensoryModality, VectorEmbedding};

fn main() {
    let mut storage = InMemoryStorage::new();

    let node_id = [1u8; 32];
    let agent_id = [9u8; 32];

    let mut node = MemoryNode::new(
        node_id,
        VectorEmbedding::new(vec![0.12, 0.44, -0.08, 0.91]),
        b"The capital of France is Paris.".to_vec(),
        "text/plain".into(),
        SensoryModality::Textual,
        agent_id,
    );
    println!(
        "1. Ingested memory — strength = {:.4}",
        node.current_strength
    );

    // Time passes with no access: exponential decay kicks in.
    node.apply_decay_for(Duration::from_secs(24 * 3600));
    println!(
        "2. After 24h of decay — strength = {:.4}",
        node.current_strength
    );

    // Agent queries the same fact again: record_access() reinforces it.
    node.record_access(agent_id);
    println!(
        "3. Reinforced via record_access() — strength = {:.4}",
        node.current_strength
    );

    node.apply_decay_for(Duration::from_secs(6 * 24 * 3600));
    println!(
        "4. After 6 more days of decay — strength = {:.4}",
        node.current_strength
    );

    println!(
        "5. should_prune() = {} (pruning_threshold = {:.2})",
        node.should_prune(),
        node.decay_profile.pruning_threshold
    );

    storage.write_node(node);
    println!("\nStorage now holds {} node(s).", storage.node_count());
}
