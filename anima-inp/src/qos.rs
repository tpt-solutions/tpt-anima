//! QoS & Compute Trading for the Intent & Negotiation Protocol.
//!
//! Implements latency/accuracy/cost trade negotiation and mid-stream QoS
//! adjustment (trading precision for speed).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::intent::{ExecutionResult, QoSConstraints};

// ---------------------------------------------------------------------------
// QoSProfile
// ---------------------------------------------------------------------------

/// A QoS profile representing a specific trade-off configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSProfile {
    pub name: String,
    pub latency_weight: f32,
    pub accuracy_weight: f32,
    pub cost_weight: f32,
}

impl QoSProfile {
    /// Balanced: equal weight to all factors.
    pub fn balanced() -> Self {
        Self {
            name: "balanced".to_string(),
            latency_weight: 0.33,
            accuracy_weight: 0.33,
            cost_weight: 0.34,
        }
    }

    /// Speed-first: prioritizes low latency.
    pub fn speed_first() -> Self {
        Self {
            name: "speed_first".to_string(),
            latency_weight: 0.6,
            accuracy_weight: 0.2,
            cost_weight: 0.2,
        }
    }

    /// Quality-first: prioritizes accuracy.
    pub fn quality_first() -> Self {
        Self {
            name: "quality_first".to_string(),
            latency_weight: 0.2,
            accuracy_weight: 0.6,
            cost_weight: 0.2,
        }
    }

    /// Economy-first: prioritizes low cost.
    pub fn economy_first() -> Self {
        Self {
            name: "economy_first".to_string(),
            latency_weight: 0.2,
            accuracy_weight: 0.2,
            cost_weight: 0.6,
        }
    }
}

// ---------------------------------------------------------------------------
// QoSNegotiator
// ---------------------------------------------------------------------------

/// Negotiates QoS adjustments during execution.
pub struct QoSNegotiator {
    current_profile: QoSProfile,
    adjustment_history: Vec<QoSAdjustment>,
}

impl QoSNegotiator {
    pub fn new(profile: QoSProfile) -> Self {
        Self {
            current_profile: profile,
            adjustment_history: Vec::new(),
        }
    }

    /// Adjust QoS profile mid-stream (e.g., trade precision for speed).
    pub fn adjust(&mut self, new_profile: QoSProfile, reason: String) {
        let adjustment = QoSAdjustment {
            from: self.current_profile.clone(),
            to: new_profile.clone(),
            reason,
        };
        self.adjustment_history.push(adjustment);
        self.current_profile = new_profile;
    }

    /// Compute a composite score for an execution result using the current profile.
    pub fn score_result(&self, result: &ExecutionResult, constraints: &QoSConstraints) -> f64 {
        let latency_score = self.normalize_latency(result.actual_latency, constraints);
        let accuracy_score = result.actual_accuracy as f64;
        let cost_score = self.normalize_cost(result.actual_cost, constraints);

        let p = &self.current_profile;
        (p.latency_weight as f64 * latency_score)
            + (p.accuracy_weight as f64 * accuracy_score)
            + (p.cost_weight as f64 * cost_score)
    }

    fn normalize_latency(&self, latency: Duration, constraints: &QoSConstraints) -> f64 {
        let max = constraints
            .max_latency
            .unwrap_or(Duration::from_secs(60))
            .as_secs_f64();
        if max == 0.0 {
            0.0
        } else {
            1.0 - (latency.as_secs_f64() / max).min(1.0)
        }
    }

    fn normalize_cost(&self, cost: f64, constraints: &QoSConstraints) -> f64 {
        let max = constraints.max_cost.unwrap_or(f64::MAX);
        if max == 0.0 || max == f64::MAX {
            1.0
        } else {
            1.0 - (cost / max).min(1.0)
        }
    }

    /// Get the current profile.
    pub fn current_profile(&self) -> &QoSProfile {
        &self.current_profile
    }

    /// Get adjustment history.
    pub fn history(&self) -> &[QoSAdjustment] {
        &self.adjustment_history
    }
}

// ---------------------------------------------------------------------------
// QoSAdjustment
// ---------------------------------------------------------------------------

/// Record of a mid-stream QoS adjustment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSAdjustment {
    pub from: QoSProfile,
    pub to: QoSProfile,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// QoSRecommendation
// ---------------------------------------------------------------------------

/// Recommends a QoS profile based on observed conditions.
pub struct QoSRecommender;

impl QoSRecommender {
    /// Recommend a profile based on current constraints and observed performance.
    pub fn recommend(
        constraints: &QoSConstraints,
        recent_latency: Duration,
        recent_accuracy: f32,
    ) -> QoSProfile {
        let latency_pressure =
            recent_latency > constraints.max_latency.unwrap_or(Duration::from_secs(60));

        let accuracy_pressure = constraints
            .min_accuracy
            .is_some_and(|min| recent_accuracy < min);

        if latency_pressure && accuracy_pressure {
            // Both under pressure — go balanced
            QoSProfile::balanced()
        } else if latency_pressure {
            // Latency is the bottleneck — trade accuracy for speed
            QoSProfile::speed_first()
        } else if accuracy_pressure {
            // Accuracy is the bottleneck — trade speed for quality
            QoSProfile::quality_first()
        } else {
            // No pressure — can be more economical
            QoSProfile::economy_first()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::ExecutionStatus;

    fn make_result(latency_ms: u64, accuracy: f32, cost: f64) -> ExecutionResult {
        ExecutionResult {
            intent_id: uuid::Uuid::new_v4(),
            provider_id: uuid::Uuid::new_v4(),
            actual_cost: cost,
            actual_latency: Duration::from_millis(latency_ms),
            actual_accuracy: accuracy,
            proofs: vec![],
            status: ExecutionStatus::Completed,
        }
    }

    #[test]
    fn qos_profiles_are_distinct() {
        let balanced = QoSProfile::balanced();
        let speed = QoSProfile::speed_first();
        let quality = QoSProfile::quality_first();
        let economy = QoSProfile::economy_first();

        assert!(speed.latency_weight > balanced.latency_weight);
        assert!(quality.accuracy_weight > balanced.accuracy_weight);
        assert!(economy.cost_weight > balanced.cost_weight);
    }

    #[test]
    fn negotiator_adjustment() {
        let mut negotiator = QoSNegotiator::new(QoSProfile::balanced());
        assert_eq!(negotiator.current_profile().name, "balanced");

        negotiator.adjust(QoSProfile::speed_first(), "latency pressure".to_string());
        assert_eq!(negotiator.current_profile().name, "speed_first");
        assert_eq!(negotiator.history().len(), 1);
    }

    #[test]
    fn score_result_with_balanced_profile() {
        let negotiator = QoSNegotiator::new(QoSProfile::balanced());
        let constraints = QoSConstraints::default();
        let result = make_result(100, 0.95, 0.5);

        let score = negotiator.score_result(&result, &constraints);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn recommender_speed_first_for_latency_pressure() {
        let constraints = QoSConstraints {
            max_latency: Some(Duration::from_millis(50)),
            ..Default::default()
        };
        let profile = QoSRecommender::recommend(&constraints, Duration::from_millis(200), 0.99);
        assert_eq!(profile.name, "speed_first");
    }

    #[test]
    fn recommender_quality_first_for_accuracy_pressure() {
        let constraints = QoSConstraints {
            min_accuracy: Some(0.99),
            ..Default::default()
        };
        let profile = QoSRecommender::recommend(&constraints, Duration::from_millis(10), 0.8);
        assert_eq!(profile.name, "quality_first");
    }

    #[test]
    fn recommender_economy_first_when_no_pressure() {
        let constraints = QoSConstraints {
            max_latency: Some(Duration::from_secs(10)),
            min_accuracy: Some(0.5),
            ..Default::default()
        };
        let profile = QoSRecommender::recommend(&constraints, Duration::from_millis(100), 0.9);
        assert_eq!(profile.name, "economy_first");
    }
}
