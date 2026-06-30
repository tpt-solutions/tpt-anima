//! Thought Bubble fault isolation — manages creation, quarantine, and teardown.

use crate::types::*;
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

// ── ThoughtBubbleManager ────────────────────────────────────────────────────

pub struct ThoughtBubbleManager {
    bubbles: HashMap<Uuid, ThoughtBubble>,
}

impl ThoughtBubbleManager {
    pub fn new() -> Self {
        Self {
            bubbles: HashMap::new(),
        }
    }

    /// Create a new ThoughtBubble with the given isolation level and
    /// optional memory regions.
    pub fn create_bubble(
        &mut self,
        sandbox_id: String,
        isolation_level: IsolationLevel,
        memory_regions: Vec<MemoryRegion>,
    ) -> &ThoughtBubble {
        let id = Uuid::new_v4();
        let bubble = ThoughtBubble {
            id,
            sandbox_id,
            isolation_level,
            memory_regions,
            created_at: SystemTime::now(),
            is_compromised: false,
        };
        self.bubbles.insert(id, bubble);
        &self.bubbles[&id]
    }

    /// Quarantine a bubble by marking it as compromised.
    /// Returns `true` if the bubble existed and was not already compromised.
    pub fn quarantine(&mut self, id: Uuid) -> bool {
        match self.bubbles.get_mut(&id) {
            Some(bubble) if !bubble.is_compromised => {
                bubble.is_compromised = true;
                true
            }
            _ => false,
        }
    }

    /// Remove a bubble from the manager. Returns `true` if it existed.
    pub fn teardown(&mut self, id: Uuid) -> bool {
        self.bubbles.remove(&id).is_some()
    }

    /// Returns the count of active (non-compromised) bubbles.
    pub fn active_bubbles(&self) -> usize {
        self.bubbles.values().filter(|b| !b.is_compromised).count()
    }

    /// Returns the count of all bubbles (compromised or not).
    pub fn total_bubbles(&self) -> usize {
        self.bubbles.len()
    }

    /// Check if a specific bubble is quarantined (compromised).
    pub fn is_quarantined(&self, id: Uuid) -> bool {
        self.bubbles.get(&id).is_some_and(|b| b.is_compromised)
    }

    /// Get a reference to a bubble by id.
    pub fn get(&self, id: Uuid) -> Option<&ThoughtBubble> {
        self.bubbles.get(&id)
    }

    /// List all bubble ids.
    pub fn list_ids(&self) -> Vec<Uuid> {
        self.bubbles.keys().copied().collect()
    }
}

impl Default for ThoughtBubbleManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_region(region_type: MemoryRegionType) -> MemoryRegion {
        MemoryRegion {
            id: Uuid::new_v4(),
            size_bytes: 1024,
            region_type,
            data_hash: [0xaa; 32],
        }
    }

    #[test]
    fn create_bubble_returns_unique_id() {
        let mut mgr = ThoughtBubbleManager::new();
        let id1 = mgr
            .create_bubble("sb-1".into(), IsolationLevel::Enclave, vec![])
            .id;
        let id2 = mgr
            .create_bubble("sb-2".into(), IsolationLevel::Process, vec![])
            .id;
        assert_ne!(id1, id2);
        assert_eq!(mgr.total_bubbles(), 2);
        assert_eq!(mgr.active_bubbles(), 2);
    }

    #[test]
    fn quarantine_marks_compromised() {
        let mut mgr = ThoughtBubbleManager::new();
        let b = mgr.create_bubble("sb-1".into(), IsolationLevel::Thread, vec![]);
        let id = b.id;
        assert!(mgr.quarantine(id));
        assert!(mgr.is_quarantined(id));
        assert_eq!(mgr.active_bubbles(), 0);
    }

    #[test]
    fn quarantine_already_compromised_returns_false() {
        let mut mgr = ThoughtBubbleManager::new();
        let b = mgr.create_bubble("sb-1".into(), IsolationLevel::Thread, vec![]);
        let id = b.id;
        assert!(mgr.quarantine(id));
        assert!(!mgr.quarantine(id));
    }

    #[test]
    fn teardown_removes_bubble() {
        let mut mgr = ThoughtBubbleManager::new();
        let b = mgr.create_bubble("sb-1".into(), IsolationLevel::Process, vec![]);
        let id = b.id;
        assert!(mgr.teardown(id));
        assert_eq!(mgr.total_bubbles(), 0);
        assert!(mgr.get(id).is_none());
    }

    #[test]
    fn teardown_nonexistent_returns_false() {
        let mut mgr = ThoughtBubbleManager::new();
        assert!(!mgr.teardown(Uuid::new_v4()));
    }

    #[test]
    fn create_with_memory_regions() {
        let mut mgr = ThoughtBubbleManager::new();
        let regions = vec![
            sample_region(MemoryRegionType::VRAM),
            sample_region(MemoryRegionType::RAM),
        ];
        let b = mgr.create_bubble("sb-1".into(), IsolationLevel::Enclave, regions);
        assert_eq!(b.memory_regions.len(), 2);
        assert_eq!(b.isolation_level, IsolationLevel::Enclave);
        assert!(!b.is_compromised);
    }

    #[test]
    fn list_ids_returns_all() {
        let mut mgr = ThoughtBubbleManager::new();
        let id1 = mgr
            .create_bubble("a".into(), IsolationLevel::Process, vec![])
            .id;
        let id2 = mgr
            .create_bubble("b".into(), IsolationLevel::Hypervisor, vec![])
            .id;
        let ids = mgr.list_ids();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn default_creates_empty_manager() {
        let mgr = ThoughtBubbleManager::default();
        assert_eq!(mgr.total_bubbles(), 0);
        assert_eq!(mgr.active_bubbles(), 0);
    }
}
