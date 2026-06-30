//! Semantic State Engine — the memory substrate of TPT Anima.
//!
//! Stores data as unified semantic tensors with native temporal dynamics,
//! causal topology, and multi-modal sensory ingestion.

pub mod causal_graph;
pub mod decay;
pub mod ledger;
pub mod mcp;
pub mod memory_node;
pub mod sensory;
pub mod storage;

pub use memory_node::{
    AgentId, CausalLink, CausalLinkType, ComputeLedgerEntry, DecayProfile, LedgerOperation,
    MemoryId, MemoryMetadata, MemoryNode, SensoryModality, SseError, VectorEmbedding,
};

pub use causal_graph::{CausalError, CausalGraph};
pub use decay::{CollectionStats, MemoryType};
pub use ledger::{CostBreakdown, CostRates, Ledger};
pub use mcp::{
    McpAttendTool, McpCausalChainResource, McpContextChainTool, McpMemoryResource,
    McpSubstrateResource,
};
pub use sensory::{SensoryError, SensoryIngestionInterface, SensoryManager};
pub use storage::InMemoryStorage;
