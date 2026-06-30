//! Intent Broadcasting & Bid Collection.
//!
//! Broadcasts intents to matching capability providers, collects bids within
//! a configurable timeout, and selects the optimal provider.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::intent::{CapabilityBid, Intent, IntentId, QoSConstraints};
use crate::registry::CapabilityRegistry;

// ---------------------------------------------------------------------------
// BroadcastConfig
// ---------------------------------------------------------------------------

/// Configuration for a broadcast round.
#[derive(Debug, Clone)]
pub struct BroadcastConfig {
    /// Maximum time to wait for bids after broadcasting.
    pub bid_timeout: Duration,
    /// Maximum number of bids to collect before evaluating.
    pub max_bids: usize,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            bid_timeout: Duration::from_secs(5),
            max_bids: 100,
        }
    }
}

// ---------------------------------------------------------------------------
// BroadcastResult
// ---------------------------------------------------------------------------

/// The result of a broadcast + bid collection round.
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    pub intent_id: IntentId,
    pub bids_received: Vec<CapabilityBid>,
    pub elapsed: Duration,
    pub timed_out: bool,
}

impl BroadcastResult {
    /// Select the winning bid based on QoS scoring.
    ///
    /// Returns `None` if no bids were received.
    pub fn select_winner(&self, qos: &QoSConstraints) -> Option<&CapabilityBid> {
        self.bids_received.iter().min_by(|a, b| {
            a.score(qos)
                .partial_cmp(&b.score(qos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Select top N bids by QoS score.
    pub fn top_bids(&self, qos: &QoSConstraints, n: usize) -> Vec<&CapabilityBid> {
        let mut sorted: Vec<&CapabilityBid> = self.bids_received.iter().collect();
        sorted.sort_by(|a, b| {
            a.score(qos)
                .partial_cmp(&b.score(qos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(n).collect()
    }
}

// ---------------------------------------------------------------------------
// IntentBroadcaster
// ---------------------------------------------------------------------------

/// Coordinates broadcasting intents and collecting bids.
pub struct IntentBroadcaster {
    config: BroadcastConfig,
    /// Active bid collection windows: intent_id → (start_time, collected_bids).
    active_rounds: HashMap<IntentId, BidCollectionRound>,
}

struct BidCollectionRound {
    start: Instant,
    bids: Vec<CapabilityBid>,
}

impl IntentBroadcaster {
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            config,
            active_rounds: HashMap::new(),
        }
    }

    /// Begin a new broadcast round for an intent.
    ///
    /// Returns the list of matching capability provider IDs that should
    /// receive the broadcast.
    pub fn begin_broadcast(
        &mut self,
        intent: &Intent,
        registry: &CapabilityRegistry,
    ) -> Vec<crate::intent::AgentId> {
        let matching = registry.lookup_by_action(&intent.action);

        let provider_ids: Vec<_> = matching
            .iter()
            .filter(|c| c.availability.available)
            .map(|c| c.provider_agent_id)
            .collect();

        self.active_rounds.insert(
            intent.id,
            BidCollectionRound {
                start: Instant::now(),
                bids: Vec::new(),
            },
        );

        provider_ids
    }

    /// Submit a bid for an active broadcast round.
    ///
    /// Returns `Err` if the intent ID is not active or the bid timeout has passed.
    pub fn submit_bid(&mut self, bid: CapabilityBid) -> Result<(), BroadcastError> {
        let round =
            self.active_rounds
                .get_mut(&bid.intent_id)
                .ok_or(BroadcastError::UnknownIntent {
                    intent_id: bid.intent_id,
                })?;

        if round.start.elapsed() > self.config.bid_timeout {
            return Err(BroadcastError::BidTimeout {
                intent_id: bid.intent_id,
            });
        }

        if round.bids.len() >= self.config.max_bids {
            return Err(BroadcastError::BidLimitReached {
                intent_id: bid.intent_id,
            });
        }

        round.bids.push(bid);
        Ok(())
    }

    /// Finalize a broadcast round and return the result.
    ///
    /// Collects all bids received so far, even if the timeout hasn't been reached.
    pub fn finalize_broadcast(&mut self, intent_id: &IntentId) -> Option<BroadcastResult> {
        let round = self.active_rounds.remove(intent_id)?;

        let timed_out = round.start.elapsed() > self.config.bid_timeout;
        let elapsed = round.start.elapsed();

        Some(BroadcastResult {
            intent_id: *intent_id,
            bids_received: round.bids,
            elapsed,
            timed_out,
        })
    }

    /// Check if a broadcast round is still active (timeout not reached).
    pub fn is_round_active(&self, intent_id: &IntentId) -> bool {
        self.active_rounds
            .get(intent_id)
            .is_some_and(|round| round.start.elapsed() <= self.config.bid_timeout)
    }

    /// Number of active broadcast rounds.
    pub fn active_round_count(&self) -> usize {
        self.active_rounds.len()
    }
}

impl Default for IntentBroadcaster {
    fn default() -> Self {
        Self::new(BroadcastConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("unknown intent {intent_id}")]
    UnknownIntent { intent_id: IntentId },

    #[error("bid timeout for intent {intent_id}")]
    BidTimeout { intent_id: IntentId },

    #[error("bid limit reached for intent {intent_id}")]
    BidLimitReached { intent_id: IntentId },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AgentId, Capability, EconomicTerms, PerformanceProfile, QoSConstraints};
    use std::time::Duration;
    use uuid::Uuid;

    fn agent() -> AgentId {
        Uuid::new_v4()
    }

    fn make_registry_with_provider(action: &str) -> (CapabilityRegistry, AgentId) {
        let mut registry = CapabilityRegistry::new();
        let aid = agent();
        let cap = Capability::new(
            vec![action.to_string()],
            PerformanceProfile {
                typical_latency: Duration::from_millis(100),
                max_latency: Duration::from_secs(5),
                typical_accuracy: 0.95,
                throughput_per_second: 100.0,
            },
            EconomicTerms::FixedPrice { credits: 0.5 },
            aid,
        );
        registry.register(cap);
        (registry, aid)
    }

    #[test]
    fn broadcast_finds_providers() {
        let (registry, aid) = make_registry_with_provider("compute");
        let mut broadcaster = IntentBroadcaster::default();

        let intent = Intent::new(
            "compute".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );

        let providers = broadcaster.begin_broadcast(&intent, &registry);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0], aid);
    }

    #[test]
    fn submit_and_finalize_bid() {
        let (registry, aid) = make_registry_with_provider("compute");
        let mut broadcaster = IntentBroadcaster::default();

        let intent = Intent::new(
            "compute".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );

        let providers = broadcaster.begin_broadcast(&intent, &registry);
        assert_eq!(providers.len(), 1);

        let bid = CapabilityBid::new(
            intent.id,
            aid,
            Uuid::new_v4(),
            0.5,
            Duration::from_millis(200),
            0.95,
            0.9,
        );

        broadcaster.submit_bid(bid).unwrap();

        let result = broadcaster.finalize_broadcast(&intent.id).unwrap();
        assert_eq!(result.bids_received.len(), 1);
        assert!(!result.timed_out);
    }

    #[test]
    fn select_winner_returns_best_bid() {
        let qos = QoSConstraints::default();
        let intent_id = Uuid::new_v4();

        let bid_good = CapabilityBid::new(
            intent_id,
            agent(),
            Uuid::new_v4(),
            0.3,
            Duration::from_millis(50),
            0.99,
            0.95,
        );
        let bid_bad = CapabilityBid::new(
            intent_id,
            agent(),
            Uuid::new_v4(),
            0.9,
            Duration::from_millis(500),
            0.8,
            0.5,
        );

        let result = BroadcastResult {
            intent_id,
            bids_received: vec![bid_bad, bid_good.clone()],
            elapsed: Duration::from_millis(10),
            timed_out: false,
        };

        let winner = result.select_winner(&qos).unwrap();
        assert_eq!(winner.provider_id, bid_good.provider_id);
    }

    #[test]
    fn unknown_intent_returns_error() {
        let mut broadcaster = IntentBroadcaster::default();
        let bid = CapabilityBid::new(
            Uuid::new_v4(), // not registered
            agent(),
            Uuid::new_v4(),
            0.5,
            Duration::from_millis(100),
            0.9,
            0.8,
        );
        assert!(broadcaster.submit_bid(bid).is_err());
    }
}
