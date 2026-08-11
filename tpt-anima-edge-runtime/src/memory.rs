use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("entry not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub data: Vec<u8>,
    pub strength: f32,
    pub tags: Vec<String>,
}

/// Lightweight in-memory store for edge devices.
/// Holds memory entries and supports attention-weighted retrieval and decay.
#[derive(Debug, Clone)]
pub struct EdgeMemory {
    entries: Vec<MemoryEntry>,
}

impl EdgeMemory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn store(&mut self, data: Vec<u8>, tags: Vec<String>) -> String {
        let id = Uuid::new_v4().to_string();
        self.entries.push(MemoryEntry {
            id: id.clone(),
            data,
            strength: 1.0,
            tags,
        });
        id
    }

    pub fn retrieve(&self, id: &str) -> Result<&MemoryEntry, MemoryError> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| MemoryError::NotFound(id.to_string()))
    }

    /// Retrieve entries sorted by `strength` descending, limited to `limit`.
    /// Accesses reinforce strength via multiplicative boost.
    pub fn attend(&mut self, limit: usize) -> Vec<&MemoryEntry> {
        self.entries.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for entry in self.entries.iter_mut().take(limit) {
            entry.strength = (entry.strength * 1.1).min(1.0);
        }
        self.entries.iter().take(limit).collect()
    }

    /// Apply exponential decay to all entries.
    /// `factor` should be in (0, 1) — e.g. 0.95 for mild decay.
    pub fn decay(&mut self, factor: f32) {
        for entry in &mut self.entries {
            entry.strength *= factor;
        }
    }

    /// Remove entries whose strength drops below `threshold`.
    pub fn prune(&mut self, threshold: f32) {
        self.entries.retain(|e| e.strength >= threshold);
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.entries.iter().map(|e| e.data.len()).sum()
    }
}

impl Default for EdgeMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_store_is_empty() {
        let mem = EdgeMemory::new();
        assert_eq!(mem.count(), 0);
        assert_eq!(mem.total_bytes(), 0);
    }

    #[test]
    fn store_and_retrieve() {
        let mut mem = EdgeMemory::new();
        let id = mem.store(b"hello".to_vec(), vec!["greeting".into()]);
        let entry = mem.retrieve(&id).unwrap();
        assert_eq!(entry.data, b"hello");
        assert_eq!(entry.strength, 1.0);
        assert!(entry.tags.contains(&"greeting".to_string()));
    }

    #[test]
    fn retrieve_nonexistent_returns_error() {
        let mem = EdgeMemory::new();
        assert!(mem.retrieve("nope").is_err());
    }

    #[test]
    fn attend_returns_strongest_entries() {
        let mut mem = EdgeMemory::new();
        let _id1 = mem.store(vec![1], vec![]);
        let id2 = mem.store(vec![2], vec![]);
        // Make entry 1 weaker by manipulating the store directly via decay,
        // then set id2 back to full strength
        mem.decay(0.1); // both become 0.1
                        // Manually reset id2 to full strength for the test
        if let Some(e) = mem.entries.iter_mut().find(|e| e.id == id2) {
            e.strength = 1.0;
        }
        let results = mem.attend(1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id2);
    }

    #[test]
    fn decay_reduces_strength() {
        let mut mem = EdgeMemory::new();
        mem.store(vec![0], vec![]);
        mem.decay(0.5);
        // Check strength directly without attend (attend modifies strength)
        let entry = &mem.entries[0];
        assert!((entry.strength - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn prune_removes_weak_entries() {
        let mut mem = EdgeMemory::new();
        mem.store(vec![0], vec![]);
        mem.decay(0.01);
        mem.prune(0.5);
        assert_eq!(mem.count(), 0);
    }

    #[test]
    fn total_bytes_is_sum_of_data_lengths() {
        let mut mem = EdgeMemory::new();
        mem.store(vec![0; 10], vec![]);
        mem.store(vec![0; 5], vec![]);
        assert_eq!(mem.total_bytes(), 15);
    }
}
