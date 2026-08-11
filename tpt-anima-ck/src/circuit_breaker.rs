//! Cognitive Circuit Breakers — monitors inference output for anomalies.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ── Breaker state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

// ── Trigger type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BreakerTriggerType {
    EntropySpike,
    RepetitionLoop,
    CausalViolation,
}

// ── Breaker event ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerEvent {
    #[serde(with = "crate::types::serde_system_time")]
    pub timestamp: SystemTime,
    pub trigger_type: BreakerTriggerType,
    pub details: String,
}

// ── Circuit Breaker ─────────────────────────────────────────────────────────

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitBreakerState,
    consecutive_entropy_spikes: u32,
    consecutive_repetitions: u32,
    consecutive_causal_violations: u32,
    events: Vec<BreakerEvent>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitBreakerState::Closed,
            consecutive_entropy_spikes: 0,
            consecutive_repetitions: 0,
            consecutive_causal_violations: 0,
            events: Vec::new(),
        }
    }

    /// Check if entropy of the output exceeds the configured threshold.
    /// Returns `true` if an anomaly is detected.
    pub fn check_entropy(&mut self, entropy: f32) -> bool {
        if entropy > self.config.entropy_threshold {
            self.consecutive_entropy_spikes += 1;
            self.events.push(BreakerEvent {
                timestamp: SystemTime::now(),
                trigger_type: BreakerTriggerType::EntropySpike,
                details: format!(
                    "entropy {entropy:.4} exceeded threshold {}",
                    self.config.entropy_threshold
                ),
            });
            true
        } else {
            self.consecutive_entropy_spikes = 0;
            false
        }
    }

    /// Check for repetition in output tokens. `repetition_count` is the
    /// number of times the same token appeared consecutively.
    pub fn check_repetition(&mut self, repetition_count: u32) -> bool {
        if repetition_count > self.config.max_repetition_count {
            self.consecutive_repetitions += 1;
            self.events.push(BreakerEvent {
                timestamp: SystemTime::now(),
                trigger_type: BreakerTriggerType::RepetitionLoop,
                details: format!(
                    "repetition count {repetition_count} exceeded max {}",
                    self.config.max_repetition_count
                ),
            });
            true
        } else {
            self.consecutive_repetitions = 0;
            false
        }
    }

    /// Check causal integrity. Returns `true` if a causal violation is
    /// detected (e.g. output contradicts the prompt or prior context).
    pub fn check_causal_integrity(&mut self, is_violation: bool) -> bool {
        if is_violation {
            self.consecutive_causal_violations += 1;
            self.events.push(BreakerEvent {
                timestamp: SystemTime::now(),
                trigger_type: BreakerTriggerType::CausalViolation,
                details: "causal integrity violation detected".into(),
            });
            true
        } else {
            self.consecutive_causal_violations = 0;
            false
        }
    }

    /// Should the breaker trip based on current state?
    pub fn should_trip(&self) -> bool {
        self.state == CircuitBreakerState::Closed
            && (self.consecutive_entropy_spikes >= 3
                || self.consecutive_repetitions >= 3
                || self.consecutive_causal_violations >= self.config.causal_violation_tolerance)
    }

    /// Trip the breaker, transitioning to Open.
    pub fn trip(&mut self) {
        self.state = CircuitBreakerState::Open;
    }

    /// Attempt to reset. If already Open, transitions to HalfOpen.
    /// If HalfOpen, transitions back to Closed.
    pub fn reset(&mut self) {
        self.state = match self.state {
            CircuitBreakerState::Open => CircuitBreakerState::HalfOpen,
            CircuitBreakerState::HalfOpen => CircuitBreakerState::Closed,
            CircuitBreakerState::Closed => CircuitBreakerState::Closed,
        };
        self.consecutive_entropy_spikes = 0;
        self.consecutive_repetitions = 0;
        self.consecutive_causal_violations = 0;
    }

    /// Current breaker state.
    pub fn state(&self) -> CircuitBreakerState {
        self.state
    }

    /// All recorded events.
    pub fn events(&self) -> &[BreakerEvent] {
        &self.events
    }

    /// Number of events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig {
            entropy_threshold: 0.85,
            max_repetition_count: 5,
            causal_violation_tolerance: 2,
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            entropy_threshold: 0.8,
            max_repetition_count: 3,
            causal_violation_tolerance: 2,
        }
    }

    #[test]
    fn entropy_below_threshold_no_anomaly() {
        let mut cb = CircuitBreaker::new(default_config());
        assert!(!cb.check_entropy(0.5));
        assert_eq!(cb.event_count(), 0);
    }

    #[test]
    fn entropy_above_threshold_records_event() {
        let mut cb = CircuitBreaker::new(default_config());
        assert!(cb.check_entropy(0.9));
        assert_eq!(cb.event_count(), 1);
        assert_eq!(
            cb.events()[0].trigger_type,
            BreakerTriggerType::EntropySpike
        );
    }

    #[test]
    fn repetition_above_max_records_event() {
        let mut cb = CircuitBreaker::new(default_config());
        assert!(cb.check_repetition(10));
        assert_eq!(cb.event_count(), 1);
        assert_eq!(
            cb.events()[0].trigger_type,
            BreakerTriggerType::RepetitionLoop
        );
    }

    #[test]
    fn causal_violation_records_event() {
        let mut cb = CircuitBreaker::new(default_config());
        assert!(cb.check_causal_integrity(true));
        assert_eq!(cb.event_count(), 1);
        assert_eq!(
            cb.events()[0].trigger_type,
            BreakerTriggerType::CausalViolation
        );
    }

    #[test]
    fn trip_after_consecutive_entropy_spikes() {
        let mut cb = CircuitBreaker::new(default_config());
        assert!(!cb.should_trip());
        cb.check_entropy(0.9);
        cb.check_entropy(0.9);
        assert!(!cb.should_trip()); // 2 consecutive, need 3
        cb.check_entropy(0.9);
        assert!(cb.should_trip());
        cb.trip();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    #[test]
    fn reset_transitions_through_half_open() {
        let mut cb = CircuitBreaker::new(default_config());
        cb.trip();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        cb.reset();
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
        cb.reset();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn consecutive_count_resets_on_anomaly_clear() {
        let mut cb = CircuitBreaker::new(default_config());
        cb.check_entropy(0.9);
        cb.check_entropy(0.9);
        // Normal reading resets the counter
        cb.check_entropy(0.3);
        assert!(!cb.should_trip());
    }

    #[test]
    fn causal_violation_trips_at_tolerance() {
        let mut cb = CircuitBreaker::new(default_config());
        cb.check_causal_integrity(true);
        assert!(!cb.should_trip()); // 1 violation, tolerance is 2
        cb.check_causal_integrity(true);
        assert!(cb.should_trip()); // 2 violations, reached tolerance
    }

    #[test]
    fn half_open_cannot_trip_again() {
        let mut cb = CircuitBreaker::new(default_config());
        cb.trip();
        cb.reset(); // HalfOpen
                    // Should not trip while in HalfOpen (only trips from Closed)
        assert!(!cb.should_trip());
    }
}
