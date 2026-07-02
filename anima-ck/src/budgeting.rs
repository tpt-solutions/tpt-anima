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
    current_power_watts: f32,
    /// Per-hardware-tier temperature readings. Falls back to
    /// `current_temperature` for units with no reading of their own, since
    /// CPU/GPU/NPU dies each carry independent thermal sensors in practice.
    unit_temperatures: HashMap<ComputeUnit, f32>,
}

impl BudgetManager {
    pub fn new(budget: ComputeBudget) -> Self {
        Self {
            total_budget: budget,
            allocations: HashMap::new(),
            current_temperature: 25.0, // ambient default
            current_power_watts: 0.0,
            unit_temperatures: HashMap::new(),
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

    /// Record current power draw (watts) from hardware telemetry.
    pub fn update_power_draw(&mut self, watts: f32) {
        self.current_power_watts = watts.max(0.0);
    }

    /// Current power draw in watts.
    pub fn current_power_draw(&self) -> f32 {
        self.current_power_watts
    }

    /// Remaining power headroom before hitting `max_power_watts`.
    pub fn power_headroom_watts(&self) -> f32 {
        (self.total_budget.thermal_constraints.max_power_watts - self.current_power_watts).max(0.0)
    }

    /// Whether current power draw has reached the configured budget.
    pub fn is_power_constrained(&self) -> bool {
        self.current_power_watts >= self.total_budget.thermal_constraints.max_power_watts
    }

    /// Fraction (0.0..=1.0) of a requested FLOP allocation that should be
    /// granted at the current thermal status. Allocations are scaled down
    /// progressively as temperature rises, rather than cut off abruptly, so
    /// the system sheds load smoothly instead of oscillating between full
    /// allocation and thermal shutdown.
    pub fn thermal_throttle_factor(&self) -> f32 {
        Self::throttle_factor_for_status(self.thermal_status())
    }

    fn throttle_factor_for_status(status: ThermalStatus) -> f32 {
        match status {
            ThermalStatus::Cool | ThermalStatus::Warm => 1.0,
            ThermalStatus::Hot => 0.75,
            ThermalStatus::Throttled => 0.4,
            ThermalStatus::Critical => 0.0,
        }
    }

    /// Record a temperature reading for a specific hardware tier (CPU, GPU,
    /// or NPU). Each tier has its own thermal envelope in real hardware —
    /// a hot GPU die shouldn't throttle CPU-bound inference and vice versa.
    pub fn update_unit_temperature(&mut self, unit: ComputeUnit, temp_celsius: f32) -> ThermalStatus {
        self.unit_temperatures.insert(unit, temp_celsius);
        self.unit_thermal_status(unit)
    }

    /// Thermal status for a specific hardware tier. Falls back to the
    /// system-wide reading if this unit has never reported its own.
    pub fn unit_thermal_status(&self, unit: ComputeUnit) -> ThermalStatus {
        let temp = self
            .unit_temperatures
            .get(&unit)
            .copied()
            .unwrap_or(self.current_temperature);
        ThermalStatus::from_temperature(temp, self.total_budget.thermal_constraints.max_temp_celsius)
    }

    /// Throttle factor for a specific hardware tier, mirroring
    /// [`thermal_throttle_factor`](Self::thermal_throttle_factor) but scoped
    /// to one compute unit's own thermal envelope.
    pub fn unit_thermal_throttle_factor(&self, unit: ComputeUnit) -> f32 {
        Self::throttle_factor_for_status(self.unit_thermal_status(unit))
    }

    /// Thermal- and power-aware FLOP allocation.
    ///
    /// On top of the plain FLOPS budget check in [`allocate_flops`](Self::allocate_flops),
    /// this refuses new allocations once the power budget is exhausted and
    /// scales the granted FLOPS down under thermal pressure
    /// (see [`thermal_throttle_factor`](Self::thermal_throttle_factor)).
    /// Returns the FLOPS actually granted, which may be less than requested.
    pub fn allocate_flops_constrained(
        &mut self,
        unit: ComputeUnit,
        flops: u64,
        estimated_power_watts: f32,
    ) -> Result<u64, BudgetError> {
        if self.is_power_constrained() {
            return Err(BudgetError::PowerBudgetExceeded {
                current_watts: self.current_power_watts,
                max_watts: self.total_budget.thermal_constraints.max_power_watts,
            });
        }

        let factor = self.unit_thermal_throttle_factor(unit);
        if factor <= 0.0 {
            return Err(BudgetError::ThermalCritical {
                temperature_celsius: self
                    .unit_temperatures
                    .get(&unit)
                    .copied()
                    .unwrap_or(self.current_temperature),
            });
        }

        let throttled_flops = ((flops as f64) * factor as f64) as u64;
        let granted = self
            .allocate_flops(unit, throttled_flops)
            .map_err(BudgetError::FlopsBudgetExceeded)?;

        self.current_power_watts += estimated_power_watts.max(0.0);
        Ok(granted)
    }

    /// Release power previously accounted for by [`allocate_flops_constrained`](Self::allocate_flops_constrained).
    pub fn release_power_watts(&mut self, watts: f32) {
        self.current_power_watts = (self.current_power_watts - watts.max(0.0)).max(0.0);
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

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum BudgetError {
    #[error("FLOPS budget exceeded: {0} remaining")]
    FlopsBudgetExceeded(u64),
    #[error("power budget exceeded: {current_watts}W drawn of {max_watts}W max")]
    PowerBudgetExceeded { current_watts: f32, max_watts: f32 },
    #[error("thermal critical at {temperature_celsius}°C — allocation refused")]
    ThermalCritical { temperature_celsius: f32 },
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

    #[test]
    fn power_draw_tracks_headroom_and_constraint() {
        let mut mgr = BudgetManager::new(test_budget());
        assert_eq!(mgr.power_headroom_watts(), 300.0);
        assert!(!mgr.is_power_constrained());

        mgr.update_power_draw(300.0);
        assert_eq!(mgr.power_headroom_watts(), 0.0);
        assert!(mgr.is_power_constrained());
    }

    #[test]
    fn thermal_throttle_factor_scales_with_status() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_temperature(30.0); // Cool
        assert_eq!(mgr.thermal_throttle_factor(), 1.0);

        mgr.update_temperature(62.0); // Hot
        assert_eq!(mgr.thermal_throttle_factor(), 0.75);

        mgr.update_temperature(73.0); // Throttled
        assert_eq!(mgr.thermal_throttle_factor(), 0.4);

        mgr.update_temperature(80.0); // Critical
        assert_eq!(mgr.thermal_throttle_factor(), 0.0);
    }

    #[test]
    fn allocate_flops_constrained_scales_down_when_hot() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_temperature(62.0); // Hot -> 0.75 factor

        let granted = mgr
            .allocate_flops_constrained(ComputeUnit::GPU, 400, 100.0)
            .unwrap();

        assert_eq!(granted, 300); // 400 * 0.75
        assert_eq!(mgr.current_power_draw(), 100.0);
    }

    #[test]
    fn allocate_flops_constrained_refuses_at_critical_temperature() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_temperature(80.0); // Critical

        let result = mgr.allocate_flops_constrained(ComputeUnit::GPU, 400, 100.0);
        assert!(matches!(result, Err(BudgetError::ThermalCritical { .. })));
        assert_eq!(mgr.current_power_draw(), 0.0);
    }

    #[test]
    fn allocate_flops_constrained_refuses_when_power_exhausted() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_power_draw(300.0); // at max_power_watts

        let result = mgr.allocate_flops_constrained(ComputeUnit::GPU, 400, 50.0);
        assert!(matches!(result, Err(BudgetError::PowerBudgetExceeded { .. })));
    }

    #[test]
    fn release_power_watts_frees_headroom() {
        let mut mgr = BudgetManager::new(test_budget());
        mgr.update_power_draw(200.0);
        mgr.release_power_watts(150.0);
        assert_eq!(mgr.current_power_draw(), 50.0);

        // Never goes negative.
        mgr.release_power_watts(1000.0);
        assert_eq!(mgr.current_power_draw(), 0.0);
    }
}
