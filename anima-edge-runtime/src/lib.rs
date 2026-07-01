//! Tier 3 Edge Runtime — lightweight SSE + INP for phones, IoT, and browsers.
//!
//! Provides a minimal memory store ([`memory::EdgeMemory`]) and intent
//! handler ([`intent::EdgeIntentHandler`]) coordinated by a runtime
//! ([`runtime::EdgeRuntime`]) suitable for resource-constrained devices.
//!
//! Also provides the cross-tier communication protocol ([`cross_tier`])
//! for inter-tier message routing between Core Hypervisor, Co-Processor,
//! and Edge Runtime nodes.

pub mod cross_tier;
pub mod intent;
pub mod memory;
pub mod runtime;

pub use cross_tier::{
    assign_role, create_handshake, deserialize_envelope, serialize_envelope,
    validate_envelope, validate_handshake, CapabilityMessage, CrossTierEnvelope,
    CrossTierError, CrossTierPayload, EnvelopeBuilder, ErrorMessage, Handshake,
    HandshakeAck, HeartbeatMessage, IntentMessage, IntentQoS, MemorySyncMessage,
    SyncOperation, Tier, TierCapabilities, TierRole, TierRoute,
    DEFAULT_MAX_HOPS, PROTOCOL_VERSION,
};
pub use intent::{EdgeError, EdgeIntent, EdgeIntentHandler};
pub use memory::{EdgeMemory, MemoryError};
pub use runtime::{EdgeRuntime, RuntimeStats};
