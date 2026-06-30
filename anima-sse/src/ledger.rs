//! Compute Ledger — SSE Portion.
//!
//! Tracks micro-costs per memory access and provides aggregate cost analysis
//! across the SSE.

use crate::memory_node::{AgentId, ComputeLedgerEntry, LedgerOperation, MemoryNode};

// ---------------------------------------------------------------------------
// Cost configuration
// ---------------------------------------------------------------------------

/// Cost rates for different memory operations (in Compute Credits).
#[derive(Debug, Clone)]
pub struct CostRates {
    pub read: f64,
    pub write: f64,
    pub decay: f64,
    pub prune: f64,
}

impl Default for CostRates {
    fn default() -> Self {
        Self {
            read: 0.001,
            write: 0.01,
            decay: 0.0,
            prune: 0.0005,
        }
    }
}

// ---------------------------------------------------------------------------
// Ledger — per-node cost tracking
// ---------------------------------------------------------------------------

/// Compute ledger operations for [`MemoryNode`]s.
pub struct Ledger {
    rates: CostRates,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            rates: CostRates::default(),
        }
    }

    pub fn with_rates(rates: CostRates) -> Self {
        Self { rates }
    }

    /// Record a read access and append the cost entry to the node's ledger.
    pub fn record_read(&self, node: &mut MemoryNode, agent_id: AgentId) {
        let cost = self.rates.read;
        node.record_access(agent_id);
        // record_access already appends a read entry; override cost
        if let Some(last) = node.ledger.last_mut() {
            last.cost = cost;
        }
    }

    /// Record a write operation and append the cost entry.
    pub fn record_write(&self, node: &mut MemoryNode, agent_id: AgentId) {
        node.ledger.push(ComputeLedgerEntry::new(
            LedgerOperation::Write,
            self.rates.write,
            agent_id,
        ));
    }

    /// Record a decay operation.
    pub fn record_decay(&self, node: &mut MemoryNode) {
        node.ledger.push(ComputeLedgerEntry::new(
            LedgerOperation::Decay,
            self.rates.decay,
            ComputeLedgerEntry::SYSTEM_AGENT,
        ));
    }

    /// Record a prune operation.
    pub fn record_prune(&self, node: &mut MemoryNode) {
        node.ledger.push(ComputeLedgerEntry::new(
            LedgerOperation::Prune,
            self.rates.prune,
            ComputeLedgerEntry::SYSTEM_AGENT,
        ));
    }

    /// Compute total cost for a single node's ledger.
    pub fn node_total_cost(node: &MemoryNode) -> f64 {
        node.ledger.iter().map(|e| e.cost).sum()
    }

    /// Compute total cost across all nodes.
    pub fn total_cost(nodes: &[MemoryNode]) -> f64 {
        nodes.iter().map(Self::node_total_cost).sum()
    }

    /// Compute cost breakdown by operation type across all nodes.
    pub fn cost_breakdown(nodes: &[MemoryNode]) -> CostBreakdown {
        let mut breakdown = CostBreakdown::default();
        for node in nodes {
            for entry in &node.ledger {
                match entry.operation {
                    LedgerOperation::Read => breakdown.read += entry.cost,
                    LedgerOperation::Write => breakdown.write += entry.cost,
                    LedgerOperation::Decay => breakdown.decay += entry.cost,
                    LedgerOperation::Prune => breakdown.prune += entry.cost,
                }
            }
        }
        breakdown
    }

    /// Compute cost per agent across all nodes.
    pub fn cost_per_agent(nodes: &[MemoryNode]) -> std::collections::HashMap<AgentId, f64> {
        let mut costs = std::collections::HashMap::new();
        for node in nodes {
            for entry in &node.ledger {
                *costs.entry(entry.agent_id).or_insert(0.0) += entry.cost;
            }
        }
        costs
    }

    /// Compute cost for a specific time window (between two timestamps).
    pub fn cost_in_window(
        nodes: &[MemoryNode],
        start: std::time::SystemTime,
        end: std::time::SystemTime,
    ) -> f64 {
        nodes
            .iter()
            .flat_map(|n| &n.ledger)
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .map(|e| e.cost)
            .sum()
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CostBreakdown
// ---------------------------------------------------------------------------

/// Breakdown of costs by operation type.
#[derive(Debug, Clone, Default)]
pub struct CostBreakdown {
    pub read: f64,
    pub write: f64,
    pub decay: f64,
    pub prune: f64,
}

impl CostBreakdown {
    pub fn total(&self) -> f64 {
        self.read + self.write + self.decay + self.prune
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_node::{SensoryModality, VectorEmbedding};

    const AGENT: AgentId = [1u8; 32];

    fn make_node(id: u8) -> MemoryNode {
        MemoryNode::new(
            [id; 32],
            VectorEmbedding::new(vec![1.0, 0.0]),
            b"test".to_vec(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            AGENT,
        )
    }

    #[test]
    fn node_total_cost_includes_all_entries() {
        let node = make_node(1);
        let cost = Ledger::node_total_cost(&node);
        // Node starts with one write entry
        assert!(cost > 0.0);
    }

    #[test]
    fn total_cost_sums_across_nodes() {
        let nodes = vec![make_node(1), make_node(2)];
        let total = Ledger::total_cost(&nodes);
        assert!(total > 0.0);
    }

    #[test]
    fn cost_breakdown_by_operation() {
        let nodes = vec![make_node(1), make_node(2)];
        let breakdown = Ledger::cost_breakdown(&nodes);
        // All nodes start with write entries
        assert!(breakdown.write > 0.0);
        assert!(breakdown.read == 0.0); // no reads yet
    }

    #[test]
    fn record_read_appends_entry() {
        let ledger = Ledger::new();
        let mut node = make_node(1);
        ledger.record_read(&mut node, AGENT);

        let reads: Vec<_> = node
            .ledger
            .iter()
            .filter(|e| e.operation == LedgerOperation::Read)
            .collect();
        assert_eq!(reads.len(), 1);
    }

    #[test]
    fn record_write_appends_entry() {
        let ledger = Ledger::new();
        let mut node = make_node(1);
        ledger.record_write(&mut node, AGENT);

        let writes: Vec<_> = node
            .ledger
            .iter()
            .filter(|e| e.operation == LedgerOperation::Write)
            .collect();
        assert_eq!(writes.len(), 2); // initial write + recorded write
    }

    #[test]
    fn cost_per_agent_aggregates() {
        let mut nodes = vec![make_node(1)];
        let ledger = Ledger::new();
        ledger.record_read(&mut nodes[0], AGENT);

        let per_agent = Ledger::cost_per_agent(&nodes);
        assert!(per_agent.contains_key(&AGENT));
        assert!(per_agent[&AGENT] > 0.0);
    }

    #[test]
    fn custom_rates() {
        let rates = CostRates {
            read: 0.1,
            write: 0.2,
            decay: 0.3,
            prune: 0.4,
        };
        let ledger = Ledger::with_rates(rates);
        let mut node = make_node(1);
        ledger.record_read(&mut node, AGENT);

        let last = node.ledger.last().unwrap();
        assert!((last.cost - 0.1).abs() < 1e-10);
    }
}
