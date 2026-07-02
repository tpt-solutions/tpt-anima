//! Cognitive Kernel — the brainstem of TPT Anima.
//!
//! The fundamental compute unit is an *inference*, not a thread.
//! The CK manages attention, context, and tensor execution natively.
//! Phase 3 implementation lives in sub-modules added during Months 7–9.

pub mod budgeting;
pub mod circuit_breaker;
pub mod jit;
pub mod kv_swap_optimization;
pub mod mcp;
pub mod memory_fabric;
pub mod scheduler;
pub mod thought_bubble;
pub mod types;

pub use budgeting::{BudgetError, BudgetManager, ComputeBudget, ComputeUnit, ThermalStatus};
pub use kv_swap_optimization::{
    ContextBuffer, KVCacheOptimizer, SwapOperation, SwapPlan, SwapPriority, SwapStatistics,
    SwapStrategy,
};
