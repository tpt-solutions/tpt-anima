//! Core data structures for the Intent & Negotiation Protocol.
//!
//! An [`Intent`] describes what an agent wants to achieve; a [`Capability`]
//! describes what a provider can do; [`QoSConstraints`] bound the negotiation.

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

pub type IntentId = Uuid;
pub type AgentId = Uuid;
pub type CapabilityId = Uuid;

// ---------------------------------------------------------------------------
// Intent
// ---------------------------------------------------------------------------

/// A goal descriptor broadcast by an agent seeking execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: IntentId,
    pub action: String,
    pub payload: serde_json::Value,
    pub qos: QoSConstraints,
    pub economic_terms: EconomicTerms,
    pub fallback_strategies: Vec<FallbackStrategy>,
    pub ttl: Duration,
    #[serde(with = "serde_system_time")]
    pub created_at: SystemTime,
    pub origin_agent_id: AgentId,
}

impl Intent {
    pub fn new(
        action: String,
        payload: serde_json::Value,
        qos: QoSConstraints,
        economic_terms: EconomicTerms,
        origin_agent_id: AgentId,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            action,
            payload,
            qos,
            economic_terms,
            fallback_strategies: Vec::new(),
            ttl: Duration::from_secs(30),
            created_at: SystemTime::now(),
            origin_agent_id,
        }
    }

    /// Returns `true` if the intent's TTL has expired.
    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.created_at)
            .map(|elapsed| elapsed > self.ttl)
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// A registered capability that an agent or service can fulfill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub action_types: Vec<String>,
    pub performance_profile: PerformanceProfile,
    pub pricing: EconomicTerms,
    pub availability: AvailabilityWindow,
    pub provider_agent_id: AgentId,
}

impl Capability {
    pub fn new(
        action_types: Vec<String>,
        performance_profile: PerformanceProfile,
        pricing: EconomicTerms,
        provider_agent_id: AgentId,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            action_types,
            performance_profile,
            pricing,
            availability: AvailabilityWindow::always(),
            provider_agent_id,
        }
    }

    /// Returns `true` if this capability can handle the given action.
    pub fn handles_action(&self, action: &str) -> bool {
        self.action_types.iter().any(|a| a == action)
    }
}

// ---------------------------------------------------------------------------
// QoSConstraints
// ---------------------------------------------------------------------------

/// Quality-of-Service bounds for an intent negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSConstraints {
    pub max_latency: Option<Duration>,
    pub min_accuracy: Option<f32>,
    pub max_cost: Option<f64>,
    pub required_compliance: Vec<String>,
    pub preferred_region: Option<String>,
    pub priority: Priority,
    pub deadline: Option<SystemTime>,
}

impl Default for QoSConstraints {
    fn default() -> Self {
        Self {
            max_latency: Some(Duration::from_secs(10)),
            min_accuracy: Some(0.9),
            max_cost: Some(1.0),
            required_compliance: Vec::new(),
            preferred_region: None,
            priority: Priority::Normal,
            deadline: None,
        }
    }
}

/// Priority levels for intent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Deferred = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

// ---------------------------------------------------------------------------
// EconomicTerms
// ---------------------------------------------------------------------------

/// Pricing model for capability execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EconomicTerms {
    /// Fixed price per execution.
    FixedPrice { credits: f64 },
    /// Per-unit pricing (per token, per pixel, etc.).
    PerUnit { credits_per_unit: f64, unit: String },
    /// Auction-based: providers bid their price.
    Auction { max_budget: f64 },
    /// Subscription: flat rate for a time window.
    Subscription {
        credits_per_period: f64,
        period: Duration,
    },
    /// Revenue share: percentage of outcome value.
    RevenueShare { percentage: f64 },
}

// ---------------------------------------------------------------------------
// CapabilityBid
// ---------------------------------------------------------------------------

/// A provider's bid in response to an intent broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityBid {
    pub intent_id: IntentId,
    pub provider_id: AgentId,
    pub capability_id: CapabilityId,
    pub estimated_cost: f64,
    pub estimated_latency: Duration,
    pub estimated_accuracy: f32,
    pub confidence: f32,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
}

impl CapabilityBid {
    pub fn new(
        intent_id: IntentId,
        provider_id: AgentId,
        capability_id: CapabilityId,
        estimated_cost: f64,
        estimated_latency: Duration,
        estimated_accuracy: f32,
        confidence: f32,
    ) -> Self {
        Self {
            intent_id,
            provider_id,
            capability_id,
            estimated_cost,
            estimated_latency,
            estimated_accuracy,
            confidence,
            timestamp: SystemTime::now(),
        }
    }

    /// Compute a composite score for bid ranking (lower is better).
    pub fn score(&self, constraints: &QoSConstraints) -> f64 {
        let cost_weight = 0.4f64;
        let latency_weight = 0.3f64;
        let accuracy_weight = 0.3f64;

        let cost_score = self.estimated_cost / constraints.max_cost.unwrap_or(f64::MAX).max(1.0);
        let latency_score = self.estimated_latency.as_secs_f64()
            / constraints
                .max_latency
                .unwrap_or(Duration::from_secs(60))
                .as_secs_f64()
                .max(0.1);
        let accuracy_score = 1.0 - self.estimated_accuracy as f64;

        cost_weight * cost_score + latency_weight * latency_score + accuracy_weight * accuracy_score
    }
}

// ---------------------------------------------------------------------------
// ExecutionResult
// ---------------------------------------------------------------------------

/// The outcome of executing an intent on a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub intent_id: IntentId,
    pub provider_id: AgentId,
    pub actual_cost: f64,
    pub actual_latency: Duration,
    pub actual_accuracy: f32,
    pub proofs: Vec<String>,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Completed,
    Failed { reason: String },
    PartiallyCompleted { progress: f32 },
    TimedOut,
    Cancelled,
}

// ---------------------------------------------------------------------------
// FallbackStrategy
// ---------------------------------------------------------------------------

/// Defines a fallback action when the primary execution path fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStrategy {
    pub trigger: FallbackTrigger,
    pub action: String,
    pub max_retries: u32,
}

/// Conditions that trigger a fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackTrigger {
    ProviderFailure,
    Timeout,
    SlaBreach { metric: String, threshold: f64 },
    CostOverrun,
    AccuracyBelow { threshold: f32 },
}

// ---------------------------------------------------------------------------
// PerformanceProfile & AvailabilityWindow
// ---------------------------------------------------------------------------

/// Expected performance characteristics of a capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub typical_latency: Duration,
    pub max_latency: Duration,
    pub typical_accuracy: f32,
    pub throughput_per_second: f64,
}

/// Time window when a capability is available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityWindow {
    pub available: bool,
    pub start_hour: Option<u8>,
    pub end_hour: Option<u8>,
    pub timezone: Option<String>,
}

impl AvailabilityWindow {
    pub fn always() -> Self {
        Self {
            available: true,
            start_hour: None,
            end_hour: None,
            timezone: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Serde helper — SystemTime
// ---------------------------------------------------------------------------

pub(crate) mod serde_system_time {
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> AgentId {
        Uuid::new_v4()
    }

    #[test]
    fn intent_creation_and_expiry() {
        let mut intent = Intent::new(
            "test_action".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );
        assert!(!intent.is_expired());
        intent.ttl = Duration::ZERO;
        assert!(intent.is_expired());
    }

    #[test]
    fn capability_handles_action() {
        let cap = Capability::new(
            vec!["data_transfer".to_string(), "compute".to_string()],
            PerformanceProfile {
                typical_latency: Duration::from_millis(100),
                max_latency: Duration::from_secs(5),
                typical_accuracy: 0.95,
                throughput_per_second: 100.0,
            },
            EconomicTerms::FixedPrice { credits: 0.5 },
            agent(),
        );
        assert!(cap.handles_action("data_transfer"));
        assert!(!cap.handles_action("unknown"));
    }

    #[test]
    fn bid_score_lower_is_better() {
        let bid = CapabilityBid::new(
            Uuid::new_v4(),
            agent(),
            Uuid::new_v4(),
            0.5,
            Duration::from_millis(100),
            0.95,
            0.8,
        );
        let qos = QoSConstraints::default();
        let score = bid.score(&qos);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn qos_defaults_are_sensible() {
        let qos = QoSConstraints::default();
        assert!(qos.max_latency.is_some());
        assert!(qos.min_accuracy.is_some());
        assert_eq!(qos.priority, Priority::Normal);
    }

    #[test]
    fn execution_status_variants() {
        let completed = ExecutionStatus::Completed;
        let failed = ExecutionStatus::Failed {
            reason: "timeout".to_string(),
        };
        assert_ne!(format!("{:?}", completed), format!("{:?}", failed));
    }

    #[test]
    fn fallback_trigger_variants() {
        let triggers = [
            FallbackTrigger::ProviderFailure,
            FallbackTrigger::Timeout,
            FallbackTrigger::SlaBreach {
                metric: "latency".to_string(),
                threshold: 1.0,
            },
            FallbackTrigger::CostOverrun,
            FallbackTrigger::AccuracyBelow { threshold: 0.5 },
        ];
        assert_eq!(triggers.len(), 5);
    }

    #[test]
    fn availability_window_always() {
        let aw = AvailabilityWindow::always();
        assert!(aw.available);
        assert!(aw.start_hour.is_none());
    }

    #[test]
    fn economic_terms_variants() {
        let terms = [
            EconomicTerms::FixedPrice { credits: 1.0 },
            EconomicTerms::PerUnit {
                credits_per_unit: 0.01,
                unit: "token".to_string(),
            },
            EconomicTerms::Auction { max_budget: 10.0 },
            EconomicTerms::Subscription {
                credits_per_period: 5.0,
                period: Duration::from_secs(3600),
            },
            EconomicTerms::RevenueShare { percentage: 0.1 },
        ];
        assert_eq!(terms.len(), 5);
    }
}
