//! Compute Ledger — INP Portion.
//!
//! Tracks Compute Credit transfers between agents per negotiation and provides
//! an interface for external stablecoin/crypto rails integration.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::intent::serde_system_time;
use crate::intent::{AgentId, IntentId};

// ---------------------------------------------------------------------------
// Compute Credit
// ---------------------------------------------------------------------------

/// A Compute Credit — the native micro-economy unit of TPT Anima.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComputeCredits(pub f64);

impl ComputeCredits {
    pub fn new(amount: f64) -> Self {
        Self(amount)
    }

    pub fn zero() -> Self {
        Self(0.0)
    }

    pub fn amount(&self) -> f64 {
        self.0
    }
}

impl std::ops::Add for ComputeCredits {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for ComputeCredits {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

// ---------------------------------------------------------------------------
// Transaction
// ---------------------------------------------------------------------------

/// A single Compute Credit transfer between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub from_agent: AgentId,
    pub to_agent: AgentId,
    pub amount: ComputeCredits,
    pub intent_id: IntentId,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
    pub description: String,
}

impl Transaction {
    pub fn new(
        from_agent: AgentId,
        to_agent: AgentId,
        amount: ComputeCredits,
        intent_id: IntentId,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent,
            to_agent,
            amount,
            intent_id,
            timestamp: SystemTime::now(),
            description,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentBalance
// ---------------------------------------------------------------------------

/// Balance record for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBalance {
    pub agent_id: AgentId,
    pub balance: ComputeCredits,
    pub total_earned: ComputeCredits,
    pub total_spent: ComputeCredits,
}

impl AgentBalance {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            balance: ComputeCredits::zero(),
            total_earned: ComputeCredits::zero(),
            total_spent: ComputeCredits::zero(),
        }
    }
}

// ---------------------------------------------------------------------------
// External Rails Interface
// ---------------------------------------------------------------------------

/// Trait for external payment rail integration (stablecoin, crypto, etc.).
pub trait ExternalRails {
    /// Deposit external funds and mint Compute Credits.
    fn deposit(&self, agent_id: AgentId, amount: f64) -> Result<ComputeCredits, RailsError>;

    /// Withdraw Compute Credits and transfer to external rail.
    fn withdraw(&self, agent_id: AgentId, credits: ComputeCredits) -> Result<String, RailsError>;

    /// Get current exchange rate (external currency → Compute Credits).
    fn exchange_rate(&self, currency: &str) -> Result<f64, RailsError>;
}

/// Mock implementation for testing.
pub struct MockExternalRails;

impl ExternalRails for MockExternalRails {
    fn deposit(&self, _agent_id: AgentId, amount: f64) -> Result<ComputeCredits, RailsError> {
        Ok(ComputeCredits::new(amount))
    }

    fn withdraw(&self, _agent_id: AgentId, _credits: ComputeCredits) -> Result<String, RailsError> {
        Ok(format!("tx_{}", Uuid::new_v4()))
    }

    fn exchange_rate(&self, _currency: &str) -> Result<f64, RailsError> {
        Ok(1.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RailsError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("invalid currency: {0}")]
    InvalidCurrency(String),
    #[error("rail unavailable: {0}")]
    Unavailable(String),
}

// ---------------------------------------------------------------------------
// ComputeLedger
// ---------------------------------------------------------------------------

/// The INP Compute Ledger — tracks balances and transactions between agents.
pub struct ComputeLedger {
    balances: HashMap<AgentId, AgentBalance>,
    transactions: Vec<Transaction>,
}

impl ComputeLedger {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            transactions: Vec::new(),
        }
    }

    /// Ensure an agent has a balance entry.
    pub fn ensure_agent(&mut self, agent_id: AgentId) {
        self.balances
            .entry(agent_id)
            .or_insert_with(|| AgentBalance::new(agent_id));
    }

    /// Transfer Compute Credits between agents.
    pub fn transfer(
        &mut self,
        from: AgentId,
        to: AgentId,
        amount: ComputeCredits,
        intent_id: IntentId,
        description: String,
    ) -> Result<Transaction, LedgerError> {
        self.ensure_agent(from);
        self.ensure_agent(to);

        let from_balance = self.balances.get(&from).unwrap().balance;
        if from_balance.0 < amount.0 {
            return Err(LedgerError::InsufficientFunds {
                available: from_balance.0,
                requested: amount.0,
            });
        }

        let tx = Transaction::new(from, to, amount, intent_id, description);

        // Update balances
        let from_bal = self.balances.get_mut(&from).unwrap();
        from_bal.balance = from_bal.balance - amount;
        from_bal.total_spent = from_bal.total_spent + amount;

        let to_bal = self.balances.get_mut(&to).unwrap();
        to_bal.balance = to_bal.balance + amount;
        to_bal.total_earned = to_bal.total_earned + amount;

        self.transactions.push(tx.clone());
        Ok(tx)
    }

    /// Get an agent's current balance.
    pub fn balance_of(&self, agent_id: &AgentId) -> ComputeCredits {
        self.balances
            .get(agent_id)
            .map(|b| b.balance)
            .unwrap_or(ComputeCredits::zero())
    }

    /// Get full balance record for an agent.
    pub fn agent_balance(&self, agent_id: &AgentId) -> Option<&AgentBalance> {
        self.balances.get(agent_id)
    }

    /// Get all transactions for a specific intent.
    pub fn transactions_for_intent(&self, intent_id: &IntentId) -> Vec<&Transaction> {
        self.transactions
            .iter()
            .filter(|tx| tx.intent_id == *intent_id)
            .collect()
    }

    /// Get all transactions involving a specific agent.
    pub fn transactions_for_agent(&self, agent_id: &AgentId) -> Vec<&Transaction> {
        self.transactions
            .iter()
            .filter(|tx| tx.from_agent == *agent_id || tx.to_agent == *agent_id)
            .collect()
    }

    /// Total credits in circulation.
    pub fn total_circulation(&self) -> ComputeCredits {
        self.balances
            .values()
            .map(|b| b.balance)
            .fold(ComputeCredits::zero(), |a, b| a + b)
    }

    /// Number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.balances.len()
    }

    /// Total transactions recorded.
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}

impl Default for ComputeLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("insufficient funds: available={available}, requested={requested}")]
    InsufficientFunds { available: f64, requested: f64 },
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

    fn intent() -> IntentId {
        Uuid::new_v4()
    }

    #[test]
    fn compute_credits_arithmetic() {
        let a = ComputeCredits::new(10.0);
        let b = ComputeCredits::new(3.0);
        assert_eq!((a + b).amount(), 13.0);
        assert_eq!((a - b).amount(), 7.0);
    }

    #[test]
    fn transfer_success() {
        let mut ledger = ComputeLedger::new();
        let alice = agent();
        let bob = agent();

        // Give Alice some credits
        ledger.ensure_agent(alice);
        ledger.ensure_agent(bob);
        ledger.balances.get_mut(&alice).unwrap().balance = ComputeCredits::new(100.0);

        let tx = ledger
            .transfer(
                alice,
                bob,
                ComputeCredits::new(30.0),
                intent(),
                "test".to_string(),
            )
            .unwrap();

        assert_eq!(tx.amount.amount(), 30.0);
        assert_eq!(ledger.balance_of(&alice).amount(), 70.0);
        assert_eq!(ledger.balance_of(&bob).amount(), 30.0);
    }

    #[test]
    fn transfer_insufficient_funds() {
        let mut ledger = ComputeLedger::new();
        let alice = agent();
        let bob = agent();
        ledger.ensure_agent(alice);
        ledger.ensure_agent(bob);

        let result = ledger.transfer(
            alice,
            bob,
            ComputeCredits::new(100.0),
            intent(),
            "test".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn transactions_for_intent() {
        let mut ledger = ComputeLedger::new();
        let alice = agent();
        let bob = agent();
        let iid = intent();
        ledger.ensure_agent(alice);
        ledger.ensure_agent(bob);
        ledger.balances.get_mut(&alice).unwrap().balance = ComputeCredits::new(100.0);

        ledger
            .transfer(
                alice,
                bob,
                ComputeCredits::new(10.0),
                iid,
                "one".to_string(),
            )
            .unwrap();
        ledger
            .transfer(
                alice,
                bob,
                ComputeCredits::new(20.0),
                iid,
                "two".to_string(),
            )
            .unwrap();

        let txs = ledger.transactions_for_intent(&iid);
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn total_circulation() {
        let mut ledger = ComputeLedger::new();
        let a = agent();
        let b = agent();
        ledger.ensure_agent(a);
        ledger.ensure_agent(b);
        ledger.balances.get_mut(&a).unwrap().balance = ComputeCredits::new(50.0);
        ledger.balances.get_mut(&b).unwrap().balance = ComputeCredits::new(30.0);

        assert_eq!(ledger.total_circulation().amount(), 80.0);
    }

    #[test]
    fn mock_external_rails() {
        let rails = MockExternalRails;
        let aid = agent();
        let credits = rails.deposit(aid, 100.0).unwrap();
        assert_eq!(credits.amount(), 100.0);

        let tx_id = rails.withdraw(aid, ComputeCredits::new(50.0)).unwrap();
        assert!(tx_id.starts_with("tx_"));
    }
}
