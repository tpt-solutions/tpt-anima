use crate::types::*;
use sha2::{Digest, Sha256};
use std::time::SystemTime;

pub struct AttestationService;

impl Default for AttestationService {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationService {
    pub fn new() -> Self {
        AttestationService
    }

    pub fn attest_tpm(&self, tpm_data: &[u8]) -> Result<HardwareAttestation, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"tpm_attestation");
        hasher.update(tpm_data);
        let attestation_data = hasher.finalize().to_vec();

        Ok(HardwareAttestation {
            platform: AttestationPlatform::TPM,
            attestation_data,
            timestamp: SystemTime::now(),
            verified: true,
        })
    }

    pub fn attest_trustzone(&self, tz_data: &[u8]) -> Result<HardwareAttestation, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"trustzone_attestation");
        hasher.update(tz_data);
        let attestation_data = hasher.finalize().to_vec();

        Ok(HardwareAttestation {
            platform: AttestationPlatform::TrustZone,
            attestation_data,
            timestamp: SystemTime::now(),
            verified: true,
        })
    }

    pub fn attest_sgx(&self, sgx_data: &[u8]) -> Result<HardwareAttestation, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"sgx_attestation");
        hasher.update(sgx_data);
        let attestation_data = hasher.finalize().to_vec();

        Ok(HardwareAttestation {
            platform: AttestationPlatform::SGX,
            attestation_data,
            timestamp: SystemTime::now(),
            verified: true,
        })
    }

    pub fn attest_nvidia_cc(&self, nvidia_data: &[u8]) -> Result<HardwareAttestation, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"nvidia_cc_attestation");
        hasher.update(nvidia_data);
        let attestation_data = hasher.finalize().to_vec();

        Ok(HardwareAttestation {
            platform: AttestationPlatform::NVIDIACC,
            attestation_data,
            timestamp: SystemTime::now(),
            verified: true,
        })
    }

    pub fn attest_apple_se(&self, apple_data: &[u8]) -> Result<HardwareAttestation, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"apple_se_attestation");
        hasher.update(apple_data);
        let attestation_data = hasher.finalize().to_vec();

        Ok(HardwareAttestation {
            platform: AttestationPlatform::AppleSE,
            attestation_data,
            timestamp: SystemTime::now(),
            verified: true,
        })
    }

    pub fn verify_attestation(&self, attestation: &HardwareAttestation) -> Result<(), ZkpaError> {
        if attestation.attestation_data.is_empty() {
            return Err(ZkpaError::AttestationFailed(
                "empty attestation data".to_string(),
            ));
        }
        if !attestation.verified {
            return Err(ZkpaError::AttestationFailed(
                "attestation not verified".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tpm_attestation() {
        let svc = AttestationService::new();
        let attestation = svc.attest_tpm(b"tpm_quote_data").unwrap();
        assert!(matches!(attestation.platform, AttestationPlatform::TPM));
        assert!(attestation.verified);
        assert!(!attestation.attestation_data.is_empty());
    }

    #[test]
    fn test_verify_attestation_success() {
        let svc = AttestationService::new();
        let attestation = svc.attest_sgx(b"sgx_report").unwrap();
        assert!(svc.verify_attestation(&attestation).is_ok());
    }

    #[test]
    fn test_verify_attestation_empty_data_fails() {
        let svc = AttestationService::new();
        let attestation = HardwareAttestation {
            platform: AttestationPlatform::TPM,
            attestation_data: vec![],
            timestamp: SystemTime::now(),
            verified: true,
        };
        assert!(svc.verify_attestation(&attestation).is_err());
    }

    #[test]
    fn test_verify_attestation_not_verified_fails() {
        let svc = AttestationService::new();
        let attestation = HardwareAttestation {
            platform: AttestationPlatform::TrustZone,
            attestation_data: vec![1, 2, 3],
            timestamp: SystemTime::now(),
            verified: false,
        };
        assert!(svc.verify_attestation(&attestation).is_err());
    }

    #[test]
    fn test_nvidia_cc_attestation() {
        let svc = AttestationService::new();
        let attestation = svc.attest_nvidia_cc(b"gpu_evidence").unwrap();
        assert!(matches!(
            attestation.platform,
            AttestationPlatform::NVIDIACC
        ));
    }
}
