//! INP Routing Consensus & Spam Prevention.
//!
//! Implements distributed consensus mechanisms for intent broadcasting
//! that prevent spam without requiring a central coordinator:
//! - Rate limiting per agent
//! - Proof-of-work for broadcast requests
//! - Economic stake requirements
//! - Reputation-based throttling

use crate::intent::{AgentId, Intent};
use sha2::Digest;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

// ── Rate Limiter ───────────────────────────────────────────────────────────────

/// Rate limiter for intent broadcasts per agent.
/// Prevents spam by enforcing token-bucket rate limiting.
pub struct RateLimiter {
    /// Max tokens per agent's bucket
    max_tokens: u32,
    /// Refill rate (tokens per second)
    refill_rate: f32,
    /// Token buckets per agent
    buckets: HashMap<AgentId, TokenBucket>,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f32,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u32, current_time: Instant) -> Self {
        Self {
            tokens: max_tokens as f32,
            last_refill: current_time,
        }
    }

    fn refill(&mut self, rate: f32, now: Instant) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f32();
        self.tokens = (self.tokens + elapsed * rate).min(rate * 100.0); // Cap at reasonable max
        self.last_refill = now;
    }

    fn try_consume(&mut self, cost: f32) -> bool {
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    fn current_tokens(&self) -> f32 {
        self.tokens
    }
}

impl RateLimiter {
    pub fn new(max_tokens_per_second: u32) -> Self {
        Self {
            max_tokens: max_tokens_per_second,
            refill_rate: max_tokens_per_second as f32,
            buckets: HashMap::new(),
        }
    }

    /// Check if an agent can broadcast. Returns true if allowed.
    pub fn check_broadcast(&mut self, agent_id: AgentId, cost: f32) -> bool {
        let now = Instant::now();
        let bucket = self
            .buckets
            .entry(agent_id)
            .or_insert_with(|| TokenBucket::new(self.max_tokens, now));
        bucket.refill(self.refill_rate, now);
        bucket.try_consume(cost)
    }

    /// Get current token balance for an agent.
    pub fn tokens_for(&self, agent_id: &AgentId) -> f32 {
        self.buckets
            .get(agent_id)
            .map(|b| b.current_tokens())
            .unwrap_or(0.0)
    }
}

// ── Proof of Work ────────────────────────────────────────────────────────────

/// Proof-of-work requirement for intent broadcasts.
/// Prevents spam by requiring computational work before broadcasting.
pub struct ProofOfWork {
    /// Difficulty in leading zero bits required
    difficulty: u32,
    /// Whether PoW is enabled
    enabled: bool,
}

impl ProofOfWork {
    pub fn new(difficulty: u32) -> Self {
        Self {
            difficulty,
            enabled: true,
        }
    }

    /// Create a disabled instance where any nonce passes verification.
    pub fn disabled() -> Self {
        Self {
            difficulty: 0,
            enabled: false,
        }
    }

    /// Verify a PoW submission.
    /// Returns true if the hash has sufficient leading zero bits.
    pub fn verify(&self, data: &[u8], nonce: u64) -> bool {
        if !self.enabled {
            return true;
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        hasher.update(nonce.to_le_bytes());
        let hash = hasher.finalize();

        // Count leading zero bits in the hash
        let mut zero_bits = 0u32;
        'outer: for byte in hash {
            if byte == 0 {
                zero_bits += 8;
            } else {
                for i in 0..8 {
                    if (byte >> (7 - i)) & 1 == 1 {
                        break 'outer;
                    }
                    zero_bits += 1;
                }
            }
        }

        zero_bits >= self.difficulty
    }

    /// Find a valid nonce for the given data (solving PoW).
    pub fn solve(&self, data: &[u8]) -> Option<u64> {
        if !self.enabled {
            return Some(0);
        }

        // Simple brute-force solution (in production, use more sophisticated mining)
        (0..u64::MAX).find(|&nonce| self.verify(data, nonce))
    }
}

// ── Economic Stake ───────────────────────────────────────────────────────────

/// Economic stake requirement for broadcasting intents.
#[derive(Debug, Clone)]
pub struct EconomicStake {
    /// Minimum credits required to broadcast
    min_stake: f64,
    /// Stake per broadcast (locked during processing)
    stake_per_broadcast: f64,
}

impl EconomicStake {
    pub fn new(min_stake: f64, stake_per_broadcast: f64) -> Self {
        Self {
            min_stake,
            stake_per_broadcast,
        }
    }

    /// Check if an agent has sufficient stake for broadcasting.
    pub fn check_stake(
        &self,
        agent_id: &AgentId,
        registry: &crate::registry::CapabilityRegistry,
    ) -> bool {
        // In production, this would query the agent's actual credit balance
        // For now, we assume all registered agents have sufficient stake
        registry.get(agent_id).is_some() || !registry.capabilities_of(agent_id).is_empty()
    }

    /// Get the stake required per broadcast.
    pub fn required_stake(&self) -> f64 {
        self.stake_per_broadcast
    }
}

// ── Reputation Manager ───────────────────────────────────────────────────────

/// Reputation-based throttling for intent routing.
pub struct ReputationManager {
    /// Action history per agent (for decay), used to derive reputation scores
    actions: HashMap<AgentId, VecDeque<TimestampedAction>>,
}

#[derive(Debug, Clone)]
struct TimestampedAction {
    timestamp: Instant,
    successful: bool,
}

impl ReputationManager {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    /// Record an action outcome for reputation scoring.
    pub fn record_action(&mut self, agent_id: AgentId, successful: bool) {
        let now = Instant::now();
        self.actions
            .entry(agent_id)
            .or_default()
            .push_back(TimestampedAction {
                timestamp: now,
                successful,
            });

        // Clean up old actions (older than 1 hour)
        if let Some(actions) = self.actions.get_mut(&agent_id) {
            actions.retain(|a| now.duration_since(a.timestamp) < Duration::from_secs(3600));
        }
    }

    /// Get reputation score for an agent (0.0 to 1.0).
    pub fn score(&mut self, agent_id: &AgentId) -> f32 {
        let actions = self.actions.get(agent_id).map(|a| a.len()).unwrap_or(0);
        let successful = self
            .actions
            .get(agent_id)
            .map(|a| a.iter().filter(|x| x.successful).count())
            .unwrap_or(0);

        if actions == 0 {
            1.0 // New agents start with neutral reputation
        } else {
            successful as f32 / actions as f32
        }
    }

    /// Check if an agent is throttled based on reputation.
    pub fn is_throttled(&mut self, agent_id: AgentId) -> bool {
        let score = self.score(&agent_id);
        score < 0.3 // Throttle agents with poor reputation
    }
}

impl Default for ReputationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Consensus Manager ───────────────────────────────────────────────────────

/// Unified consensus manager combining all anti-spam mechanisms.
pub struct ConsensusManager {
    rate_limiter: RateLimiter,
    proof_of_work: Option<ProofOfWork>,
    stake: EconomicStake,
    reputation: ReputationManager,
}

impl ConsensusManager {
    pub fn new(rate_limit: u32, pow_difficulty: Option<u32>, min_stake: f64) -> Self {
        Self {
            rate_limiter: RateLimiter::new(rate_limit),
            proof_of_work: pow_difficulty.map(ProofOfWork::new),
            stake: EconomicStake::new(min_stake, min_stake / 10.0),
            reputation: ReputationManager::new(),
        }
    }

    /// Validate an intent broadcast request.
    /// Returns (is_valid, error_message).
    pub fn validate_broadcast(
        &mut self,
        agent_id: AgentId,
        intent: &Intent,
        registry: &crate::registry::CapabilityRegistry,
    ) -> Result<(), String> {
        // Check reputation first
        if self.reputation.is_throttled(agent_id) {
            return Err("Agent reputation below threshold".to_string());
        }

        // Check economic stake
        if !self.stake.check_stake(&agent_id, registry) {
            return Err("Insufficient economic stake".to_string());
        }

        // Check rate limit (cost based on priority)
        let cost = match intent.qos.priority {
            crate::intent::Priority::Critical => 5.0,
            crate::intent::Priority::High => 2.0,
            crate::intent::Priority::Normal => 1.0,
            crate::intent::Priority::Low => 0.5,
            crate::intent::Priority::Deferred => 0.1,
        };

        if !self.rate_limiter.check_broadcast(agent_id, cost) {
            return Err(format!("Rate limit exceeded: {:.1} tokens required", cost));
        }

        Ok(())
    }

    /// Verify proof-of-work submission.
    pub fn verify_pow(&self, data: &[u8], nonce: u64) -> Result<(), String> {
        self.proof_of_work
            .as_ref()
            .map(|pow| {
                if pow.verify(data, nonce) {
                    Ok(())
                } else {
                    Err("Invalid proof-of-work".to_string())
                }
            })
            .unwrap_or(Ok(()))
    }

    /// Get consensus statistics.
    pub fn stats(&self) -> ConsensusStats {
        ConsensusStats {
            rate_limiter_max_tokens: self.rate_limiter.max_tokens,
            pow_enabled: self.proof_of_work.is_some(),
            min_stake: self.stake.min_stake,
        }
    }
}

impl Default for ConsensusManager {
    fn default() -> Self {
        Self::new(100, Some(16), 10.0) // 100 broadcasts/sec, 16-bit PoW, 10 credit minimum
    }
}

// ── Statistics ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConsensusStats {
    pub rate_limiter_max_tokens: u32,
    pub pow_enabled: bool,
    pub min_stake: f64,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn agent() -> AgentId {
        Uuid::new_v4()
    }

    #[test]
    fn rate_limiter_allows_initial_broadcast() {
        let mut limiter = RateLimiter::new(10);
        let agent_id = agent();
        assert!(limiter.check_broadcast(agent_id, 1.0));
    }

    #[test]
    fn rate_limiter_blocks_excessive_broadcasts() {
        let mut limiter = RateLimiter::new(5);
        let agent_id = agent();

        // Should allow 5 broadcasts
        for _ in 0..5 {
            assert!(limiter.check_broadcast(agent_id, 1.0));
        }

        // Should block the 6th
        assert!(!limiter.check_broadcast(agent_id, 1.0));
    }

    #[test]
    fn pow_verify_difficulty() {
        let pow = ProofOfWork::new(4); // 4 bits difficulty
        let data = b"test_intent_data";

        // Find a valid nonce
        if let Some(nonce) = pow.solve(data) {
            assert!(pow.verify(data, nonce));
        }
    }

    #[test]
    fn pow_disabled_always_passes() {
        let pow = ProofOfWork::disabled();
        // When disabled, any nonce should pass
        assert!(pow.verify(b"test_intent_data", 0));
        assert_eq!(pow.solve(b"test_intent_data"), Some(0));
    }

    #[test]
    fn reputation_record_and_score() {
        let mut rep = ReputationManager::new();
        let agent_id = agent();

        // Record successful actions
        for _ in 0..4 {
            rep.record_action(agent_id, true);
        }
        // Record one failed action
        rep.record_action(agent_id, false);

        let score = rep.score(&agent_id);
        assert!((score - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn reputation_new_agent_score() {
        let mut rep = ReputationManager::new();
        let agent_id = agent();
        assert_eq!(rep.score(&agent_id), 1.0);
    }

    #[test]
    fn consensus_manager_default() {
        let cm = ConsensusManager::default();
        let stats = cm.stats();
        assert_eq!(stats.rate_limiter_max_tokens, 100);
        assert!(stats.pow_enabled);
    }
}
