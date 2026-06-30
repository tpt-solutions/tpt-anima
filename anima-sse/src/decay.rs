//! Temporal Decay Engine for the Semantic State Engine.
//!
//! Provides batch decay operations over collections of [`MemoryNode`]s and
//! configurable decay profiles for different memory types (episodic, semantic,
//! procedural).

use crate::memory_node::{DecayProfile, MemoryNode};

// ---------------------------------------------------------------------------
// Memory type decay presets
// ---------------------------------------------------------------------------

/// Predefined decay profiles calibrated for different memory types.
///
/// - **Episodic** — fast decay (events fade quickly unless reinforced).
/// - **Semantic** — slow decay (facts persist longer).
/// - **Procedural** — very slow decay (skills are sticky).
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
}

impl MemoryType {
    /// Return a [`DecayProfile`] calibrated for this memory type.
    pub fn decay_profile(&self) -> DecayProfile {
        match self {
            MemoryType::Episodic => DecayProfile {
                decay_rate: 0.05,
                reinforcement_factor: 1.8,
                max_strength: 1.0,
                pruning_threshold: 0.1,
                last_decay_at: std::time::SystemTime::now(),
                pinned: false,
            },
            MemoryType::Semantic => DecayProfile {
                decay_rate: 0.005,
                reinforcement_factor: 1.3,
                max_strength: 1.0,
                pruning_threshold: 0.03,
                last_decay_at: std::time::SystemTime::now(),
                pinned: false,
            },
            MemoryType::Procedural => DecayProfile {
                decay_rate: 0.001,
                reinforcement_factor: 1.1,
                max_strength: 1.0,
                pruning_threshold: 0.02,
                last_decay_at: std::time::SystemTime::now(),
                pinned: false,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Batch decay operations
// ---------------------------------------------------------------------------

/// Apply temporal decay to all non-pinned nodes in a collection.
///
/// Returns the number of nodes that were decayed (pinned nodes are skipped).
pub fn apply_batch_decay(nodes: &mut [MemoryNode]) -> usize {
    let mut count = 0;
    for node in nodes.iter_mut() {
        if !node.decay_profile.pinned {
            node.apply_decay();
            count += 1;
        }
    }
    count
}

/// Apply temporal decay for an explicit elapsed duration to all non-pinned nodes.
///
/// Useful for testing or simulating time passage without real-time waits.
pub fn apply_batch_decay_for(nodes: &mut [MemoryNode], elapsed: std::time::Duration) {
    for node in nodes.iter_mut() {
        node.apply_decay_for(elapsed);
    }
}

/// Identify all nodes eligible for pruning (strength < pruning_threshold).
///
/// Returns indices of nodes that should be pruned.
pub fn find_prunable(nodes: &[MemoryNode]) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.should_prune())
        .map(|(i, _)| i)
        .collect()
}

/// Remove all nodes where [`should_prune`](MemoryNode::should_prune) is true.
///
/// Returns the number of pruned nodes.
pub fn prune_collection(nodes: &mut Vec<MemoryNode>) -> usize {
    let before = nodes.len();
    nodes.retain(|node| !node.should_prune());
    before - nodes.len()
}

/// Compute aggregate statistics for a collection of memory nodes.
pub fn collection_stats(nodes: &[MemoryNode]) -> CollectionStats {
    if nodes.is_empty() {
        return CollectionStats::default();
    }

    let total = nodes.len() as f64;
    let total_strength: f32 = nodes.iter().map(|n| n.current_strength).sum();
    let total_ledger_cost: f64 = nodes.iter().flat_map(|n| &n.ledger).map(|e| e.cost).sum();

    let pinned_count = nodes.iter().filter(|n| n.decay_profile.pinned).count();
    let prunable_count = nodes.iter().filter(|n| n.should_prune()).count();

    CollectionStats {
        total_nodes: nodes.len(),
        avg_strength: total_strength / total as f32,
        min_strength: nodes
            .iter()
            .map(|n| n.current_strength)
            .fold(f32::INFINITY, f32::min),
        max_strength: nodes
            .iter()
            .map(|n| n.current_strength)
            .fold(f32::NEG_INFINITY, f32::max),
        pinned_count,
        prunable_count,
        total_ledger_cost,
    }
}

/// Aggregate statistics for a memory collection.
#[derive(Debug, Clone, Default)]
pub struct CollectionStats {
    pub total_nodes: usize,
    pub avg_strength: f32,
    pub min_strength: f32,
    pub max_strength: f32,
    pub pinned_count: usize,
    pub prunable_count: usize,
    pub total_ledger_cost: f64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_node::{AgentId, SensoryModality, VectorEmbedding};

    const AGENT: AgentId = [1u8; 32];

    fn make_node(id: u8, strength: f32) -> MemoryNode {
        let mut node = MemoryNode::new(
            [id; 32],
            VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
            b"test".to_vec(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            AGENT,
        );
        node.current_strength = strength;
        node
    }

    #[test]
    fn memory_type_profiles_are_distinct() {
        let epi = MemoryType::Episodic.decay_profile();
        let sem = MemoryType::Semantic.decay_profile();
        let pro = MemoryType::Procedural.decay_profile();

        assert!(epi.decay_rate > sem.decay_rate);
        assert!(sem.decay_rate > pro.decay_rate);
    }

    #[test]
    fn apply_batch_decay_skips_pinned() {
        let mut nodes = vec![make_node(1, 0.5), make_node(2, 0.5)];
        nodes[1].decay_profile.pinned = true;

        let count = apply_batch_decay(&mut nodes);
        assert_eq!(count, 1);
    }

    #[test]
    fn apply_batch_decay_for_reduces_strength() {
        let mut nodes = vec![make_node(1, 1.0), make_node(2, 1.0)];
        apply_batch_decay_for(&mut nodes, std::time::Duration::from_secs(3600));

        for node in &nodes {
            assert!(node.current_strength < 1.0);
        }
    }

    #[test]
    fn find_prunable_identifies_weak_nodes() {
        let mut nodes = vec![make_node(1, 1.0), make_node(2, 0.01), make_node(3, 0.001)];
        nodes[2].decay_profile.pinned = true; // pinned, should not be prunable

        let prunable = find_prunable(&nodes);
        assert!(prunable.contains(&1)); // node[1] strength 0.01 < 0.05
        assert!(!prunable.contains(&2)); // node[2] is pinned
        assert!(!prunable.contains(&0)); // node[0] strength 1.0
    }

    #[test]
    fn prune_collection_removes_weak_nodes() {
        let mut nodes = vec![make_node(1, 1.0), make_node(2, 0.01), make_node(3, 0.001)];
        let pruned = prune_collection(&mut nodes);
        // Both node[1] (0.01) and node[2] (0.001) are below threshold 0.05
        assert_eq!(pruned, 2);
        assert_eq!(nodes.len(), 1);
        assert!((nodes[0].current_strength - 1.0).abs() < 1e-6);
    }

    #[test]
    fn collection_stats_empty() {
        let stats = collection_stats(&[]);
        assert_eq!(stats.total_nodes, 0);
    }

    #[test]
    fn collection_stats_computes_correctly() {
        let nodes = vec![make_node(1, 0.8), make_node(2, 0.4)];
        let stats = collection_stats(&nodes);
        assert_eq!(stats.total_nodes, 2);
        assert!((stats.avg_strength - 0.6).abs() < 1e-6);
        assert!((stats.min_strength - 0.4).abs() < 1e-6);
        assert!((stats.max_strength - 0.8).abs() < 1e-6);
        assert_eq!(stats.prunable_count, 0);
    }
}
