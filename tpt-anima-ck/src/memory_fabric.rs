//! Unified Memory Fabric — manages a unified address space across VRAM/RAM/NVMe.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── Usage stats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FabricUsageStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub by_region_type: HashMap<MemoryRegionType, u64>,
}

impl FabricUsageStats {
    pub fn free_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.used_bytes)
    }
}

// ── Access record for prefetching ───────────────────────────────────────────

#[derive(Debug, Clone)]
struct AccessRecord {
    region_id: Uuid,
    access_count: u32,
}

// ── Memory Fabric ───────────────────────────────────────────────────────────

pub struct MemoryFabric {
    regions: HashMap<Uuid, MemoryRegion>,
    total_capacity: u64,
    access_history: Vec<AccessRecord>,
}

impl MemoryFabric {
    pub fn new(total_capacity: u64) -> Self {
        Self {
            regions: HashMap::new(),
            total_capacity,
            access_history: Vec::new(),
        }
    }

    /// Allocate a region of the given size and type. Returns `None` if
    /// insufficient space remains.
    pub fn allocate(
        &mut self,
        size_bytes: u64,
        region_type: MemoryRegionType,
    ) -> Option<MemoryRegion> {
        if self.used_bytes() + size_bytes > self.total_capacity {
            return None;
        }
        let region = MemoryRegion {
            id: Uuid::new_v4(),
            size_bytes,
            region_type,
            data_hash: [0u8; 32],
        };
        self.regions.insert(region.id, region.clone());
        Some(region)
    }

    /// Deallocate a region by id. Returns `true` if the region existed.
    pub fn deallocate(&mut self, id: Uuid) -> bool {
        self.regions.remove(&id).is_some()
    }

    /// Page-in: move a region from a slower tier to a faster tier.
    pub fn page_in(&mut self, id: Uuid, target_type: MemoryRegionType) -> Option<MemoryRegion> {
        let region = self.regions.get_mut(&id)?;
        region.region_type = match (region.region_type, target_type) {
            (MemoryRegionType::NVMe, _) => target_type,
            (MemoryRegionType::RAM, MemoryRegionType::VRAM) => target_type,
            (current, _) => current,
        };
        Some(region.clone())
    }

    /// Page-out: move a region to a slower tier.
    pub fn page_out(&mut self, id: Uuid, target_type: MemoryRegionType) -> Option<MemoryRegion> {
        let region = self.regions.get_mut(&id)?;
        region.region_type = match (region.region_type, target_type) {
            (MemoryRegionType::VRAM, _) => target_type,
            (MemoryRegionType::RAM, MemoryRegionType::NVMe) => target_type,
            (current, _) => current,
        };
        Some(region.clone())
    }

    /// Predictive prefetch: returns region ids that have been accessed
    /// frequently and are on slower tiers — candidates for page-in.
    pub fn prefetch(&mut self, threshold: u32) -> Vec<Uuid> {
        let mut counts: HashMap<Uuid, u32> = HashMap::new();
        for record in &self.access_history {
            *counts.entry(record.region_id).or_insert(0) += record.access_count;
        }
        self.regions
            .iter()
            .filter(|(_, r)| r.region_type != MemoryRegionType::VRAM)
            .filter(|(id, _)| counts.get(id).copied().unwrap_or(0) >= threshold)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Record an access to a region (for prefetch heuristics).
    pub fn record_access(&mut self, id: Uuid) {
        self.access_history.push(AccessRecord {
            region_id: id,
            access_count: 1,
        });
    }

    /// Return current usage statistics.
    pub fn usage_stats(&self) -> FabricUsageStats {
        let mut by_type: HashMap<MemoryRegionType, u64> = HashMap::new();
        for r in self.regions.values() {
            *by_type.entry(r.region_type).or_insert(0) += r.size_bytes;
        }
        FabricUsageStats {
            total_bytes: self.total_capacity,
            used_bytes: self.used_bytes(),
            by_region_type: by_type,
        }
    }

    fn used_bytes(&self) -> u64 {
        self.regions.values().map(|r| r.size_bytes).sum()
    }
}

impl Default for MemoryFabric {
    fn default() -> Self {
        Self::new(8 * 1024 * 1024 * 1024) // 8 GiB default
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_usage() {
        let mut fabric = MemoryFabric::new(1024);
        let r = fabric.allocate(512, MemoryRegionType::VRAM).unwrap();
        assert_eq!(r.size_bytes, 512);
        assert_eq!(r.region_type, MemoryRegionType::VRAM);

        let stats = fabric.usage_stats();
        assert_eq!(stats.used_bytes, 512);
        assert_eq!(stats.free_bytes(), 512);
        assert_eq!(
            *stats.by_region_type.get(&MemoryRegionType::VRAM).unwrap(),
            512
        );
    }

    #[test]
    fn allocate_exceeds_capacity_returns_none() {
        let mut fabric = MemoryFabric::new(100);
        assert!(fabric.allocate(200, MemoryRegionType::RAM).is_none());
    }

    #[test]
    fn deallocate_frees_space() {
        let mut fabric = MemoryFabric::new(1024);
        let r = fabric.allocate(512, MemoryRegionType::VRAM).unwrap();
        assert!(fabric.deallocate(r.id));
        assert_eq!(fabric.usage_stats().used_bytes, 0);
    }

    #[test]
    fn deallocate_nonexistent_returns_false() {
        let mut fabric = MemoryFabric::new(1024);
        assert!(!fabric.deallocate(Uuid::new_v4()));
    }

    #[test]
    fn page_in_promotes_to_faster_tier() {
        let mut fabric = MemoryFabric::new(4096);
        let r = fabric.allocate(256, MemoryRegionType::NVMe).unwrap();
        let updated = fabric.page_in(r.id, MemoryRegionType::VRAM).unwrap();
        assert_eq!(updated.region_type, MemoryRegionType::VRAM);
    }

    #[test]
    fn page_out_demotes_to_slower_tier() {
        let mut fabric = MemoryFabric::new(4096);
        let r = fabric.allocate(256, MemoryRegionType::VRAM).unwrap();
        let updated = fabric.page_out(r.id, MemoryRegionType::NVMe).unwrap();
        assert_eq!(updated.region_type, MemoryRegionType::NVMe);
    }

    #[test]
    fn page_in_existing_ram_to_vram() {
        let mut fabric = MemoryFabric::new(4096);
        let r = fabric.allocate(128, MemoryRegionType::RAM).unwrap();
        let updated = fabric.page_in(r.id, MemoryRegionType::VRAM).unwrap();
        assert_eq!(updated.region_type, MemoryRegionType::VRAM);
    }

    #[test]
    fn prefetch_returns_frequent_slow_regions() {
        let mut fabric = MemoryFabric::new(4096);
        let r1 = fabric.allocate(128, MemoryRegionType::NVMe).unwrap();
        let _r2 = fabric.allocate(128, MemoryRegionType::VRAM).unwrap();

        // Record 3 accesses to r1 (on NVMe, below threshold of 2)
        fabric.record_access(r1.id);
        fabric.record_access(r1.id);
        fabric.record_access(r1.id);

        let candidates = fabric.prefetch(2);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], r1.id);
    }

    #[test]
    fn prefetch_skips_already_fast_regions() {
        let mut fabric = MemoryFabric::new(4096);
        let r = fabric.allocate(128, MemoryRegionType::VRAM).unwrap();
        for _ in 0..10 {
            fabric.record_access(r.id);
        }
        assert!(fabric.prefetch(2).is_empty());
    }
}
