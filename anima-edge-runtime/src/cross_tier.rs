use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies which deployment tier a node belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    /// Tier 1: Type-1 bare-metal hypervisor (full Anima Core)
    CoreHypervisor,
    /// Tier 2: eBPF kernel module / OS co-processor
    CoProcessor,
    /// Tier 3: Wasm / native binary (phones, IoT, browsers)
    EdgeRuntime,
}

/// Capability flags advertised by a tier node during handshake.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TierCapabilities {
    pub supports_sse: bool,
    pub supports_inp: bool,
    pub supports_ck: bool,
    pub supports_zkpa: bool,
    pub max_memory_bytes: u64,
    pub max_concurrent_intents: u32,
}

/// Initial handshake message sent when a tier node connects to the mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub node_id: Uuid,
    pub tier: Tier,
    pub capabilities: TierCapabilities,
    pub protocol_version: u32,
    pub timestamp: u64,
}

/// Response to a handshake, acknowledging the connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeAck {
    pub node_id: Uuid,
    pub accepted: bool,
    pub reason: Option<String>,
    pub assigned_role: TierRole,
}

/// Role assigned to a node within the cross-tier mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TierRole {
    /// Full node — participates in all layers
    Full,
    /// Relay node — forwards messages between tiers
    Relay,
    /// Leaf node — only produces/consumes, no relay
    Leaf,
}

/// Envelope for all cross-tier messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTierEnvelope {
    pub envelope_id: Uuid,
    pub source_node: Uuid,
    pub target_node: Option<Uuid>,
    pub tier_route: TierRoute,
    pub payload: CrossTierPayload,
    pub timestamp: u64,
    pub ttl: u32,
}

/// Routing information for cross-tier messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierRoute {
    pub origin_tier: Tier,
    pub destination_tier: Tier,
    pub hop_count: u32,
    pub max_hops: u32,
}

/// Payload variants for cross-tier communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossTierPayload {
    /// Memory synchronization between tiers
    MemorySync(MemorySyncMessage),
    /// Intent forwarding across tiers
    IntentForward(IntentMessage),
    /// Heartbeat / liveness probe
    Heartbeat(HeartbeatMessage),
    /// Capability advertisement update
    CapabilityUpdate(CapabilityMessage),
    /// Error notification
    Error(ErrorMessage),
}

/// Memory synchronization message — propagate memory state across tiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncMessage {
    pub sync_id: Uuid,
    pub operation: SyncOperation,
    pub node_ids: Vec<Uuid>,
    pub data: Vec<u8>,
    pub causal_checksum: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOperation {
    Push,
    Pull,
    Delta,
    FullSync,
}

/// Intent message — forward an intent to another tier for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentMessage {
    pub intent_id: Uuid,
    pub action: String,
    pub payload: serde_json::Value,
    pub qos: IntentQoS,
    pub origin_tier: Tier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentQoS {
    pub max_latency_ms: u32,
    pub min_accuracy: f64,
    pub cost_budget: u64,
}

/// Heartbeat message for cross-tier liveness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    pub sequence: u64,
    pub load_factor: f64,
    pub memory_pressure: f64,
    pub active_intents: u32,
}

/// Capability update message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMessage {
    pub capabilities: TierCapabilities,
    pub changed: bool,
}

/// Error message for cross-tier fault notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: u32,
    pub message: String,
    pub recoverable: bool,
}

/// Errors that can occur in cross-tier communication.
#[derive(Debug, thiserror::Error)]
pub enum CrossTierError {
    #[error("handshake rejected: {0}")]
    HandshakeRejected(String),

    #[error("TTL expired after {0} hops")]
    TtlExpired(u32),

    #[error("tier route not found from {0:?} to {1:?}")]
    RouteNotFound(Tier, Tier),

    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    #[error("capability not supported: {0}")]
    UnsupportedCapability(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("node {0} not found")]
    NodeNotFound(Uuid),
}

/// Current protocol version.
pub const PROTOCOL_VERSION: u32 = 1;

/// Default max hops for cross-tier routing.
pub const DEFAULT_MAX_HOPS: u32 = 8;

/// Builder for constructing cross-tier envelopes.
pub struct EnvelopeBuilder {
    source_node: Uuid,
    target_node: Option<Uuid>,
    origin_tier: Tier,
    destination_tier: Tier,
    payload: CrossTierPayload,
    ttl: u32,
}

impl EnvelopeBuilder {
    pub fn new(
        source_node: Uuid,
        origin_tier: Tier,
        destination_tier: Tier,
        payload: CrossTierPayload,
    ) -> Self {
        Self {
            source_node,
            target_node: None,
            origin_tier,
            destination_tier,
            payload,
            ttl: DEFAULT_MAX_HOPS,
        }
    }

    pub fn target(mut self, target: Uuid) -> Self {
        self.target_node = Some(target);
        self
    }

    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn build(self) -> CrossTierEnvelope {
        CrossTierEnvelope {
            envelope_id: Uuid::new_v4(),
            source_node: self.source_node,
            target_node: self.target_node,
            tier_route: TierRoute {
                origin_tier: self.origin_tier,
                destination_tier: self.destination_tier,
                hop_count: 0,
                max_hops: self.ttl,
            },
            payload: self.payload,
            timestamp: current_timestamp_ms(),
            ttl: self.ttl,
        }
    }
}

/// Serialize an envelope to JSON bytes.
pub fn serialize_envelope(env: &CrossTierEnvelope) -> Result<Vec<u8>, CrossTierError> {
    serde_json::to_vec(env).map_err(|e| CrossTierError::Serialization(e.to_string()))
}

/// Deserialize an envelope from JSON bytes.
pub fn deserialize_envelope(bytes: &[u8]) -> Result<CrossTierEnvelope, CrossTierError> {
    serde_json::from_slice(bytes).map_err(|e| CrossTierError::Serialization(e.to_string()))
}

/// Validate an envelope for forwarding.
pub fn validate_envelope(env: &CrossTierEnvelope) -> Result<(), CrossTierError> {
    if env.tier_route.hop_count >= env.tier_route.max_hops {
        return Err(CrossTierError::TtlExpired(env.tier_route.hop_count));
    }
    Ok(())
}

/// Create a handshake message.
pub fn create_handshake(tier: Tier, capabilities: TierCapabilities) -> Handshake {
    Handshake {
        node_id: Uuid::new_v4(),
        tier,
        capabilities,
        protocol_version: PROTOCOL_VERSION,
        timestamp: current_timestamp_ms(),
    }
}

/// Validate a handshake against our protocol version.
pub fn validate_handshake(handshake: &Handshake) -> Result<(), CrossTierError> {
    if handshake.protocol_version != PROTOCOL_VERSION {
        return Err(CrossTierError::VersionMismatch {
            expected: PROTOCOL_VERSION,
            actual: handshake.protocol_version,
        });
    }
    Ok(())
}

/// Determine the role for a node based on its tier and capabilities.
pub fn assign_role(tier: Tier, caps: &TierCapabilities) -> TierRole {
    match tier {
        Tier::CoreHypervisor if caps.supports_sse && caps.supports_ck => TierRole::Full,
        Tier::CoProcessor if caps.max_concurrent_intents >= 10 => TierRole::Relay,
        _ => TierRole::Leaf,
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_builder_creates_valid_envelope() {
        let node = Uuid::new_v4();
        let payload = CrossTierPayload::Heartbeat(HeartbeatMessage {
            sequence: 1,
            load_factor: 0.5,
            memory_pressure: 0.3,
            active_intents: 2,
        });
        let env = EnvelopeBuilder::new(node, Tier::EdgeRuntime, Tier::CoreHypervisor, payload)
            .ttl(5)
            .build();

        assert_eq!(env.source_node, node);
        assert_eq!(env.tier_route.origin_tier, Tier::EdgeRuntime);
        assert_eq!(env.tier_route.destination_tier, Tier::CoreHypervisor);
        assert_eq!(env.ttl, 5);
        assert_eq!(env.tier_route.hop_count, 0);
    }

    #[test]
    fn serialize_roundtrip() {
        let node = Uuid::new_v4();
        let payload = CrossTierPayload::Heartbeat(HeartbeatMessage {
            sequence: 42,
            load_factor: 0.8,
            memory_pressure: 0.1,
            active_intents: 5,
        });
        let env = EnvelopeBuilder::new(node, Tier::CoProcessor, Tier::EdgeRuntime, payload).build();

        let bytes = serialize_envelope(&env).unwrap();
        let recovered = deserialize_envelope(&bytes).unwrap();

        assert_eq!(recovered.envelope_id, env.envelope_id);
        assert_eq!(recovered.source_node, node);
        assert_eq!(recovered.ttl, DEFAULT_MAX_HOPS);
    }

    #[test]
    fn validate_envelope_allows_fresh() {
        let payload = CrossTierPayload::Heartbeat(HeartbeatMessage {
            sequence: 0,
            load_factor: 0.0,
            memory_pressure: 0.0,
            active_intents: 0,
        });
        let env = EnvelopeBuilder::new(Uuid::new_v4(), Tier::EdgeRuntime, Tier::CoreHypervisor, payload)
            .build();
        assert!(validate_envelope(&env).is_ok());
    }

    #[test]
    fn validate_envelope_rejects_expired() {
        let payload = CrossTierPayload::Heartbeat(HeartbeatMessage {
            sequence: 0,
            load_factor: 0.0,
            memory_pressure: 0.0,
            active_intents: 0,
        });
        let mut env = EnvelopeBuilder::new(Uuid::new_v4(), Tier::EdgeRuntime, Tier::CoreHypervisor, payload)
            .build();
        env.tier_route.hop_count = 10;
        env.tier_route.max_hops = 8;
        assert!(validate_envelope(&env).is_err());
    }

    #[test]
    fn handshake_validation_accepts_current_version() {
        let hs = create_handshake(Tier::EdgeRuntime, TierCapabilities::default());
        assert!(validate_handshake(&hs).is_ok());
    }

    #[test]
    fn handshake_validation_rejects_old_version() {
        let mut hs = create_handshake(Tier::EdgeRuntime, TierCapabilities::default());
        hs.protocol_version = 0;
        assert!(validate_handshake(&hs).is_err());
    }

    #[test]
    fn assign_role_core_with_full_caps_is_full() {
        let caps = TierCapabilities {
            supports_sse: true,
            supports_inp: true,
            supports_ck: true,
            supports_zkpa: true,
            max_memory_bytes: 1024 * 1024 * 1024,
            max_concurrent_intents: 100,
        };
        assert_eq!(assign_role(Tier::CoreHypervisor, &caps), TierRole::Full);
    }

    #[test]
    fn assign_role_edge_is_leaf() {
        let caps = TierCapabilities {
            supports_sse: true,
            ..Default::default()
        };
        assert_eq!(assign_role(Tier::EdgeRuntime, &caps), TierRole::Leaf);
    }

    #[test]
    fn _co_processor_with_capacity_is_relay() {
        let caps = TierCapabilities {
            max_concurrent_intents: 20,
            ..Default::default()
        };
        assert_eq!(assign_role(Tier::CoProcessor, &caps), TierRole::Relay);
    }

    #[test]
    fn memory_sync_message_serializes() {
        let msg = MemorySyncMessage {
            sync_id: Uuid::new_v4(),
            operation: SyncOperation::Delta,
            node_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
            data: vec![1, 2, 3, 4],
            causal_checksum: [0u8; 32],
        };
        let payload = CrossTierPayload::MemorySync(msg);
        let env = EnvelopeBuilder::new(Uuid::new_v4(), Tier::CoreHypervisor, Tier::EdgeRuntime, payload).build();
        let bytes = serialize_envelope(&env).unwrap();
        let recovered = deserialize_envelope(&bytes).unwrap();
        match recovered.payload {
            CrossTierPayload::MemorySync(sync) => {
                assert_eq!(sync.data, vec![1, 2, 3, 4]);
                assert_eq!(sync.node_ids.len(), 2);
            }
            _ => panic!("wrong payload type"),
        }
    }

    #[test]
    fn intent_forward_message_serializes() {
        let msg = IntentMessage {
            intent_id: Uuid::new_v4(),
            action: "compute".into(),
            payload: serde_json::json!({"input": "test"}),
            qos: IntentQoS {
                max_latency_ms: 100,
                min_accuracy: 0.95,
                cost_budget: 50,
            },
            origin_tier: Tier::EdgeRuntime,
        };
        let payload = CrossTierPayload::IntentForward(msg);
        let env = EnvelopeBuilder::new(Uuid::new_v4(), Tier::EdgeRuntime, Tier::CoProcessor, payload).build();
        let bytes = serialize_envelope(&env).unwrap();
        let recovered = deserialize_envelope(&bytes).unwrap();
        match recovered.payload {
            CrossTierPayload::IntentForward(intent) => {
                assert_eq!(intent.action, "compute");
                assert_eq!(intent.qos.max_latency_ms, 100);
            }
            _ => panic!("wrong payload type"),
        }
    }
}