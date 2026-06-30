use crate::types::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

pub struct LivenessMonitor {
    chains: HashMap<Uuid, Vec<LivenessHeartbeat>>,
}

impl Default for LivenessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl LivenessMonitor {
    pub fn new() -> Self {
        LivenessMonitor {
            chains: HashMap::new(),
        }
    }

    pub fn emit_heartbeat(
        &mut self,
        agent_id: Uuid,
        proof: Option<ZKProof>,
    ) -> Result<LivenessHeartbeat, ZkpaError> {
        let chain = self.chains.entry(agent_id).or_default();
        let sequence_num = chain.len() as u64;

        let hash_chain_link = if let Some(last) = chain.last() {
            let mut hasher = Sha256::new();
            hasher.update(last.hash_chain_link);
            hasher.update(sequence_num.to_le_bytes());
            hasher.finalize().into()
        } else {
            let mut hasher = Sha256::new();
            hasher.update(agent_id.as_bytes());
            hasher.update(sequence_num.to_le_bytes());
            hasher.finalize().into()
        };

        let heartbeat = LivenessHeartbeat {
            agent_id,
            sequence_num,
            hash_chain_link,
            proof,
            timestamp: SystemTime::now(),
        };

        chain.push(heartbeat.clone());
        Ok(heartbeat)
    }

    pub fn validate_chain(&self, agent_id: Uuid) -> Result<(), ZkpaError> {
        let chain = self.chains.get(&agent_id).ok_or(ZkpaError::LivenessGap)?;
        if chain.is_empty() {
            return Err(ZkpaError::LivenessGap);
        }

        for window in chain.windows(2) {
            let prev = &window[0];
            let curr = &window[1];

            let mut hasher = Sha256::new();
            hasher.update(prev.hash_chain_link);
            hasher.update(curr.sequence_num.to_le_bytes());
            let expected: [u8; 32] = hasher.finalize().into();

            if curr.hash_chain_link != expected {
                return Err(ZkpaError::LivenessGap);
            }
        }
        Ok(())
    }

    pub fn detect_gaps(&self, agent_id: Uuid, expected_count: u64) -> Result<Vec<u64>, ZkpaError> {
        let chain = self.chains.get(&agent_id).ok_or(ZkpaError::LivenessGap)?;
        let actual_count = chain.len() as u64;
        if actual_count < expected_count {
            let gaps: Vec<u64> = (actual_count..expected_count).collect();
            Ok(gaps)
        } else {
            Ok(vec![])
        }
    }

    pub fn chain_length(&self, agent_id: Uuid) -> usize {
        self.chains.get(&agent_id).map(|c| c.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_heartbeat() {
        let mut monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        let hb = monitor.emit_heartbeat(agent, None).unwrap();
        assert_eq!(hb.sequence_num, 0);
        assert_eq!(hb.agent_id, agent);
    }

    #[test]
    fn test_validate_chain_success() {
        let mut monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        monitor.emit_heartbeat(agent, None).unwrap();
        monitor.emit_heartbeat(agent, None).unwrap();
        monitor.emit_heartbeat(agent, None).unwrap();
        assert!(monitor.validate_chain(agent).is_ok());
    }

    #[test]
    fn test_validate_empty_chain_fails() {
        let monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        assert!(monitor.validate_chain(agent).is_err());
    }

    #[test]
    fn test_detect_gaps() {
        let mut monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        monitor.emit_heartbeat(agent, None).unwrap();
        monitor.emit_heartbeat(agent, None).unwrap();
        let gaps = monitor.detect_gaps(agent, 5).unwrap();
        assert_eq!(gaps, vec![2, 3, 4]);
    }

    #[test]
    fn test_chain_length() {
        let mut monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        assert_eq!(monitor.chain_length(agent), 0);
        monitor.emit_heartbeat(agent, None).unwrap();
        monitor.emit_heartbeat(agent, None).unwrap();
        assert_eq!(monitor.chain_length(agent), 2);
    }

    #[test]
    fn test_hash_chain_links() {
        let mut monitor = LivenessMonitor::new();
        let agent = Uuid::new_v4();
        let hb0 = monitor.emit_heartbeat(agent, None).unwrap();
        let hb1 = monitor.emit_heartbeat(agent, None).unwrap();
        // Each heartbeat should have a different hash
        assert_ne!(hb0.hash_chain_link, hb1.hash_chain_link);
    }
}
