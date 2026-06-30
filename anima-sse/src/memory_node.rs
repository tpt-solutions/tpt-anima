//! Core data structures for the Semantic State Engine.
//!
//! [`MemoryNode`] is the fundamental storage unit: a vector embedding paired
//! with a raw payload, typed causal links to related nodes, temporal-decay
//! state, and a per-node compute micro-ledger.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Serde helper — SystemTime ↔ nanoseconds since UNIX epoch (u64)
// ---------------------------------------------------------------------------

mod serde_system_time {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let nanos = t
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos() as u64;
        nanos.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let nanos = u64::deserialize(d)?;
        Ok(UNIX_EPOCH + Duration::from_nanos(nanos))
    }
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// 256-bit content-addressed identifier for a memory node.
pub type MemoryId = [u8; 32];

/// 256-bit identifier for an agent operating within TPT Anima.
pub type AgentId = [u8; 32];

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can arise when working with SSE data structures.
#[derive(Debug, Error)]
pub enum SseError {
    #[error("vector dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: u32, got: u32 },

    #[error("vector has zero magnitude and cannot be normalised")]
    ZeroMagnitude,
}

// ---------------------------------------------------------------------------
// VectorEmbedding
// ---------------------------------------------------------------------------

/// Dense vector representation of a memory's semantic content.
///
/// When both vectors are [`normalised`](VectorEmbedding::normalize)
/// [`cosine_similarity`](VectorEmbedding::cosine_similarity) uses the fast
/// dot-product shortcut; otherwise it computes the full formula.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub values: Vec<f32>,
    pub dimensions: u32,
    pub normalized: bool,
}

impl VectorEmbedding {
    pub fn new(values: Vec<f32>) -> Self {
        let dimensions = values.len() as u32;
        Self {
            values,
            dimensions,
            normalized: false,
        }
    }

    /// Normalise this vector to unit length in-place.
    pub fn normalize(&mut self) -> Result<(), SseError> {
        let magnitude: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude == 0.0 {
            return Err(SseError::ZeroMagnitude);
        }
        self.values.iter_mut().for_each(|x| *x /= magnitude);
        self.normalized = true;
        Ok(())
    }

    /// Cosine similarity ∈ [−1, 1] between `self` and `other`.
    ///
    /// Returns [`SseError::DimensionMismatch`] when the vectors have different
    /// lengths, and [`SseError::ZeroMagnitude`] when either is the zero vector.
    pub fn cosine_similarity(&self, other: &VectorEmbedding) -> Result<f32, SseError> {
        if self.dimensions != other.dimensions {
            return Err(SseError::DimensionMismatch {
                expected: self.dimensions,
                got: other.dimensions,
            });
        }
        let dot: f32 = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| a * b)
            .sum();
        if self.normalized && other.normalized {
            return Ok(dot.clamp(-1.0, 1.0));
        }
        let mag_a: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = other.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            return Err(SseError::ZeroMagnitude);
        }
        Ok((dot / (mag_a * mag_b)).clamp(-1.0, 1.0))
    }
}

// ---------------------------------------------------------------------------
// SensoryModality
// ---------------------------------------------------------------------------

/// Describes the sensory origin of a memory node's payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SensoryModality {
    /// Text: documents, conversations, source code.
    Textual,
    /// Visual: image frames, video, spatial point-clouds.
    Visual,
    /// Audio: speech, music, environmental sound.
    Audio,
    /// Combination of two or more modalities (e.g. a captioned image).
    MultiModal(Vec<SensoryModality>),
    /// Abstract / derived data with no direct sensory origin.
    Abstract,
}

// ---------------------------------------------------------------------------
// CausalLinkType / CausalLink
// ---------------------------------------------------------------------------

/// The semantic nature of the directed relationship between two memory nodes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CausalLinkType {
    /// Node A directly caused node B to exist.
    Caused,
    /// Node A occurred immediately before B (temporal, not necessarily causal).
    Preceded,
    /// Node A is a component or sub-part of B.
    PartOf,
    /// Node A is semantically similar to B (high cosine similarity).
    SimilarTo,
    /// Node A contains information that contradicts B.
    Contradicts,
    /// Node A is a summary or abstraction of B.
    Abstracts,
}

/// A directed causal edge from a memory node to another node in the SSE graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalLink {
    /// Target memory node.
    pub target_id: MemoryId,
    pub link_type: CausalLinkType,
    /// Relationship strength ∈ [0, 1]; clamped on construction.
    pub strength: f32,
    #[serde(with = "serde_system_time")]
    pub created_at: SystemTime,
    /// Number of times the causal-chain traversal has crossed this edge.
    pub traversal_count: u64,
}

impl CausalLink {
    pub fn new(target_id: MemoryId, link_type: CausalLinkType, strength: f32) -> Self {
        Self {
            target_id,
            link_type,
            strength: strength.clamp(0.0, 1.0),
            created_at: SystemTime::now(),
            traversal_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// DecayProfile
// ---------------------------------------------------------------------------

/// Temporal-decay configuration for a single memory node.
///
/// Each call to [`apply_decay_for`](MemoryNode::apply_decay_for) applies:
///
/// ```text
/// strength = strength × exp(−decay_rate × Δhours)
/// ```
///
/// Each call to [`record_access`](MemoryNode::record_access) applies:
///
/// ```text
/// strength = min(strength × reinforcement_factor, max_strength)
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecayProfile {
    /// λ in the exponential formula — typical range `0.001..0.1` per hour.
    pub decay_rate: f32,
    /// Multiplicative strength boost applied on every read access.
    pub reinforcement_factor: f32,
    /// Upper bound on `current_strength`; prevents runaway reinforcement.
    pub max_strength: f32,
    /// Nodes whose `current_strength` drops below this are eligible for pruning.
    pub pruning_threshold: f32,
    /// Timestamp of the most recent decay application for this node.
    #[serde(with = "serde_system_time")]
    pub last_decay_at: SystemTime,
    /// When `true`, this node is exempt from both decay and pruning.
    pub pinned: bool,
}

impl Default for DecayProfile {
    fn default() -> Self {
        Self {
            decay_rate: 0.01,          // ~1 % per hour
            reinforcement_factor: 1.5, // 50 % boost per access
            max_strength: 1.0,
            pruning_threshold: 0.05, // prune below 5 %
            last_decay_at: SystemTime::now(),
            pinned: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ComputeLedgerEntry
// ---------------------------------------------------------------------------

/// The type of operation that generated a ledger cost record.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LedgerOperation {
    Read,
    Write,
    Decay,
    Prune,
}

/// A single micro-cost record in a memory node's compute ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComputeLedgerEntry {
    pub operation: LedgerOperation,
    /// Cost in Compute Credits charged for this operation.
    pub cost: f64,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
    /// Agent that triggered the operation; all-zeros = system.
    pub agent_id: AgentId,
}

impl ComputeLedgerEntry {
    pub fn new(operation: LedgerOperation, cost: f64, agent_id: AgentId) -> Self {
        Self {
            operation,
            cost,
            timestamp: SystemTime::now(),
            agent_id,
        }
    }

    /// Sentinel agent ID used for system-initiated operations (decay, pruning).
    pub const SYSTEM_AGENT: AgentId = [0u8; 32];
}

// ---------------------------------------------------------------------------
// MemoryMetadata
// ---------------------------------------------------------------------------

/// Creation, access, and modification audit trail for a memory node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryMetadata {
    #[serde(with = "serde_system_time")]
    pub created_at: SystemTime,
    #[serde(with = "serde_system_time")]
    pub last_accessed_at: SystemTime,
    #[serde(with = "serde_system_time")]
    pub last_modified_at: SystemTime,
    pub access_count: u64,
    pub creator_agent_id: AgentId,
    pub label: Option<String>,
    pub tags: Vec<String>,
    /// Arbitrary key-value pairs for extensible metadata.
    pub custom: HashMap<String, String>,
}

impl MemoryMetadata {
    pub fn new(creator_agent_id: AgentId) -> Self {
        let now = SystemTime::now();
        Self {
            created_at: now,
            last_accessed_at: now,
            last_modified_at: now,
            access_count: 0,
            creator_agent_id,
            label: None,
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryNode
// ---------------------------------------------------------------------------

/// The fundamental storage unit of the Semantic State Engine.
///
/// Combines a semantic vector, a raw payload, causal links, temporal-decay
/// state, and a per-node compute micro-ledger into a single, self-contained
/// memory record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: MemoryId,
    pub embedding: VectorEmbedding,
    /// Raw payload bytes — text, image data, audio samples, etc.
    pub payload: Vec<u8>,
    /// MIME type of the payload (e.g. `"text/plain"`, `"image/png"`).
    pub payload_format: String,
    pub sensory_modality: SensoryModality,
    /// Outgoing causal edges to related memory nodes.
    pub causal_links: Vec<CausalLink>,
    /// Current strength ∈ [0, 1]: decays over time, reinforced by access.
    pub current_strength: f32,
    pub decay_profile: DecayProfile,
    /// Append-only list of micro-cost records for this node.
    pub ledger: Vec<ComputeLedgerEntry>,
    pub metadata: MemoryMetadata,
    /// Monotonically increasing write counter for optimistic concurrency.
    pub version: u64,
    /// SHA-256 of `payload` — verified by [`verify_payload_integrity`](Self::verify_payload_integrity).
    pub payload_hash: [u8; 32],
}

impl MemoryNode {
    /// Create a new memory node at full strength with default decay settings.
    pub fn new(
        id: MemoryId,
        embedding: VectorEmbedding,
        payload: Vec<u8>,
        payload_format: String,
        sensory_modality: SensoryModality,
        creator_agent_id: AgentId,
    ) -> Self {
        let payload_hash = Self::hash_payload(&payload);
        let write_entry = ComputeLedgerEntry::new(LedgerOperation::Write, 0.01, creator_agent_id);
        Self {
            id,
            embedding,
            payload,
            payload_format,
            sensory_modality,
            causal_links: Vec::new(),
            current_strength: 1.0,
            decay_profile: DecayProfile::default(),
            ledger: vec![write_entry],
            metadata: MemoryMetadata::new(creator_agent_id),
            version: 1,
            payload_hash,
        }
    }

    /// Record a read access: reinforce strength and append a ledger entry.
    pub fn record_access(&mut self, agent_id: AgentId) {
        self.metadata.access_count += 1;
        self.metadata.last_accessed_at = SystemTime::now();
        self.current_strength = (self.current_strength * self.decay_profile.reinforcement_factor)
            .min(self.decay_profile.max_strength);
        self.ledger.push(ComputeLedgerEntry::new(
            LedgerOperation::Read,
            0.001,
            agent_id,
        ));
    }

    /// Apply exponential temporal decay based on real elapsed time.
    ///
    /// Computes `elapsed = now − last_decay_at` then delegates to
    /// [`apply_decay_for`](Self::apply_decay_for). No-op for pinned nodes.
    pub fn apply_decay(&mut self) {
        if self.decay_profile.pinned {
            return;
        }
        let elapsed = SystemTime::now()
            .duration_since(self.decay_profile.last_decay_at)
            .unwrap_or(Duration::ZERO);
        self.apply_decay_for(elapsed);
    }

    /// Apply exponential decay for an explicit `elapsed` duration.
    ///
    /// ```text
    /// strength = strength × exp(−decay_rate × Δhours)
    /// ```
    ///
    /// Separating the elapsed-time argument makes the core decay logic
    /// fully testable without a real-time dependency.  No-op for pinned nodes.
    pub fn apply_decay_for(&mut self, elapsed: Duration) {
        if self.decay_profile.pinned {
            return;
        }
        let hours = elapsed.as_secs_f32() / 3600.0;
        self.current_strength =
            (self.current_strength * (-self.decay_profile.decay_rate * hours).exp()).max(0.0);
        self.decay_profile.last_decay_at = SystemTime::now();
        self.ledger.push(ComputeLedgerEntry::new(
            LedgerOperation::Decay,
            0.0,
            ComputeLedgerEntry::SYSTEM_AGENT,
        ));
    }

    /// Returns `true` when `current_strength < pruning_threshold`.
    /// Always `false` for pinned nodes.
    pub fn should_prune(&self) -> bool {
        !self.decay_profile.pinned && self.current_strength < self.decay_profile.pruning_threshold
    }

    /// Append a directed causal link to another memory node.
    pub fn add_causal_link(&mut self, link: CausalLink) {
        self.causal_links.push(link);
    }

    /// Returns `true` when the stored `payload_hash` matches a fresh SHA-256
    /// of `payload` — detects in-memory corruption or unauthorised mutation.
    pub fn verify_payload_integrity(&self) -> bool {
        Self::hash_payload(&self.payload) == self.payload_hash
    }

    fn hash_payload(payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.finalize().into()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    const AGENT_A: AgentId = [1u8; 32];
    const NODE_ID: MemoryId = [42u8; 32];
    const TARGET_ID: MemoryId = [99u8; 32];

    fn basic_node() -> MemoryNode {
        MemoryNode::new(
            NODE_ID,
            VectorEmbedding::new(vec![1.0, 0.0, 0.0]),
            b"hello anima".to_vec(),
            "text/plain".to_string(),
            SensoryModality::Textual,
            AGENT_A,
        )
    }

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    // ── VectorEmbedding ──────────────────────────────────────────────────────

    #[test]
    fn embedding_new_sets_dimensions() {
        let e = VectorEmbedding::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(e.dimensions, 3);
        assert!(!e.normalized);
    }

    #[test]
    fn embedding_normalize_produces_unit_vector() {
        let mut e = VectorEmbedding::new(vec![3.0, 4.0]); // magnitude = 5
        e.normalize().unwrap();
        let mag: f32 = e.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(approx_eq(mag, 1.0, 1e-6), "magnitude={mag}");
        assert!(e.normalized);
    }

    #[test]
    fn embedding_normalize_zero_vector_errors() {
        let mut e = VectorEmbedding::new(vec![0.0, 0.0, 0.0]);
        assert!(matches!(e.normalize(), Err(SseError::ZeroMagnitude)));
    }

    #[test]
    fn cosine_similarity_identical_normalised_vectors_is_one() {
        let mut a = VectorEmbedding::new(vec![1.0, 2.0, 3.0]);
        let mut b = VectorEmbedding::new(vec![1.0, 2.0, 3.0]);
        a.normalize().unwrap();
        b.normalize().unwrap();
        let sim = a.cosine_similarity(&b).unwrap();
        assert!(approx_eq(sim, 1.0, 1e-6), "sim={sim}");
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors_is_zero() {
        let mut a = VectorEmbedding::new(vec![1.0, 0.0]);
        let mut b = VectorEmbedding::new(vec![0.0, 1.0]);
        a.normalize().unwrap();
        b.normalize().unwrap();
        let sim = a.cosine_similarity(&b).unwrap();
        assert!(approx_eq(sim, 0.0, 1e-6), "sim={sim}");
    }

    #[test]
    fn cosine_similarity_opposite_vectors_is_minus_one() {
        let mut a = VectorEmbedding::new(vec![1.0, 0.0]);
        let mut b = VectorEmbedding::new(vec![-1.0, 0.0]);
        a.normalize().unwrap();
        b.normalize().unwrap();
        let sim = a.cosine_similarity(&b).unwrap();
        assert!(approx_eq(sim, -1.0, 1e-6), "sim={sim}");
    }

    #[test]
    fn cosine_similarity_unnormalised_matches_expected() {
        // [1,1] vs [1,0] → cos 45° = 1/√2 ≈ 0.7071
        let a = VectorEmbedding::new(vec![1.0, 1.0]);
        let b = VectorEmbedding::new(vec![1.0, 0.0]);
        let sim = a.cosine_similarity(&b).unwrap();
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            approx_eq(sim, expected, 1e-5),
            "sim={sim}, expected={expected}"
        );
    }

    #[test]
    fn cosine_similarity_dimension_mismatch_errors() {
        let a = VectorEmbedding::new(vec![1.0, 0.0]);
        let b = VectorEmbedding::new(vec![1.0, 0.0, 0.0]);
        assert!(matches!(
            a.cosine_similarity(&b),
            Err(SseError::DimensionMismatch {
                expected: 2,
                got: 3
            })
        ));
    }

    #[test]
    fn cosine_similarity_zero_vector_errors() {
        let a = VectorEmbedding::new(vec![0.0, 0.0]);
        let b = VectorEmbedding::new(vec![1.0, 0.0]);
        assert!(matches!(
            a.cosine_similarity(&b),
            Err(SseError::ZeroMagnitude)
        ));
    }

    // ── SensoryModality ──────────────────────────────────────────────────────

    #[test]
    fn sensory_modality_scalar_variants_are_distinct() {
        let variants = [
            SensoryModality::Textual,
            SensoryModality::Visual,
            SensoryModality::Audio,
            SensoryModality::Abstract,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b)
                } else {
                    assert_ne!(a, b)
                }
            }
        }
    }

    #[test]
    fn sensory_modality_multimodal_preserves_children() {
        let m = SensoryModality::MultiModal(vec![SensoryModality::Visual, SensoryModality::Audio]);
        assert_eq!(
            m,
            SensoryModality::MultiModal(vec![SensoryModality::Visual, SensoryModality::Audio])
        );
    }

    #[test]
    fn sensory_modality_multimodal_nested() {
        // Verify recursive MultiModal compiles and compares correctly.
        let inner = SensoryModality::MultiModal(vec![SensoryModality::Textual]);
        let outer = SensoryModality::MultiModal(vec![inner.clone(), SensoryModality::Visual]);
        assert_eq!(
            outer,
            SensoryModality::MultiModal(vec![inner, SensoryModality::Visual])
        );
    }

    // ── CausalLinkType ───────────────────────────────────────────────────────

    #[test]
    fn causal_link_type_all_variants_are_distinct() {
        let types = [
            CausalLinkType::Caused,
            CausalLinkType::Preceded,
            CausalLinkType::PartOf,
            CausalLinkType::SimilarTo,
            CausalLinkType::Contradicts,
            CausalLinkType::Abstracts,
        ];
        for (i, a) in types.iter().enumerate() {
            for (j, b) in types.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b)
                } else {
                    assert_ne!(a, b)
                }
            }
        }
    }

    // ── CausalLink ───────────────────────────────────────────────────────────

    #[test]
    fn causal_link_clamps_strength_above_one() {
        let link = CausalLink::new(TARGET_ID, CausalLinkType::Caused, 2.5);
        assert!(approx_eq(link.strength, 1.0, 1e-6));
    }

    #[test]
    fn causal_link_clamps_strength_below_zero() {
        let link = CausalLink::new(TARGET_ID, CausalLinkType::Caused, -0.5);
        assert!(approx_eq(link.strength, 0.0, 1e-6));
    }

    #[test]
    fn causal_link_starts_with_zero_traversals() {
        let link = CausalLink::new(TARGET_ID, CausalLinkType::SimilarTo, 0.9);
        assert_eq!(link.traversal_count, 0);
    }

    #[test]
    fn causal_link_stores_correct_type_and_target() {
        let link = CausalLink::new(TARGET_ID, CausalLinkType::Abstracts, 0.7);
        assert_eq!(link.target_id, TARGET_ID);
        assert_eq!(link.link_type, CausalLinkType::Abstracts);
    }

    // ── DecayProfile ─────────────────────────────────────────────────────────

    #[test]
    fn decay_profile_default_values_are_sensible() {
        let p = DecayProfile::default();
        assert!(
            p.decay_rate > 0.0 && p.decay_rate < 1.0,
            "decay_rate={}",
            p.decay_rate
        );
        assert!(
            p.reinforcement_factor > 1.0,
            "reinforcement_factor={}",
            p.reinforcement_factor
        );
        assert!(approx_eq(p.max_strength, 1.0, 1e-6));
        assert!(
            p.pruning_threshold > 0.0 && p.pruning_threshold < p.max_strength,
            "threshold={}",
            p.pruning_threshold
        );
        assert!(!p.pinned);
    }

    // ── ComputeLedgerEntry ───────────────────────────────────────────────────

    #[test]
    fn ledger_entry_stores_all_operation_types() {
        for op in [
            LedgerOperation::Read,
            LedgerOperation::Write,
            LedgerOperation::Decay,
            LedgerOperation::Prune,
        ] {
            let entry = ComputeLedgerEntry::new(op.clone(), 0.42, AGENT_A);
            assert_eq!(entry.operation, op);
            assert!(approx_eq(entry.cost as f32, 0.42, 1e-5));
            assert_eq!(entry.agent_id, AGENT_A);
        }
    }

    #[test]
    fn ledger_system_agent_sentinel_is_all_zeros() {
        assert_eq!(ComputeLedgerEntry::SYSTEM_AGENT, [0u8; 32]);
    }

    // ── MemoryMetadata ───────────────────────────────────────────────────────

    #[test]
    fn metadata_initial_state() {
        let meta = MemoryMetadata::new(AGENT_A);
        assert_eq!(meta.access_count, 0);
        assert_eq!(meta.creator_agent_id, AGENT_A);
        assert!(meta.label.is_none());
        assert!(meta.tags.is_empty());
        assert!(meta.custom.is_empty());
    }

    // ── MemoryNode ───────────────────────────────────────────────────────────

    #[test]
    fn memory_node_creation_initial_state() {
        let node = basic_node();
        assert_eq!(node.id, NODE_ID);
        assert!(approx_eq(node.current_strength, 1.0, 1e-6));
        assert_eq!(node.version, 1);
        assert!(node.causal_links.is_empty());
        // Write entry appended on construction
        assert_eq!(node.ledger.len(), 1);
        assert_eq!(node.ledger[0].operation, LedgerOperation::Write);
        assert_eq!(node.ledger[0].agent_id, AGENT_A);
    }

    #[test]
    fn record_access_increments_access_count() {
        let mut node = basic_node();
        node.record_access(AGENT_A);
        node.record_access(AGENT_A);
        assert_eq!(node.metadata.access_count, 2);
    }

    #[test]
    fn record_access_reinforces_strength() {
        let mut node = basic_node();
        node.current_strength = 0.5;
        let factor = node.decay_profile.reinforcement_factor;
        let max = node.decay_profile.max_strength;
        node.record_access(AGENT_A);
        let expected = (0.5 * factor).min(max);
        assert!(approx_eq(node.current_strength, expected, 1e-6));
    }

    #[test]
    fn record_access_caps_at_max_strength() {
        let mut node = basic_node(); // starts at 1.0 == max_strength
        for _ in 0..5 {
            node.record_access(AGENT_A);
        }
        assert!(approx_eq(
            node.current_strength,
            node.decay_profile.max_strength,
            1e-6
        ));
    }

    #[test]
    fn record_access_appends_read_ledger_entry() {
        let mut node = basic_node();
        let before = node.ledger.len();
        node.record_access(AGENT_A);
        assert_eq!(node.ledger.len(), before + 1);
        let last = node.ledger.last().unwrap();
        assert_eq!(last.operation, LedgerOperation::Read);
        assert_eq!(last.agent_id, AGENT_A);
    }

    #[test]
    fn apply_decay_for_reduces_strength() {
        let mut node = basic_node();
        let before = node.current_strength;
        node.apply_decay_for(Duration::from_secs(100 * 3600)); // 100 hours
        assert!(
            node.current_strength < before,
            "strength should have decayed"
        );
    }

    #[test]
    fn apply_decay_for_follows_exponential_formula() {
        let mut node = basic_node();
        let rate = node.decay_profile.decay_rate;
        let hours: f32 = 50.0;
        node.apply_decay_for(Duration::from_secs(50 * 3600));
        let expected = (-rate * hours).exp();
        assert!(
            approx_eq(node.current_strength, expected, 1e-5),
            "strength={}, expected={expected}",
            node.current_strength
        );
    }

    #[test]
    fn apply_decay_for_pinned_node_leaves_strength_unchanged() {
        let mut node = basic_node();
        node.decay_profile.pinned = true;
        node.apply_decay_for(Duration::from_secs(1_000_000));
        assert!(approx_eq(node.current_strength, 1.0, 1e-6));
    }

    #[test]
    fn apply_decay_for_pinned_node_appends_no_ledger_entry() {
        let mut node = basic_node();
        node.decay_profile.pinned = true;
        let before = node.ledger.len();
        node.apply_decay_for(Duration::from_secs(3600));
        assert_eq!(node.ledger.len(), before, "pinned node must not log decay");
    }

    #[test]
    fn apply_decay_for_appends_decay_ledger_entry() {
        let mut node = basic_node();
        let before = node.ledger.len();
        node.apply_decay_for(Duration::from_secs(3600));
        assert_eq!(node.ledger.len(), before + 1);
        let last = node.ledger.last().unwrap();
        assert_eq!(last.operation, LedgerOperation::Decay);
        assert_eq!(last.agent_id, ComputeLedgerEntry::SYSTEM_AGENT);
    }

    #[test]
    fn apply_decay_for_never_produces_negative_strength() {
        let mut node = basic_node();
        // Absurdly high decay + very long elapsed: result must stay ≥ 0
        node.decay_profile.decay_rate = 100.0;
        node.apply_decay_for(Duration::from_secs(1_000_000));
        assert!(node.current_strength >= 0.0);
    }

    #[test]
    fn should_prune_false_when_above_threshold() {
        let node = basic_node(); // strength = 1.0, threshold = 0.05
        assert!(!node.should_prune());
    }

    #[test]
    fn should_prune_true_when_below_threshold() {
        let mut node = basic_node();
        node.current_strength = node.decay_profile.pruning_threshold - 0.001;
        assert!(node.should_prune());
    }

    #[test]
    fn should_prune_false_at_exactly_threshold() {
        let mut node = basic_node();
        node.current_strength = node.decay_profile.pruning_threshold;
        // threshold is exclusive: strength < threshold triggers prune
        assert!(!node.should_prune());
    }

    #[test]
    fn should_prune_never_true_for_pinned_node() {
        let mut node = basic_node();
        node.decay_profile.pinned = true;
        node.current_strength = 0.0;
        assert!(!node.should_prune(), "pinned nodes must never be pruned");
    }

    #[test]
    fn add_causal_link_appends_to_list() {
        let mut node = basic_node();
        let link = CausalLink::new(TARGET_ID, CausalLinkType::Caused, 0.8);
        node.add_causal_link(link);
        assert_eq!(node.causal_links.len(), 1);
        assert_eq!(node.causal_links[0].link_type, CausalLinkType::Caused);
        assert_eq!(node.causal_links[0].target_id, TARGET_ID);
    }

    #[test]
    fn add_multiple_causal_links_preserves_all() {
        let mut node = basic_node();
        node.add_causal_link(CausalLink::new(TARGET_ID, CausalLinkType::Caused, 0.9));
        node.add_causal_link(CausalLink::new([7u8; 32], CausalLinkType::SimilarTo, 0.6));
        node.add_causal_link(CausalLink::new([8u8; 32], CausalLinkType::Preceded, 0.4));
        assert_eq!(node.causal_links.len(), 3);
        assert_eq!(node.causal_links[1].link_type, CausalLinkType::SimilarTo);
    }

    #[test]
    fn payload_hash_matches_on_creation() {
        let node = basic_node();
        assert!(node.verify_payload_integrity());
    }

    #[test]
    fn payload_hash_detects_single_byte_corruption() {
        let mut node = basic_node();
        node.payload[0] ^= 0xFF; // flip all bits in first byte
        assert!(!node.verify_payload_integrity());
    }
}
