//! Unified Storage Fabric for the Semantic State Engine.
//!
//! Co-locates vectors, causal graphs, and raw payloads at the disk level.
//! Provides write/read/prune lifecycle with attention-based retrieval.

use std::collections::HashMap;

use crate::memory_node::{MemoryId, MemoryNode};

// ---------------------------------------------------------------------------
// InMemoryStorage — reference implementation
// ---------------------------------------------------------------------------

/// In-memory storage backend for the SSE.
///
/// In production, this would be backed by a memory-mapped file with co-located
/// vectors + graph + payloads. This implementation provides the correct API
/// surface and can be swapped for a persistent backend.
pub struct InMemoryStorage {
    nodes: HashMap<MemoryId, MemoryNode>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Persist a [`MemoryNode`] to storage.
    pub fn write_node(&mut self, node: MemoryNode) {
        self.nodes.insert(node.id, node);
    }

    /// Load a [`MemoryNode`] by its id.
    pub fn read_node(&self, id: &MemoryId) -> Option<&MemoryNode> {
        self.nodes.get(id)
    }

    /// Load a mutable reference to a [`MemoryNode`].
    pub fn read_node_mut(&mut self, id: &MemoryId) -> Option<&mut MemoryNode> {
        self.nodes.get_mut(id)
    }

    /// Remove a node from storage. Returns the removed node if it existed.
    pub fn delete_node(&mut self, id: &MemoryId) -> Option<MemoryNode> {
        self.nodes.remove(id)
    }

    /// Attention-based retrieval: return nodes ranked by `current_strength`
    /// (descending), not by query similarity.
    ///
    /// This implements the SSE principle that memory should be *attended to*,
    /// not *queried*. The strongest (most relevant) memories surface first.
    pub fn attention_retrieve(&self, limit: usize) -> Vec<&MemoryNode> {
        let mut nodes: Vec<&MemoryNode> = self.nodes.values().collect();
        nodes.sort_by(|a, b| {
            b.current_strength
                .partial_cmp(&a.current_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes.into_iter().take(limit).collect()
    }

    /// Retrieve nodes whose strength is above a minimum threshold.
    pub fn retrieve_above_threshold(&self, min_strength: f32) -> Vec<&MemoryNode> {
        self.nodes
            .values()
            .filter(|n| n.current_strength >= min_strength)
            .collect()
    }

    /// Retrieve nodes matching a specific sensory modality.
    pub fn retrieve_by_modality(
        &self,
        modality: &crate::memory_node::SensoryModality,
    ) -> Vec<&MemoryNode> {
        self.nodes
            .values()
            .filter(|n| n.sensory_modality == *modality)
            .collect()
    }

    /// Retrieve nodes whose payload matches a given MIME type prefix.
    pub fn retrieve_by_format(&self, mime_prefix: &str) -> Vec<&MemoryNode> {
        self.nodes
            .values()
            .filter(|n| n.payload_format.starts_with(mime_prefix))
            .collect()
    }

    /// Retrieve nodes by tag.
    pub fn retrieve_by_tag(&self, tag: &str) -> Vec<&MemoryNode> {
        self.nodes
            .values()
            .filter(|n| n.metadata.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Remove all nodes where [`should_prune`](MemoryNode::should_prune) is true.
    ///
    /// Returns the number of pruned nodes.
    pub fn prune_pass(&mut self) -> usize {
        let before = self.nodes.len();
        self.nodes.retain(|_, node| !node.should_prune());
        before - self.nodes.len()
    }

    /// Number of stored nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total payload bytes across all stored nodes.
    pub fn total_payload_bytes(&self) -> usize {
        self.nodes.values().map(|n| n.payload.len()).sum()
    }

    /// Iterate over all stored node IDs.
    pub fn node_ids(&self) -> Vec<MemoryId> {
        self.nodes.keys().copied().collect()
    }

    /// Iterate over all stored nodes.
    pub fn iter(&self) -> impl Iterator<Item = &MemoryNode> {
        self.nodes.values()
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
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
            b"test payload".to_vec(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            AGENT,
        );
        node.current_strength = strength;
        node.metadata.tags.push("test".to_string());
        node
    }

    #[test]
    fn write_and_read_node() {
        let mut storage = InMemoryStorage::new();
        let node = make_node(1, 0.9);
        let id = node.id;
        storage.write_node(node);

        let loaded = storage.read_node(&id).unwrap();
        assert_eq!(loaded.id, [1u8; 32]);
        assert!((loaded.current_strength - 0.9).abs() < 1e-6);
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let storage = InMemoryStorage::new();
        assert!(storage.read_node(&[99u8; 32]).is_none());
    }

    #[test]
    fn delete_node() {
        let mut storage = InMemoryStorage::new();
        let node = make_node(1, 0.5);
        let id = node.id;
        storage.write_node(node);

        let removed = storage.delete_node(&id).unwrap();
        assert_eq!(removed.id, [1u8; 32]);
        assert!(storage.read_node(&id).is_none());
    }

    #[test]
    fn attention_retrieve_returns_strongest_first() {
        let mut storage = InMemoryStorage::new();
        storage.write_node(make_node(1, 0.3));
        storage.write_node(make_node(2, 0.9));
        storage.write_node(make_node(3, 0.6));

        let results = storage.attention_retrieve(2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, [2u8; 32]); // 0.9
        assert_eq!(results[1].id, [3u8; 32]); // 0.6
    }

    #[test]
    fn prune_pass_removes_weak_nodes() {
        let mut storage = InMemoryStorage::new();
        storage.write_node(make_node(1, 0.9));
        storage.write_node(make_node(2, 0.01)); // below default threshold 0.05
        storage.write_node(make_node(3, 0.8));

        let pruned = storage.prune_pass();
        assert_eq!(pruned, 1);
        assert_eq!(storage.node_count(), 2);
    }

    #[test]
    fn node_count_and_payload_bytes() {
        let mut storage = InMemoryStorage::new();
        storage.write_node(make_node(1, 0.9));
        storage.write_node(make_node(2, 0.8));

        assert_eq!(storage.node_count(), 2);
        // Each node has 12 bytes payload
        assert_eq!(storage.total_payload_bytes(), 24);
    }

    #[test]
    fn retrieve_by_tag() {
        let mut storage = InMemoryStorage::new();
        storage.write_node(make_node(1, 0.9));
        storage.write_node(make_node(2, 0.8));

        let results = storage.retrieve_by_tag("test");
        assert_eq!(results.len(), 2);
    }
}
