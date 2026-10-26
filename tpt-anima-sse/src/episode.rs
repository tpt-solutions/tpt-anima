// SPDX-License-Identifier: MIT OR Apache-2.0
//! Episode records: an append-only history of runs that can be replayed in order (§14, §48.2).
//!
//! The store keeps each episode as an opaque payload under a caller-chosen kind. SSE does not
//! interpret either, so the owner of the episode format can change it without a change here.
//! Records come back in append order, which is the order replay needs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One stored episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeRecord {
    /// Position in append order, assigned by the store.
    pub sequence: u64,
    /// Caller-chosen identifier, unique within the store.
    pub episode_id: String,
    /// Caller-defined format tag, such as `"thought.episode.v1"`. The store does not read it.
    pub kind: String,
    /// When the episode started, in nanoseconds since the Unix epoch.
    pub started_at_nanos: u64,
    /// The caller's encoding of the episode.
    pub payload: Vec<u8>,
}

/// Why an episode could not be appended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpisodeStoreError {
    /// The identifier is empty.
    EmptyId,
    /// An episode with this identifier is already stored. Stored episodes are not rewritten.
    DuplicateId(String),
}

impl std::fmt::Display for EpisodeStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "episode identifier is empty"),
            Self::DuplicateId(id) => write!(f, "episode '{id}' is already stored"),
        }
    }
}

impl std::error::Error for EpisodeStoreError {}

/// Append-only episode storage, held in memory alongside the SSE memory store.
#[derive(Debug, Default)]
pub struct EpisodeStore {
    records: Vec<EpisodeRecord>,
    index: HashMap<String, usize>,
}

impl EpisodeStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an episode and return its sequence number. Identifiers must be non-empty and not
    /// already stored; a refused append stores nothing.
    pub fn append(
        &mut self,
        episode_id: &str,
        kind: &str,
        started_at_nanos: u64,
        payload: Vec<u8>,
    ) -> Result<u64, EpisodeStoreError> {
        if episode_id.is_empty() {
            return Err(EpisodeStoreError::EmptyId);
        }
        if self.index.contains_key(episode_id) {
            return Err(EpisodeStoreError::DuplicateId(episode_id.to_string()));
        }
        let sequence = self.records.len() as u64;
        self.index
            .insert(episode_id.to_string(), self.records.len());
        self.records.push(EpisodeRecord {
            sequence,
            episode_id: episode_id.to_string(),
            kind: kind.to_string(),
            started_at_nanos,
            payload,
        });
        Ok(sequence)
    }

    /// The stored episode with this identifier, if any.
    pub fn get(&self, episode_id: &str) -> Option<&EpisodeRecord> {
        self.index.get(episode_id).map(|&at| &self.records[at])
    }

    /// Every stored episode, in append order.
    pub fn iter(&self) -> impl Iterator<Item = &EpisodeRecord> {
        self.records.iter()
    }

    /// Number of stored episodes.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the store holds no episodes.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn episodes_come_back_in_append_order_with_sequence_numbers() {
        let mut store = EpisodeStore::new();
        assert_eq!(store.append("b", "k", 20, vec![2]), Ok(0));
        assert_eq!(store.append("a", "k", 10, vec![1]), Ok(1));
        let order: Vec<&str> = store.iter().map(|r| r.episode_id.as_str()).collect();
        assert_eq!(
            order,
            ["b", "a"],
            "append order, not start time or id order"
        );
        assert_eq!(store.get("a").map(|r| r.sequence), Some(1));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn duplicate_and_empty_identifiers_are_refused_and_store_nothing() {
        let mut store = EpisodeStore::new();
        store.append("a", "k", 0, vec![]).unwrap();
        assert_eq!(
            store.append("a", "k", 1, vec![9]),
            Err(EpisodeStoreError::DuplicateId("a".into()))
        );
        assert_eq!(
            store.append("", "k", 1, vec![9]),
            Err(EpisodeStoreError::EmptyId)
        );
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get("a").map(|r| r.payload.len()),
            Some(0),
            "not rewritten"
        );
    }

    #[test]
    fn payloads_and_kinds_are_kept_verbatim() {
        let mut store = EpisodeStore::new();
        store
            .append("e", "thought.episode.v1", 5, vec![0, 255, 7])
            .unwrap();
        let record = store.get("e").unwrap();
        assert_eq!(record.kind, "thought.episode.v1");
        assert_eq!(record.started_at_nanos, 5);
        assert_eq!(record.payload, [0, 255, 7]);
        assert!(!store.is_empty());
    }
}
