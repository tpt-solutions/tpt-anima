//! Compute Budgeting & Thermal management.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Compute unit ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeUnit {
    CPU,
    GPU,
    NPU,
}

// ── Thermal status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThermalStatus {
    Cool,
    Warm,
    Hot,
    Critical,
    Throttled,
}

impl ThermalStatus {
    pub fn from_temperature(temp_celsius: f32, max_temp: f32) -> Self {
        let ratio = temp_celsius / max_temp;
        if ratio >= 1.0 {
            ThermalStatus::Critical
        } else if ratio >= 0.9 {
            ThermalStatus::Throttled
        } else if ratio >= 0.75 {
            ThermalStatus::Hot
        } else if ratio >= 0.5 {
            ThermalStatus::Warm
        } else {
            ThermalStatus::Cool
        }
    }
}

// ── Compute Budget ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeBudget {
    pub flops_budget: u64,
    pub thermal_constraints: ThermalConstraints,
}

// ── Budget Manager ──────────────────────────────────────────────────────────

pub struct BudgetManager {
    total_budget: ComputeBudget,
    allocations: HashMap<ComputeUnit, u64>,
    current_temperature: f32,
}

impl BudgetManager {
    pub fn new(budget: ComputeBudget) -> Self {
        Self {
            total_budget: budget,
            allocations: HashMap::new(),
            current_temperature: 25.0, // ambient default
        }
    }

    /// Allocate FLOPS to a compute unit. Returns `Ok(allocated)` on success,
    /// or `Err(remaining_budget)` if insufficient budget remains.
    pub fn allocate_flops(&mut self, unit: ComputeUnit, flops: u64) -> Result<u64, u64> {
        let current = self.allocations.get(&unit).copied().unwrap_or(0);
        let total_used: u64 = self.allocations.values().sum();
        let remaining = self.total_budget.flops_budget.saturating_sub(total_used);

        if flops > remaining {
            return Err(remaining);
        }

        *self.allocations.entry(unit).or_insert(0) += flops;
        Ok(current + flops)
    }

    /// Release FLOPS from a compute unit. Returns `true` if the unit had
    /// an allocation.
    pub fn release_flops(&mut self, unit: ComputeUnit, flops: u64) -> bool {
        match self.allocations.get_mut(&unit) {
            Some(alloc) => {
                *alloc = alloc.saturating_sub(flops);
                if *alloc == 0 {
                    self.allocations.remove(&unit);
                }
                true
            }
            None => false,
        }
    }

    /// Update the current temperature reading and return thermal status.
    pub fn update_temperature(&mut self, temp_celsius: f32) -> ThermalStatus {
        self.current_temperature = temp_celsius;
        self.thermal_status()
    }

    /// Current thermal status based on configured constraints.
    pub fn thermal_status(&self) -> ThermalStatus {
        ThermalStatus::from_temperature(
            self.current_temperature,
            self.total_budget.thermal_constraints.max_temp_celsius,
        )
    }

    /// Total FLOPS currently allocated across all units.
    pub fn current_allocation(&self) -> u64 {
        self.allocations.values().sum()
    }

    /// FLOPS remaining in the budget.
    pub fn remaining_budget(&self) -> u64 {
        self.total_budget
            .flops_budget
            .saturating_sub(self.current_allocation())
    }

    /// Allocation for a specific compute unit.
    pub fn allocation_for(&self, unit: ComputeUnit) -> u64 {
        self.allocations.get(&unit).copied().unwrap_or(0)
    }

    /// Get a snapshot of all allocations.
    pub fn all_allocations(&self) -> &HashMap<ComputeUnit, u64> {
        &self.allocations
    }
}

impl Default for BudgetManager {
    fn default() -> Self {
        Self::new(ComputeBudget {
            flops_budget: 1_000_000_000_000, // 1 TFLOP
            thermal_constraints: ThermalConstraints {
                max_temp_celsius: 85.0,
                max_power_watts: 350.0,
            },
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_budget() -> ComputeBudget {
        ComputeBudget {
            flops_budget: 1000,
            thermal_constraints: ThermalConstraints {
                max_temp_celsius: 80.0,
                max_power_watts: 300.0,
            },
        }
    }

    #[test]
    fn allocate_flops_success() {
        let mut mgr = BudgetManager::new(test_budget());
        let allocated = mgr.allocate_flops(ComputeUnit::GPU, 400).unwrap();
        assert_eq!(allocated, 400);
        assert_eq!(mgr.current_allocation(), 400);
        assert_eq!(mgr.remaining_budget(), 600);
    }

    #[test]
    fn allocate_flops_exceeds_budget() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.allocate_flops(ComputeUnit::GPU, 800).unwrap();
        let err = mgr.allocate_flops(ComputeUnit::CPU, 300).unwrap_err();
        assert_eq!(err, 200); // remaining budget
    }

    #[test]
    fn release_flops_reduces_allocation() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.allocate_flops(ComputeUnit::GPU, 500).unwrap();
        assert!(mgr.release_flops(ComputeUnit::GPU, 200));
        assert_eq!(mgr.allocation_for(ComputeUnit::GPU), 300);
        assert_eq!(mgr.current_allocation(), 300);
    }

    #[test]
    fn release_flops_nonexistent_unit_returns_false() {
        let mut mgr = BudgetManager::new(test_budget());
        assert!(!mgr.release_flops(ComputeUnit::NPU, 100));
    }

    #[test]
    fn thermal_status_cool() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_temperature(30.0);
        assert_eq!(mgr.thermal_status(), ThermalStatus::Cool);
    }

    #[test]
    fn thermal_status_hot() {
        let mut mgr = BudgetManager::new(test_budget());
        // 80 * 0.75 = 60, so 62 should be Hot
        mgr.update_temperature(62.0);
        assert_eq!(mgr.thermal_status(), ThermalStatus::Hot);
    }

    #[test]
    fn thermal_status_critical() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_temperature(80.0);
        assert_eq!(mgr.thermal_status(), ThermalStatus::Critical);
    }

    #[test]
    fn thermal_status_throttled() {
        let mut mgr = BudgetManager::new(test_budget());
        // 80 * 0.9 = 72, so 73 should be Throttled
        mgr.update_temperature(73.0);
        assert_eq!(mgr.thermal_status(), ThermalStatus::Throttled);
    }

    #[test]
    fn multiple_unit_allocations() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.allocate_flops(ComputeUnit::CPU, 200).unwrap();
        mgr.allocate_flops(ComputeUnit::GPU, 300).unwrap();
        mgr.allocate_flops(ComputeUnit::NPU, 100).unwrap();
        assert_eq!(mgr.current_allocation(), 600);
        assert_eq!(mgr.remaining_budget(), 400);
        assert_eq!(mgr.allocation_for(ComputeUnit::GPU), 300);
    }

    #[test]
    fn default_budget_manager() {
        let mgr = BudgetManager::default();
        assert_eq!(mgr.current_allocation(), 0);
        assert_eq!(mgr.thermal_status(), ThermalStatus::Cool);
    }
}
