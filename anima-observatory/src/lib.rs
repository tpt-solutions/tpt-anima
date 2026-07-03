//! Anima Observatory — telemetry aggregation library.
//!
//! Collects and aggregates real-time telemetry from all four Anima layers:
//! SSE (Stable State Engine), INP (Intent Negotiation Protocol),
//! CK (Cognitive Kernel), and ZKPA (Zero-Knowledge Proof Attestation).

pub mod routes;
pub mod server;

pub use routes::handle_request;
pub use server::{BroadcastMessage, TelemetryServer};

use serde::{Deserialize, Serialize};

// ── Layer Events ──────────────────────────────────────────────────────────────

/// Events from the Stable State Engine layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SseEvent {
    MemoryAccessed { node_id: String, strength: f32 },
    MemoryPruned { count: usize },
    DecayApplied { node_id: String, new_strength: f32 },
}

/// Events from the Intent Negotiation Protocol layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InpEvent {
    IntentBroadcast {
        intent_id: String,
        action: String,
    },
    BidReceived {
        intent_id: String,
        provider_id: String,
    },
    NegotiationCompleted {
        intent_id: String,
        winner: String,
    },
    /// A real compute-credit transfer recorded by INP's `ComputeLedger`.
    CreditTransferred {
        from: String,
        to: String,
        amount: f64,
    },
}

/// Events from the Cognitive Kernel layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CkEvent {
    InferenceScheduled {
        inference_id: String,
        priority: String,
    },
    CircuitBreakerTripped {
        reason: String,
    },
    ThoughtBubbleCreated {
        bubble_id: String,
    },
    /// Real hardware utilization sample, sourced from the CK JIT hardware
    /// detector (see `anima-ck/src/jit/detection.rs`) rather than a random
    /// generator.
    HardwareUtilization {
        gpu: f32,
        cpu: f32,
        npu: f32,
        ram: f32,
    },
}

/// Events from the Zero-Knowledge Proof Attestation layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZkpaEvent {
    SessionCreated {
        session_id: String,
        agent_id: String,
    },
    AttestationVerified {
        platform: String,
    },
    LivenessHeartbeat {
        agent_id: String,
        sequence: u64,
    },
}

/// A unified telemetry event wrapping one of the four layer variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TelemetryEvent {
    SSE(SseEvent),
    INP(InpEvent),
    CK(CkEvent),
    ZKPA(ZkpaEvent),
}

// ── TelemetryCollector ────────────────────────────────────────────────────────

/// Collects telemetry events and provides aggregation methods.
#[derive(Debug, Clone, Default)]
pub struct TelemetryCollector {
    events: Vec<TelemetryEvent>,
}

impl TelemetryCollector {
    /// Create an empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a single telemetry event.
    pub fn record(&mut self, event: TelemetryEvent) {
        self.events.push(event);
    }

    /// Return a reference to all collected events.
    pub fn events(&self) -> &[TelemetryEvent] {
        &self.events
    }

    /// Count events grouped by layer.
    pub fn count_by_layer(&self) -> LayerCounts {
        let mut counts = LayerCounts::default();
        for ev in &self.events {
            match ev {
                TelemetryEvent::SSE(_) => counts.sse += 1,
                TelemetryEvent::INP(_) => counts.inp += 1,
                TelemetryEvent::CK(_) => counts.ck += 1,
                TelemetryEvent::ZKPA(_) => counts.zkpa += 1,
            }
        }
        counts
    }

    /// Return the most recent `n` events (in insertion order).
    pub fn recent_events(&self, n: usize) -> Vec<&TelemetryEvent> {
        let start = self.events.len().saturating_sub(n);
        self.events[start..].iter().collect()
    }

    /// Remove all collected events.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Event counts per layer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerCounts {
    pub sse: usize,
    pub inp: usize,
    pub ck: usize,
    pub zkpa: usize,
}

// ── Swarm Map ─────────────────────────────────────────────────────────────────

/// A node in the swarm topology visualization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmNode {
    pub id: String,
    pub label: String,
    pub status: String,
    pub agent_type: String,
}

/// An edge between two swarm nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmEdge {
    pub source: String,
    pub target: String,
    pub weight: f32,
}

/// A pulse event representing an intent flowing between nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PulseEvent {
    pub source: String,
    pub target: String,
    pub intent_id: String,
    pub timestamp: u64,
    pub won: bool,
}

/// Complete swarm map data for visualization.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SwarmMapData {
    pub nodes: Vec<SwarmNode>,
    pub edges: Vec<SwarmEdge>,
    pub pulse_events: Vec<PulseEvent>,
}

impl SwarmMapData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the swarm map.
    pub fn add_node(&mut self, node: SwarmNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the swarm map.
    pub fn add_edge(&mut self, edge: SwarmEdge) {
        self.edges.push(edge);
    }

    /// Record a pulse event.
    pub fn record_pulse(&mut self, pulse: PulseEvent) {
        self.pulse_events.push(pulse);
    }

    /// Find a node by ID.
    pub fn find_node(&self, id: &str) -> Option<&SwarmNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all edges for a given node (as source).
    pub fn edges_from(&self, node_id: &str) -> Vec<&SwarmEdge> {
        self.edges.iter().filter(|e| e.source == node_id).collect()
    }

    /// Get all edges for a given node (as target).
    pub fn edges_to(&self, node_id: &str) -> Vec<&SwarmEdge> {
        self.edges.iter().filter(|e| e.target == node_id).collect()
    }
}

// ── Memory Topography ─────────────────────────────────────────────────────────

/// A node in the memory topography visualization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopoNode {
    pub id: String,
    pub strength: f32,
    pub modality: String,
    pub label: String,
}

/// An edge representing associative strength between memory nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopoChain {
    pub source: String,
    pub target: String,
    pub strength: f32,
}

/// Complete memory topography data for visualization.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MemoryTopographyData {
    pub nodes: Vec<TopoNode>,
    pub chains: Vec<TopoChain>,
}

impl MemoryTopographyData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the topography.
    pub fn add_node(&mut self, node: TopoNode) {
        self.nodes.push(node);
    }

    /// Add a chain (edge) between memory nodes.
    pub fn add_chain(&mut self, chain: TopoChain) {
        self.chains.push(chain);
    }

    /// Find a node by ID.
    pub fn find_node(&self, id: &str) -> Option<&TopoNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all chains involving a given node.
    pub fn chains_for(&self, node_id: &str) -> Vec<&TopoChain> {
        self.chains
            .iter()
            .filter(|c| c.source == node_id || c.target == node_id)
            .collect()
    }

    /// Return the total number of nodes and chains.
    pub fn summary(&self) -> (usize, usize) {
        (self.nodes.len(), self.chains.len())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event_sse() -> TelemetryEvent {
        TelemetryEvent::SSE(SseEvent::MemoryAccessed {
            node_id: "n1".into(),
            strength: 0.85,
        })
    }

    fn sample_event_inp() -> TelemetryEvent {
        TelemetryEvent::INP(InpEvent::IntentBroadcast {
            intent_id: "i1".into(),
            action: "route".into(),
        })
    }

    fn sample_event_ck() -> TelemetryEvent {
        TelemetryEvent::CK(CkEvent::InferenceScheduled {
            inference_id: "inf1".into(),
            priority: "high".into(),
        })
    }

    fn sample_event_zkpa() -> TelemetryEvent {
        TelemetryEvent::ZKPA(ZkpaEvent::SessionCreated {
            session_id: "s1".into(),
            agent_id: "a1".into(),
        })
    }

    #[test]
    fn collector_records_and_counts_events() {
        let mut collector = TelemetryCollector::new();
        collector.record(sample_event_sse());
        collector.record(sample_event_inp());
        collector.record(sample_event_ck());
        collector.record(sample_event_zkpa());

        assert_eq!(collector.events().len(), 4);

        let counts = collector.count_by_layer();
        assert_eq!(counts.sse, 1);
        assert_eq!(counts.inp, 1);
        assert_eq!(counts.ck, 1);
        assert_eq!(counts.zkpa, 1);
    }

    #[test]
    fn collector_recent_events_returns_tail() {
        let mut collector = TelemetryCollector::new();
        collector.record(sample_event_sse());
        collector.record(sample_event_inp());
        collector.record(sample_event_ck());
        collector.record(sample_event_zkpa());

        let recent = collector.recent_events(2);
        assert_eq!(recent.len(), 2);
        assert!(matches!(recent[0], TelemetryEvent::CK(_)));
        assert!(matches!(recent[1], TelemetryEvent::ZKPA(_)));
    }

    #[test]
    fn collector_clears_events() {
        let mut collector = TelemetryCollector::new();
        collector.record(sample_event_sse());
        assert_eq!(collector.events().len(), 1);

        collector.clear();
        assert_eq!(collector.events().len(), 0);
        assert_eq!(collector.count_by_layer(), LayerCounts::default());
    }

    #[test]
    fn swarm_map_node_lookup() {
        let mut map = SwarmMapData::new();
        map.add_node(SwarmNode {
            id: "n1".into(),
            label: "Alpha".into(),
            status: "active".into(),
            agent_type: "cognitive".into(),
        });
        map.add_node(SwarmNode {
            id: "n2".into(),
            label: "Beta".into(),
            status: "idle".into(),
            agent_type: "relay".into(),
        });

        let found = map.find_node("n1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Alpha");

        assert!(map.find_node("n3").is_none());
    }

    #[test]
    fn swarm_map_edge_queries() {
        let mut map = SwarmMapData::new();
        map.add_edge(SwarmEdge {
            source: "n1".into(),
            target: "n2".into(),
            weight: 0.9,
        });
        map.add_edge(SwarmEdge {
            source: "n1".into(),
            target: "n3".into(),
            weight: 0.5,
        });
        map.add_edge(SwarmEdge {
            source: "n2".into(),
            target: "n3".into(),
            weight: 0.7,
        });

        let from_n1 = map.edges_from("n1");
        assert_eq!(from_n1.len(), 2);

        let to_n3 = map.edges_to("n3");
        assert_eq!(to_n3.len(), 2);

        let from_n3 = map.edges_from("n3");
        assert_eq!(from_n3.len(), 0);
    }

    #[test]
    fn swarm_map_pulse_events() {
        let mut map = SwarmMapData::new();
        map.record_pulse(PulseEvent {
            source: "n1".into(),
            target: "n2".into(),
            intent_id: "i1".into(),
            timestamp: 1000,
            won: true,
        });
        map.record_pulse(PulseEvent {
            source: "n2".into(),
            target: "n3".into(),
            intent_id: "i2".into(),
            timestamp: 2000,
            won: false,
        });

        assert_eq!(map.pulse_events.len(), 2);
        assert!(map.pulse_events[0].won);
        assert!(!map.pulse_events[1].won);
    }

    #[test]
    fn memory_topography_add_and_find() {
        let mut topo = MemoryTopographyData::new();
        topo.add_node(TopoNode {
            id: "m1".into(),
            strength: 0.9,
            modality: "text".into(),
            label: "Fact A".into(),
        });
        topo.add_chain(TopoChain {
            source: "m1".into(),
            target: "m2".into(),
            strength: 0.7,
        });

        assert!(topo.find_node("m1").is_some());
        assert!(topo.find_node("m99").is_none());

        let chains = topo.chains_for("m1");
        assert_eq!(chains.len(), 1);

        let (n_nodes, n_chains) = topo.summary();
        assert_eq!(n_nodes, 1);
        assert_eq!(n_chains, 1);
    }

    #[test]
    fn events_are_serializable() {
        let event = sample_event_sse();
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: TelemetryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }
}
