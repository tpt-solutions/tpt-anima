use crate::types::*;
use std::time::SystemTime;
use uuid::Uuid;

pub struct AgencyLedger {
    entries: Vec<AgencyLedgerEntry>,
    read_only: bool,
}

impl Default for AgencyLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl AgencyLedger {
    pub fn new() -> Self {
        AgencyLedger {
            entries: Vec::new(),
            read_only: false,
        }
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    pub fn append(
        &mut self,
        event_type: LedgerEventType,
        agent_id: Uuid,
        details: String,
    ) -> Result<AgencyLedgerEntry, ZkpaError> {
        if self.read_only {
            return Err(ZkpaError::LedgerReadOnly);
        }

        let entry = AgencyLedgerEntry {
            id: Uuid::new_v4(),
            event_type,
            agent_id,
            timestamp: SystemTime::now(),
            details,
        };
        self.entries.push(entry.clone());
        Ok(entry)
    }

    pub fn query_by_agent(&self, agent_id: Uuid) -> Vec<&AgencyLedgerEntry> {
        self.entries
            .iter()
            .filter(|e| e.agent_id == agent_id)
            .collect()
    }

    pub fn query_by_event_type(&self, event_type: &LedgerEventType) -> Vec<&AgencyLedgerEntry> {
        self.entries
            .iter()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(event_type))
            .collect()
    }

    pub fn export_format(&self, format: &str) -> Result<String, ZkpaError> {
        match format {
            "json" => serde_json::to_string_pretty(&self.entries)
                .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string())),
            _ => Err(ZkpaError::ProofGenerationFailed(format!(
                "unsupported format: {}",
                format
            ))),
        }
    }

    pub fn generate_compliance_report(&self, framework: &str) -> Result<String, ZkpaError> {
        match framework {
            "SOC2" => Ok(format!(
                "SOC2 Compliance Report\nTotal events: {}\nSession events: {}\nDelegation events: {}\nAttestation events: {}",
                self.entries.len(),
                self.query_by_event_type(&LedgerEventType::SessionCreated).len()
                    + self.query_by_event_type(&LedgerEventType::SessionRevoked).len(),
                self.query_by_event_type(&LedgerEventType::DelegationGranted).len()
                    + self.query_by_event_type(&LedgerEventType::DelegationRevoked).len(),
                self.query_by_event_type(&LedgerEventType::AttestationVerified).len()
            )),
            "HIPAA" => Ok(format!(
                "HIPAA Compliance Report\nTotal events: {}\nAudit trail entries: {}\nLiveness proofs: {}",
                self.entries.len(),
                self.entries.len(),
                self.query_by_event_type(&LedgerEventType::LivenessProven).len()
            )),
            _ => Err(ZkpaError::ProofGenerationFailed(format!(
                "unsupported framework: {}",
                framework
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_entry() {
        let mut ledger = AgencyLedger::new();
        let agent = Uuid::new_v4();
        let entry = ledger
            .append(
                LedgerEventType::SessionCreated,
                agent,
                "session started".to_string(),
            )
            .unwrap();
        assert_eq!(entry.agent_id, agent);
        assert!(matches!(entry.event_type, LedgerEventType::SessionCreated));
    }

    #[test]
    fn test_query_by_agent() {
        let mut ledger = AgencyLedger::new();
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();
        ledger
            .append(LedgerEventType::SessionCreated, agent1, "a1".to_string())
            .unwrap();
        ledger
            .append(LedgerEventType::SessionRevoked, agent1, "a1r".to_string())
            .unwrap();
        ledger
            .append(LedgerEventType::SessionCreated, agent2, "a2".to_string())
            .unwrap();
        assert_eq!(ledger.query_by_agent(agent1).len(), 2);
        assert_eq!(ledger.query_by_agent(agent2).len(), 1);
    }

    #[test]
    fn test_query_by_event_type() {
        let mut ledger = AgencyLedger::new();
        let agent = Uuid::new_v4();
        ledger
            .append(LedgerEventType::SessionCreated, agent, "s1".to_string())
            .unwrap();
        ledger
            .append(LedgerEventType::DelegationGranted, agent, "d1".to_string())
            .unwrap();
        ledger
            .append(LedgerEventType::SessionCreated, agent, "s2".to_string())
            .unwrap();
        assert_eq!(
            ledger
                .query_by_event_type(&LedgerEventType::SessionCreated)
                .len(),
            2
        );
    }

    #[test]
    fn test_read_only_prevents_append() {
        let mut ledger = AgencyLedger::new();
        ledger.set_read_only(true);
        let result = ledger.append(
            LedgerEventType::SessionCreated,
            Uuid::new_v4(),
            "test".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_compliance_report_soc2() {
        let mut ledger = AgencyLedger::new();
        let agent = Uuid::new_v4();
        ledger
            .append(LedgerEventType::SessionCreated, agent, "s".to_string())
            .unwrap();
        let report = ledger.generate_compliance_report("SOC2").unwrap();
        assert!(report.contains("SOC2 Compliance Report"));
    }

    #[test]
    fn test_compliance_report_hipaa() {
        let ledger = AgencyLedger::new();
        let report = ledger.generate_compliance_report("HIPAA").unwrap();
        assert!(report.contains("HIPAA Compliance Report"));
    }

    #[test]
    fn test_export_json() {
        let mut ledger = AgencyLedger::new();
        let agent = Uuid::new_v4();
        ledger
            .append(LedgerEventType::SessionCreated, agent, "test".to_string())
            .unwrap();
        let json = ledger.export_format("json").unwrap();
        assert!(json.contains("SessionCreated"));
    }
}
