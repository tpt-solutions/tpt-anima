//! Semantic State Engine — the memory substrate of TPT Anima.
//!
//! Stores data as unified semantic tensors with native temporal dynamics,
//! causal topology, and multi-modal sensory ingestion.
//!
//! ```
//! use std::time::Duration;
//! use tpt_anima_sse::{InMemoryStorage, MemoryNode, SensoryModality, VectorEmbedding};
//!
//! let mut storage = InMemoryStorage::new();
//! let mut node = MemoryNode::new(
//!     [1u8; 32],
//!     VectorEmbedding::new(vec![0.12, 0.44, -0.08, 0.91]),
//!     b"The capital of France is Paris.".to_vec(),
//!     "text/plain".into(),
//!     SensoryModality::Textual,
//!     [9u8; 32],
//! );
//!
//! // Memory is attended, not queried: strength decays exponentially over time...
//! node.apply_decay_for(Duration::from_secs(24 * 3600));
//! // ...and is reinforced when accessed again.
//! node.record_access([9u8; 32]);
//!
//! storage.write_node(node);
//! assert_eq!(storage.node_count(), 1);
//! ```
//!
//! See `examples/memory_lifecycle.rs` for the full walkthrough with decay output.

pub mod ann;
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
pub use ann::AnnIndex;
