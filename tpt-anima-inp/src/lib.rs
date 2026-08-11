//! Intent & Negotiation Protocol — the nervous system of TPT Anima.
//!
//! Agents broadcast goal descriptors; capability providers bid; a state-machine
//! routes intents to the optimal provider with automatic fallback chains.
//!
//! ```
//! use std::time::Duration;
//! use uuid::Uuid;
//! use tpt_anima_inp::{
//!     Capability, CapabilityBid, CapabilityRegistry, EconomicTerms, Intent,
//!     NegotiationSession, PerformanceProfile, QoSConstraints,
//! };
//!
//! let requester = Uuid::new_v4();
//! let provider = Uuid::new_v4();
//!
//! let mut registry = CapabilityRegistry::new();
//! let capability_id = registry.register(Capability::new(
//!     vec!["analyze".into()],
//!     PerformanceProfile {
//!         typical_latency: Duration::from_millis(200),
//!         max_latency: Duration::from_millis(600),
//!         typical_accuracy: 0.92,
//!         throughput_per_second: 8.0,
//!     },
//!     EconomicTerms::FixedPrice { credits: 100.0 },
//!     provider,
//! ));
//!
//! let intent = Intent::new(
//!     "analyze".into(),
//!     serde_json::json!({ "task": "Analyze Q3 report" }),
//!     QoSConstraints::default(),
//!     EconomicTerms::FixedPrice { credits: 100.0 },
//!     requester,
//! );
//! let intent_id = intent.id;
//!
//! let mut session = NegotiationSession::new(intent);
//! session.start_broadcasting();
//! session.start_collecting();
//! session.add_bid(CapabilityBid::new(
//!     intent_id, provider, capability_id, 100.0, Duration::from_millis(200), 0.92, 0.9,
//! ));
//! assert_eq!(session.bids.len(), 1);
//! ```
//!
//! See `examples/negotiation_swarm.rs` for the full walkthrough including
//! bid selection and a real `ComputeLedger` credit transfer.

pub mod broadcast;
pub mod consensus;
pub mod contracts;
pub mod intent;
pub mod ledger;
pub mod manifest;
pub mod mcp;
pub mod negotiation;
pub mod qos;
pub mod registry;

pub use intent::{
    AgentId, Capability, CapabilityBid, CapabilityId, EconomicTerms, ExecutionResult,
    ExecutionStatus, FallbackStrategy, FallbackTrigger, Intent, IntentId, PerformanceProfile,
    Priority, QoSConstraints,
};

pub use broadcast::{BroadcastConfig, BroadcastError, BroadcastResult, IntentBroadcaster};
pub use consensus::{
    ConsensusManager, ConsensusStats, EconomicStake, ProofOfWork, RateLimiter, ReputationManager,
};
pub use contracts::{ContractError, ContractValidator, SemanticContract};
pub use ledger::{ComputeCredits, ComputeLedger, ExternalRails, Transaction};
pub use manifest::{
    CapabilityManifest, ManifestCapability, ManifestError, ManifestParser, ManifestPerformance,
    ManifestPricing,
};
pub use mcp::{
    McpBidQueryTool, McpBroadcastTool, McpIntentResource, McpNegotiationResource,
    McpResultRetrievalTool,
};
pub use negotiation::{NegotiationManager, NegotiationSession, NegotiationState};
pub use qos::{QoSAdjustment, QoSNegotiator, QoSProfile, QoSRecommender};
pub use registry::CapabilityRegistry;
