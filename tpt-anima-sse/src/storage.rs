//! Unified Storage Fabric for the Semantic State Engine.
//!
//! Co-locates vectors, causal graphs, and raw payloads at the disk level.
//! Provides write/read/prune lifecycle with attention-based retrieval.

use std::collections::HashMap;

use crate::ann::AnnIndex;
use crate::memory_node::{MemoryId, MemoryNode, VectorEmbedding};

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
    /// Approximate nearest-neighbor index for semantic (cosine) retrieval.
    ann: AnnIndex,
    /// Rebuild the coarse quantizer once the index grows by this many inserts.
    ann_rebuild_stride: usize,
    ann_dirty_since_rebuild: usize,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            ann: AnnIndex::new(0),
            ann_rebuild_stride: 256,
            ann_dirty_since_rebuild: 0,
        }
    }

    /// Persist a [`MemoryNode`] to storage.
    pub fn write_node(&mut self, node: MemoryNode) {
        self.ann.insert_embedding(node.id, &node.embedding);
        self.ann_dirty_since_rebuild += 1;
        self.maybe_rebuild_ann();
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
        self.ann.remove(id);
        self.ann_dirty_since_rebuild += 1;
        self.maybe_rebuild_ann();
        self.nodes.remove(id)
    }

    /// Rebuild the ANN coarse quantizer if enough inserts/deletes have
    /// accumulated since the last build. Cheap no-op when under the stride.
    fn maybe_rebuild_ann(&mut self) {
        if self.ann_dirty_since_rebuild >= self.ann_rebuild_stride {
            self.ann.rebuild();
            self.ann_dirty_since_rebuild = 0;
        }
    }

    /// Force a rebuild of the approximate nearest-neighbor index. Call after
    /// bulk loads or before a latency-critical semantic search so the coarse
    /// quantizer reflects the current memory state.
    pub fn rebuild_semantic_index(&mut self) {
        self.ann.rebuild();
        self.ann_dirty_since_rebuild = 0;
    }

    /// Approximate nearest-neighbor semantic retrieval.
    ///
    /// Returns up to `k` stored nodes ordered by cosine similarity to `query`
    /// (descending), using the inverted-file index. Unlike
    /// [`attention_retrieve`](Self::attention_retrieve) this is *query-ranked*
    /// rather than *weight-ranked* — it finds the semantically closest memories
    /// rather than the strongest ones.
    pub fn semantic_search(&self, query: &VectorEmbedding, k: usize) -> Vec<&MemoryNode> {
        self.ann
            .search_embedding(query, k)
            .into_iter()
            .filter_map(|(id, _)| self.nodes.get(&id))
            .collect()
    }

    /// Number of vectors held in the approximate nearest-neighbor index.
    pub fn semantic_index_size(&self) -> usize {
        self.ann.len()
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
    /// Returns the number of pruned nodes. Pruned nodes are also evicted from
    /// the approximate nearest-neighbor index.
    pub fn prune_pass(&mut self) -> usize {
        let before = self.nodes.len();
        let removed: Vec<MemoryId> = self
            .nodes
            .iter()
            .filter(|(_, node)| node.should_prune())
            .map(|(id, _)| *id)
            .collect();
        for id in &removed {
            self.ann.remove(id);
        }
        self.ann_dirty_since_rebuild += removed.len();
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

    #[test]
    fn semantic_search_returns_nearest_by_cosine() {
        let mut storage = InMemoryStorage::new();
        // Two poles on a 2-D embedding space.
        let mut a = MemoryNode::new(
            [1u8; 32],
            VectorEmbedding::new(vec![1.0, 0.0]),
            b"alpha".to_vec(),
            "text/plain".into(),
            SensoryModality::Textual,
            AGENT,
        );
        a.current_strength = 0.1; // weak, so attention-rank would lose it
        let mut b = MemoryNode::new(
            [2u8; 32],
            VectorEmbedding::new(vec![0.0, 1.0]),
            b"beta".to_vec(),
            "text/plain".into(),
            SensoryModality::Textual,
            AGENT,
        );
        b.current_strength = 0.9; // strong
        storage.write_node(a);
        storage.write_node(b);
        storage.rebuild_semantic_index();

        // Query near pole A must surface the semantically closest node first,
        // regardless of its low attention weight.
        let query = VectorEmbedding::new(vec![0.99, 0.01]);
        let results = storage.semantic_search(&query, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, [1u8; 32]);
    }

    #[test]
    fn semantic_index_evicts_on_prune_and_delete() {
        let mut storage = InMemoryStorage::new();
        storage.write_node(make_node(1, 0.9));
        storage.write_node(make_node(2, 0.01)); // below threshold
        storage.rebuild_semantic_index();
        assert_eq!(storage.semantic_index_size(), 2);

        storage.delete_node(&[1u8; 32]);
        assert_eq!(storage.semantic_index_size(), 1);

        storage.write_node(make_node(2, 0.01));
        storage.prune_pass();
        assert_eq!(storage.semantic_index_size(), 0);
    }
}
