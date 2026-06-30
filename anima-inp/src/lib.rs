//! Intent & Negotiation Protocol — the nervous system of TPT Anima.
//!
//! Agents broadcast goal descriptors; capability providers bid; a state-machine
//! routes intents to the optimal provider with automatic fallback chains.

pub mod broadcast;
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
pub use contracts::{ContractError, ContractValidator, SemanticContract};
pub use ledger::{ComputeCredits, ComputeLedger, ExternalRails, Transaction};
pub use mcp::{
    McpBidQueryTool, McpBroadcastTool, McpIntentResource, McpNegotiationResource,
    McpResultRetrievalTool,
};
pub use manifest::{
    CapabilityManifest, ManifestCapability, ManifestError, ManifestParser, ManifestPerformance,
    ManifestPricing,
};
pub use negotiation::{NegotiationManager, NegotiationSession, NegotiationState};
pub use qos::{QoSAdjustment, QoSNegotiator, QoSProfile, QoSRecommender};
pub use registry::CapabilityRegistry;
