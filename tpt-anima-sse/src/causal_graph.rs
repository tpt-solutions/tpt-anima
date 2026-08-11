//! Causal Graph Engine for the Semantic State Engine.
//!
//! Provides graph storage and traversal operations over [`MemoryNode`] causal
//! links, enabling context-chain retrieval and causal integrity validation.

use std::collections::{HashMap, VecDeque};

use crate::memory_node::{CausalLink, CausalLinkType, MemoryId, MemoryNode};

// ---------------------------------------------------------------------------
// CausalGraph
// ---------------------------------------------------------------------------

/// Adjacency-list graph keyed by [`MemoryId`].
///
/// Each node maps to a list of outgoing [`CausalLink`]s. The graph supports
/// BFS/DFS traversal, context-chain retrieval, and cycle/contradiction detection.
pub struct CausalGraph {
    /// source_id → outgoing links
    adjacency: HashMap<MemoryId, Vec<CausalLink>>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
        }
    }

    /// Register a memory node in the graph (creates an entry even if it has
    /// no outgoing links).
    pub fn register_node(&mut self, node_id: MemoryId) {
        self.adjacency.entry(node_id).or_default();
    }

    /// Add a directed causal link from `source_id` to `target_id`.
    ///
    /// Auto-registers both nodes if absent. Returns `Err` if the link would
    /// create a self-loop.
    pub fn add_link(
        &mut self,
        source_id: MemoryId,
        target_id: MemoryId,
        link: CausalLink,
    ) -> Result<(), CausalError> {
        if source_id == target_id {
            return Err(CausalError::SelfLoop { node_id: source_id });
        }
        self.register_node(source_id);
        self.register_node(target_id);
        self.adjacency.get_mut(&source_id).unwrap().push(link);
        Ok(())
    }

    /// Get all outgoing links for a node.
    pub fn outgoing_links(&self, node_id: &MemoryId) -> Option<&[CausalLink]> {
        self.adjacency.get(node_id).map(|v| v.as_slice())
    }

    /// Retrieve the full antecedent chain for a node via BFS.
    ///
    /// Returns nodes in breadth-first order from the target back to roots.
    /// Each entry is `(node_id, link_type, strength)`.
    pub fn get_context_chain(
        &self,
        target_id: &MemoryId,
        max_depth: usize,
    ) -> Vec<(MemoryId, CausalLinkType, f32)> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Seed: find all nodes that link TO target_id (reverse traversal)
        for (src_id, links) in &self.adjacency {
            for link in links {
                if link.target_id == *target_id {
                    queue.push_back((*src_id, link.link_type.clone(), link.strength, 0));
                }
            }
        }

        while let Some((node_id, link_type, strength, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > max_depth {
                continue;
            }
            visited.insert(node_id);
            result.push((node_id, link_type, strength));

            // Continue backward: find nodes that link TO this node
            for (src_id, links) in &self.adjacency {
                for link in links {
                    if link.target_id == node_id && !visited.contains(src_id) {
                        queue.push_back((
                            *src_id,
                            link.link_type.clone(),
                            link.strength,
                            depth + 1,
                        ));
                    }
                }
            }
        }

        result
    }

    /// BFS traversal from a starting node, returning visited node IDs in order.
    pub fn bfs(&self, start: &MemoryId) -> Vec<MemoryId> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back(*start);
        while let Some(node_id) = queue.pop_front() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);
            result.push(node_id);

            if let Some(links) = self.adjacency.get(&node_id) {
                for link in links {
                    if !visited.contains(&link.target_id) {
                        queue.push_back(link.target_id);
                    }
                }
            }
        }

        result
    }

    /// DFS traversal from a starting node, returning visited node IDs in
    /// pre-order.
    pub fn dfs(&self, start: &MemoryId) -> Vec<MemoryId> {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![*start];
        let mut result = Vec::new();

        while let Some(node_id) = stack.pop() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id);
            result.push(node_id);

            if let Some(links) = self.adjacency.get(&node_id) {
                // Push in reverse so the first link is processed first
                for link in links.iter().rev() {
                    if !visited.contains(&link.target_id) {
                        stack.push(link.target_id);
                    }
                }
            }
        }

        result
    }

    /// Detect directed cycles in the graph.
    ///
    /// Uses DFS with coloring (white/gray/black). Returns IDs of nodes that
    /// participate in at least one cycle.
    pub fn detect_cycles(&self) -> Vec<MemoryId> {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }

        let mut color: HashMap<MemoryId, Color> = self
            .adjacency
            .keys()
            .map(|&id| (id, Color::White))
            .collect();

        let mut cycles = Vec::new();

        fn dfs_cycle(
            node: MemoryId,
            adjacency: &HashMap<MemoryId, Vec<CausalLink>>,
            color: &mut HashMap<MemoryId, Color>,
            cycles: &mut Vec<MemoryId>,
        ) {
            color.insert(node, Color::Gray);

            if let Some(links) = adjacency.get(&node) {
                for link in links {
                    match color.get(&link.target_id) {
                        Some(Color::Gray) => {
                            // Back edge — cycle detected
                            if !cycles.contains(&link.target_id) {
                                cycles.push(link.target_id);
                            }
                        }
                        Some(Color::White) => {
                            dfs_cycle(link.target_id, adjacency, color, cycles);
                        }
                        _ => {}
                    }
                }
            }

            color.insert(node, Color::Black);
        }

        let nodes: Vec<MemoryId> = self.adjacency.keys().copied().collect();
        for &node in &nodes {
            if color.get(&node) == Some(&Color::White) {
                dfs_cycle(node, &self.adjacency, &mut color, &mut cycles);
            }
        }

        cycles
    }

    /// Detect contradictions: pairs of nodes linked by [`CausalLinkType::Contradicts`]
    /// that are both above a given strength threshold.
    pub fn detect_contradictions(&self, min_strength: f32) -> Vec<(MemoryId, MemoryId)> {
        let mut contradictions = Vec::new();

        for (src_id, links) in &self.adjacency {
            for link in links {
                if link.link_type == CausalLinkType::Contradicts && link.strength >= min_strength {
                    contradictions.push((*src_id, link.target_id));
                }
            }
        }

        contradictions
    }

    /// Validate causal integrity: no cycles AND no high-strength contradictions.
    pub fn validate_causal_integrity(
        &self,
        contradiction_threshold: f32,
    ) -> Result<(), CausalError> {
        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            return Err(CausalError::CyclesDetected { node_ids: cycles });
        }

        let contradictions = self.detect_contradictions(contradiction_threshold);
        if !contradictions.is_empty() {
            return Err(CausalError::ContradictionsDetected {
                pairs: contradictions,
            });
        }

        Ok(())
    }

    /// Total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.adjacency.len()
    }

    /// Total number of directed edges.
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Build from MemoryNodes
// ---------------------------------------------------------------------------

impl CausalGraph {
    /// Build a causal graph from a slice of [`MemoryNode`]s.
    ///
    /// Registers each node and copies its outgoing causal links into the graph.
    pub fn from_nodes(nodes: &[MemoryNode]) -> Self {
        let mut graph = Self::new();
        for node in nodes {
            graph.register_node(node.id);
            for link in &node.causal_links {
                let _ = graph.add_link(node.id, link.target_id, link.clone());
            }
        }
        graph
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum CausalError {
    #[error("self-loop detected on node {node_id:?}")]
    SelfLoop { node_id: MemoryId },

    #[error("causal cycles detected involving {node_ids:?}")]
    CyclesDetected { node_ids: Vec<MemoryId> },

    #[error("contradictions detected between {pairs:?}")]
    ContradictionsDetected { pairs: Vec<(MemoryId, MemoryId)> },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_node::{AgentId, SensoryModality, VectorEmbedding};

    const AGENT: AgentId = [1u8; 32];
    const ID_A: MemoryId = [1u8; 32];
    const ID_B: MemoryId = [2u8; 32];
    const ID_C: MemoryId = [3u8; 32];

    fn make_node(id: MemoryId) -> MemoryNode {
        MemoryNode::new(
            id,
            VectorEmbedding::new(vec![1.0, 0.0]),
            b"test".to_vec(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            AGENT,
        )
    }

    #[test]
    fn add_link_and_query() {
        let mut graph = CausalGraph::new();
        let link = CausalLink::new(ID_B, CausalLinkType::Caused, 0.9);
        graph.add_link(ID_A, ID_B, link).unwrap();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let outgoing = graph.outgoing_links(&ID_A).unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target_id, ID_B);
    }

    #[test]
    fn self_loop_returns_error() {
        let mut graph = CausalGraph::new();
        let link = CausalLink::new(ID_A, CausalLinkType::Caused, 0.5);
        assert!(graph.add_link(ID_A, ID_A, link).is_err());
    }

    #[test]
    fn bfs_traversal_order() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_B,
                ID_C,
                CausalLink::new(ID_C, CausalLinkType::Preceded, 0.7),
            )
            .unwrap();

        let order = graph.bfs(&ID_A);
        assert_eq!(order, vec![ID_A, ID_B, ID_C]);
    }

    #[test]
    fn dfs_traversal_order() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_A,
                ID_C,
                CausalLink::new(ID_C, CausalLinkType::SimilarTo, 0.5),
            )
            .unwrap();

        let order = graph.dfs(&ID_A);
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], ID_A);
    }

    #[test]
    fn detect_cycle() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_B,
                ID_C,
                CausalLink::new(ID_C, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_C,
                ID_A,
                CausalLink::new(ID_A, CausalLinkType::Caused, 0.9),
            )
            .unwrap();

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn no_cycle_in_dag() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_B,
                ID_C,
                CausalLink::new(ID_C, CausalLinkType::Caused, 0.9),
            )
            .unwrap();

        let cycles = graph.detect_cycles();
        assert!(cycles.is_empty());
    }

    #[test]
    fn detect_contradictions() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Contradicts, 0.8),
            )
            .unwrap();

        let contra = graph.detect_contradictions(0.5);
        assert_eq!(contra.len(), 1);
    }

    #[test]
    fn validate_causal_integrity_passes() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();

        assert!(graph.validate_causal_integrity(0.5).is_ok());
    }

    #[test]
    fn validate_causal_integrity_fails_on_cycle() {
        let mut graph = CausalGraph::new();
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_B,
                ID_A,
                CausalLink::new(ID_A, CausalLinkType::Caused, 0.9),
            )
            .unwrap();

        assert!(graph.validate_causal_integrity(0.5).is_err());
    }

    #[test]
    fn from_nodes_copies_links() {
        let mut node_a = make_node(ID_A);
        node_a.add_causal_link(CausalLink::new(ID_B, CausalLinkType::Caused, 0.9));

        let node_b = make_node(ID_B);
        let graph = CausalGraph::from_nodes(&[node_a, node_b]);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn get_context_chain_finds_ancestors() {
        let mut graph = CausalGraph::new();
        // A → B → C
        graph
            .add_link(
                ID_A,
                ID_B,
                CausalLink::new(ID_B, CausalLinkType::Caused, 0.9),
            )
            .unwrap();
        graph
            .add_link(
                ID_B,
                ID_C,
                CausalLink::new(ID_C, CausalLinkType::Caused, 0.7),
            )
            .unwrap();

        let chain = graph.get_context_chain(&ID_C, 10);
        // Should find B (direct parent) and A (grandparent)
        assert!(chain.iter().any(|(id, _, _)| *id == ID_B));
        assert!(chain.iter().any(|(id, _, _)| *id == ID_A));
    }
}
