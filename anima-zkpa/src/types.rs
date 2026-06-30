use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

pub(crate) mod serde_system_time {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        let secs = duration.as_secs();
        let nanos = duration.subsec_nanos();
        (secs, nanos).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        let (secs, nanos) = <(u64, u32)>::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs) + Duration::from_nanos(nanos as u64))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttestationPlatform {
    TPM,
    TrustZone,
    SGX,
    AMDSecure,
    NVIDIACC,
    AppleSE,
    AWSNitro,
    AzureConfidential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAttestation {
    pub platform: AttestationPlatform,
    pub attestation_data: Vec<u8>,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCredential {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub task_scope: String,
    pub compute_budget: f64,
    #[serde(with = "serde_system_time")]
    pub expires_at: SystemTime,
    pub hardware_binding: Option<HardwareAttestation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityLevel {
    Untrusted = 0,
    Standard = 1,
    Sandboxed = 2,
    HardwareEnclave = 3,
    ConfidentialCompute = 4,
    FullyVerified = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofSystem {
    Groth16,
    PLONK,
    Halo2,
    STARKs,
    Nova,
    Bulletproofs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKCircuit {
    pub circuit_type: String,
    pub proof_system: ProofSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZKProof {
    pub proof_type: String,
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub verification_key: Vec<u8>,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityDelegation {
    pub parent_id: Uuid,
    pub child_id: Uuid,
    pub scoped_permissions: Vec<String>,
    pub zk_proof: Option<ZKProof>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessHeartbeat {
    pub agent_id: Uuid,
    pub sequence_num: u64,
    pub hash_chain_link: [u8; 32],
    pub proof: Option<ZKProof>,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LedgerEventType {
    SessionCreated,
    SessionRevoked,
    DelegationGranted,
    DelegationRevoked,
    AttestationVerified,
    LivenessProven,
    ProofGenerated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyLedgerEntry {
    pub id: Uuid,
    pub event_type: LedgerEventType,
    pub agent_id: Uuid,
    #[serde(with = "serde_system_time")]
    pub timestamp: SystemTime,
    pub details: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ZkpaError {
    #[error("session expired")]
    SessionExpired,
    #[error("attestation failed: {0}")]
    AttestationFailed(String),
    #[error("proof generation failed: {0}")]
    ProofGenerationFailed(String),
    #[error("delegation denied: {0}")]
    DelegationDenied(String),
    #[error("liveness gap detected")]
    LivenessGap,
    #[error("ledger is read-only")]
    LedgerReadOnly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Untrusted < SecurityLevel::Standard);
        assert!(SecurityLevel::Standard < SecurityLevel::Sandboxed);
        assert!(SecurityLevel::Sandboxed < SecurityLevel::HardwareEnclave);
        assert!(SecurityLevel::HardwareEnclave < SecurityLevel::ConfidentialCompute);
        assert!(SecurityLevel::ConfidentialCompute < SecurityLevel::FullyVerified);
    }

    #[test]
    fn test_attestation_platform_variants() {
        let platforms = [
            AttestationPlatform::TPM,
            AttestationPlatform::TrustZone,
            AttestationPlatform::SGX,
            AttestationPlatform::AMDSecure,
            AttestationPlatform::NVIDIACC,
            AttestationPlatform::AppleSE,
            AttestationPlatform::AWSNitro,
            AttestationPlatform::AzureConfidential,
        ];
        assert_eq!(platforms.len(), 8);
    }

    #[test]
    fn test_proof_system_variants() {
        let systems = [
            ProofSystem::Groth16,
            ProofSystem::PLONK,
            ProofSystem::Halo2,
            ProofSystem::STARKs,
            ProofSystem::Nova,
            ProofSystem::Bulletproofs,
        ];
        assert_eq!(systems.len(), 6);
    }

    #[test]
    fn test_zk_proof_serialization_roundtrip() {
        let proof = ZKProof {
            proof_type: "code_integrity".to_string(),
            proof_bytes: vec![1, 2, 3, 4],
            public_inputs: vec![5, 6],
            verification_key: vec![7, 8, 9],
            timestamp: SystemTime::now(),
        };
        let json = serde_json::to_string(&proof).unwrap();
        let deserialized: ZKProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof.proof_type, deserialized.proof_type);
        assert_eq!(proof.proof_bytes, deserialized.proof_bytes);
    }

    #[test]
    fn test_session_credential_serialization() {
        let session = SessionCredential {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            task_scope: "test-task".to_string(),
            compute_budget: 100.0,
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            hardware_binding: None,
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionCredential = serde_json::from_str(&json).unwrap();
        assert_eq!(session.id, deserialized.id);
        assert_eq!(session.task_scope, deserialized.task_scope);
    }

    #[test]
    fn test_ledger_event_type_variants() {
        let events = [
            LedgerEventType::SessionCreated,
            LedgerEventType::SessionRevoked,
            LedgerEventType::DelegationGranted,
            LedgerEventType::DelegationRevoked,
            LedgerEventType::AttestationVerified,
            LedgerEventType::LivenessProven,
            LedgerEventType::ProofGenerated,
        ];
        assert_eq!(events.len(), 7);
    }

    #[test]
    fn test_zkpa_error_display() {
        let err = ZkpaError::SessionExpired;
        assert_eq!(err.to_string(), "session expired");

        let err = ZkpaError::AttestationFailed("tpm unavailable".to_string());
        assert_eq!(err.to_string(), "attestation failed: tpm unavailable");
    }

    #[test]
    fn test_hardware_attestation_serialization() {
        let attestation = HardwareAttestation {
            platform: AttestationPlatform::TPM,
            attestation_data: vec![10, 20, 30],
            timestamp: SystemTime::now(),
            verified: true,
        };
        let json = serde_json::to_string(&attestation).unwrap();
        let deserialized: HardwareAttestation = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.platform, AttestationPlatform::TPM));
        assert!(deserialized.verified);
    }

    #[test]
    fn test_authority_delegation_serialization() {
        let delegation = AuthorityDelegation {
            parent_id: Uuid::new_v4(),
            child_id: Uuid::new_v4(),
            scoped_permissions: vec!["read".to_string(), "write".to_string()],
            zk_proof: None,
            revoked: false,
        };
        let json = serde_json::to_string(&delegation).unwrap();
        let deserialized: AuthorityDelegation = serde_json::from_str(&json).unwrap();
        assert_eq!(
            delegation.scoped_permissions,
            deserialized.scoped_permissions
        );
        assert!(!deserialized.revoked);
    }
}
