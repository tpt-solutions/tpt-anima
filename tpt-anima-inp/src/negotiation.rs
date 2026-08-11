//! Negotiation State Machine for the Intent & Negotiation Protocol.
//!
//! Routes intents through Broadcasting → CollectingBids → Executing →
//! Completed/Failed with automatic fallback chain support.

use std::time::{Duration, Instant};

use crate::broadcast::{BroadcastConfig, IntentBroadcaster};
use crate::intent::{
    AgentId, CapabilityBid, ExecutionResult, ExecutionStatus, FallbackStrategy, Intent, IntentId,
};
use crate::registry::CapabilityRegistry;

// ---------------------------------------------------------------------------
// NegotiationState
// ---------------------------------------------------------------------------

/// Current state of a negotiation session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationState {
    /// Intent created, not yet broadcast.
    Created,
    /// Broadcasting to matching providers.
    Broadcasting,
    /// Collecting bids from providers.
    CollectingBids,
    /// Executing the winning capability.
    Executing { provider: AgentId },
    /// Successfully completed.
    Completed,
    /// All attempts failed.
    Failed { reason: String },
    /// Execution was cancelled.
    Cancelled,
    /// Triggering a fallback strategy.
    FallbackTriggered { strategy_index: usize },
}

// ---------------------------------------------------------------------------
// NegotiationSession
// ---------------------------------------------------------------------------

/// Manages the lifecycle of a single intent negotiation.
pub struct NegotiationSession {
    pub intent: Intent,
    pub state: NegotiationState,
    pub bids: Vec<CapabilityBid>,
    pub result: Option<ExecutionResult>,
    pub fallback_index: usize,
    created_at: Instant,
    state_changed_at: Instant,
}

impl NegotiationSession {
    pub fn new(intent: Intent) -> Self {
        let now = Instant::now();
        Self {
            intent,
            state: NegotiationState::Created,
            bids: Vec::new(),
            result: None,
            fallback_index: 0,
            created_at: now,
            state_changed_at: now,
        }
    }

    /// Transition to the Broadcasting state.
    pub fn start_broadcasting(&mut self) {
        self.state = NegotiationState::Broadcasting;
        self.state_changed_at = Instant::now();
    }

    /// Transition to CollectingBids state.
    pub fn start_collecting(&mut self) {
        self.state = NegotiationState::CollectingBids;
        self.state_changed_at = Instant::now();
    }

    /// Add a bid to the session.
    pub fn add_bid(&mut self, bid: CapabilityBid) {
        self.bids.push(bid);
    }

    /// Transition to Executing state with the winning provider.
    pub fn start_executing(&mut self, provider: AgentId) {
        self.state = NegotiationState::Executing { provider };
        self.state_changed_at = Instant::now();
    }

    /// Mark negotiation as completed.
    pub fn complete(&mut self, result: ExecutionResult) {
        self.result = Some(result);
        self.state = NegotiationState::Completed;
        self.state_changed_at = Instant::now();
    }

    /// Mark negotiation as failed.
    pub fn fail(&mut self, reason: String) {
        self.state = NegotiationState::Failed { reason };
        self.state_changed_at = Instant::now();
    }

    /// Cancel the negotiation.
    pub fn cancel(&mut self) {
        self.state = NegotiationState::Cancelled;
        self.state_changed_at = Instant::now();
    }

    /// Check if a fallback should be triggered based on the result.
    pub fn check_fallback(&self) -> Option<&FallbackStrategy> {
        if let Some(ref result) = self.result {
            match result.status {
                ExecutionStatus::Failed { .. } | ExecutionStatus::TimedOut => {
                    self.intent.fallback_strategies.get(self.fallback_index)
                }
                _ => None,
            }
        } else if self.state == NegotiationState::CollectingBids && self.bids.is_empty() {
            // No bids received — trigger fallback
            self.intent.fallback_strategies.get(self.fallback_index)
        } else {
            None
        }
    }

    /// Trigger the next fallback strategy.
    pub fn trigger_fallback(&mut self) -> Option<&FallbackStrategy> {
        let strategy = self.intent.fallback_strategies.get(self.fallback_index)?;
        self.state = NegotiationState::FallbackTriggered {
            strategy_index: self.fallback_index,
        };
        self.state_changed_at = Instant::now();
        self.fallback_index += 1;
        Some(strategy)
    }

    /// Check if the session has exceeded its TTL.
    pub fn is_expired(&self) -> bool {
        self.intent.is_expired()
    }

    /// Elapsed time since session creation.
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Elapsed time since last state change.
    pub fn since_state_change(&self) -> Duration {
        self.state_changed_at.elapsed()
    }
}

// ---------------------------------------------------------------------------
// NegotiationManager
// ---------------------------------------------------------------------------

/// Manages multiple concurrent negotiation sessions.
pub struct NegotiationManager {
    sessions: std::collections::HashMap<IntentId, NegotiationSession>,
    broadcaster: IntentBroadcaster,
}

impl NegotiationManager {
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
            broadcaster: IntentBroadcaster::new(config),
        }
    }

    /// Start a new negotiation for an intent.
    pub fn start_negotiation(&mut self, intent: Intent, registry: &CapabilityRegistry) -> IntentId {
        let intent_id = intent.id;
        let mut session = NegotiationSession::new(intent);
        session.start_broadcasting();

        let providers = self.broadcaster.begin_broadcast(&session.intent, registry);

        if providers.is_empty() {
            // No matching providers — try fallback immediately
            session.fail("no matching providers".to_string());
        } else {
            session.start_collecting();
        }

        self.sessions.insert(intent_id, session);
        intent_id
    }

    /// Submit a bid for an active negotiation.
    pub fn submit_bid(
        &mut self,
        bid: CapabilityBid,
    ) -> Result<(), crate::broadcast::BroadcastError> {
        self.broadcaster.submit_bid(bid.clone())?;

        if let Some(session) = self.sessions.get_mut(&bid.intent_id) {
            session.add_bid(bid);
        }

        Ok(())
    }

    /// Finalize a negotiation round and return the winner.
    pub fn finalize_and_select(
        &mut self,
        intent_id: &IntentId,
    ) -> Option<(IntentId, Option<AgentId>)> {
        let broadcast_result = self.broadcaster.finalize_broadcast(intent_id)?;

        let session = self.sessions.get(intent_id)?;
        let qos = &session.intent.qos;

        let winner = broadcast_result.select_winner(qos).map(|b| b.provider_id);

        Some((broadcast_result.intent_id, winner))
    }

    /// Get a session by intent ID.
    pub fn get_session(&self, intent_id: &IntentId) -> Option<&NegotiationSession> {
        self.sessions.get(intent_id)
    }

    /// Get a mutable session by intent ID.
    pub fn get_session_mut(&mut self, intent_id: &IntentId) -> Option<&mut NegotiationSession> {
        self.sessions.get_mut(intent_id)
    }

    /// Remove a completed or failed session.
    pub fn remove_session(&mut self, intent_id: &IntentId) -> Option<NegotiationSession> {
        self.sessions.remove(intent_id)
    }

    /// Number of active sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for NegotiationManager {
    fn default() -> Self {
        Self::new(BroadcastConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{EconomicTerms, FallbackTrigger, PerformanceProfile, QoSConstraints};
    use std::time::Duration;
    use uuid::Uuid;

    fn agent() -> AgentId {
        Uuid::new_v4()
    }

    #[test]
    fn session_lifecycle() {
        let intent = Intent::new(
            "test".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );

        let mut session = NegotiationSession::new(intent);
        assert_eq!(session.state, NegotiationState::Created);

        session.start_broadcasting();
        assert_eq!(session.state, NegotiationState::Broadcasting);

        session.start_collecting();
        assert_eq!(session.state, NegotiationState::CollectingBids);

        let provider = agent();
        session.start_executing(provider);
        assert_eq!(session.state, NegotiationState::Executing { provider });

        let result = ExecutionResult {
            intent_id: session.intent.id,
            provider_id: provider,
            actual_cost: 0.5,
            actual_latency: Duration::from_millis(100),
            actual_accuracy: 0.95,
            proofs: vec![],
            status: ExecutionStatus::Completed,
        };
        session.complete(result);
        assert_eq!(session.state, NegotiationState::Completed);
    }

    #[test]
    fn session_fail_and_fallback() {
        let mut intent = Intent::new(
            "test".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );
        intent.fallback_strategies.push(FallbackStrategy {
            trigger: FallbackTrigger::ProviderFailure,
            action: "retry_with_backup".to_string(),
            max_retries: 3,
        });

        let mut session = NegotiationSession::new(intent);
        session.start_broadcasting();
        session.start_collecting();

        let result = ExecutionResult {
            intent_id: session.intent.id,
            provider_id: agent(),
            actual_cost: 0.0,
            actual_latency: Duration::from_secs(5),
            actual_accuracy: 0.0,
            proofs: vec![],
            status: ExecutionStatus::Failed {
                reason: "crashed".to_string(),
            },
        };
        session.complete(result);

        let fallback = session.check_fallback();
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().action, "retry_with_backup");
    }

    #[test]
    fn session_cancel() {
        let intent = Intent::new(
            "test".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );

        let mut session = NegotiationSession::new(intent);
        session.cancel();
        assert_eq!(session.state, NegotiationState::Cancelled);
    }

    #[test]
    fn manager_start_and_submit() {
        let mut registry = CapabilityRegistry::new();
        let aid = agent();
        registry.register(crate::intent::Capability::new(
            vec!["compute".to_string()],
            PerformanceProfile {
                typical_latency: Duration::from_millis(100),
                max_latency: Duration::from_secs(5),
                typical_accuracy: 0.95,
                throughput_per_second: 100.0,
            },
            EconomicTerms::FixedPrice { credits: 0.5 },
            aid,
        ));

        let mut manager = NegotiationManager::default();
        let intent = Intent::new(
            "compute".to_string(),
            serde_json::json!({}),
            QoSConstraints::default(),
            EconomicTerms::FixedPrice { credits: 1.0 },
            agent(),
        );

        let intent_id = manager.start_negotiation(intent, &registry);
        assert!(manager.get_session(&intent_id).is_some());

        let session = manager.get_session(&intent_id).unwrap();
        assert_eq!(session.state, NegotiationState::CollectingBids);
    }
}
