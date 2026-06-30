use crate::types::*;
use sha2::{Digest, Sha256};
use std::time::SystemTime;

pub struct ProofGenerator;

impl Default for ProofGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProofGenerator {
    pub fn new() -> Self {
        ProofGenerator
    }

    pub fn generate_code_integrity_proof(
        &self,
        code_hash: &[u8],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"code_integrity");
        hasher.update(code_hash);
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "code_integrity".to_string(),
            proof_bytes: hash_result.clone(),
            public_inputs: code_hash.to_vec(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_authorization_proof(
        &self,
        agent_id: &[u8],
        permissions: &[String],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"authorization");
        hasher.update(agent_id);
        for perm in permissions {
            hasher.update(perm.as_bytes());
        }
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "authorization".to_string(),
            proof_bytes: hash_result,
            public_inputs: agent_id.to_vec(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_policy_compliance_proof(
        &self,
        policy_hash: &[u8],
        resource_hash: &[u8],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"policy_compliance");
        hasher.update(policy_hash);
        hasher.update(resource_hash);
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "policy_compliance".to_string(),
            proof_bytes: hash_result,
            public_inputs: [policy_hash, resource_hash].concat(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_hardware_attestation_proof(
        &self,
        attestation: &HardwareAttestation,
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"hardware_attestation");
        hasher.update(&attestation.attestation_data);
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "hardware_attestation".to_string(),
            proof_bytes: hash_result,
            public_inputs: attestation.attestation_data.clone(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_liveness_proof(
        &self,
        agent_id: &[u8],
        sequence_num: u64,
        hash_chain: &[u8; 32],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"liveness");
        hasher.update(agent_id);
        hasher.update(sequence_num.to_le_bytes());
        hasher.update(hash_chain);
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "liveness".to_string(),
            proof_bytes: hash_result,
            public_inputs: hash_chain.to_vec(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_computation_correctness_proof(
        &self,
        input_hash: &[u8],
        output_hash: &[u8],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"computation_correctness");
        hasher.update(input_hash);
        hasher.update(output_hash);
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "computation_correctness".to_string(),
            proof_bytes: hash_result,
            public_inputs: [input_hash, output_hash].concat(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }

    pub fn generate_delegation_proof(
        &self,
        parent_id: &[u8],
        child_id: &[u8],
        permissions: &[String],
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut hasher = Sha256::new();
        hasher.update(b"delegation");
        hasher.update(parent_id);
        hasher.update(child_id);
        for perm in permissions {
            hasher.update(perm.as_bytes());
        }
        let hash_result = hasher.finalize().to_vec();

        Ok(ZKProof {
            proof_type: "delegation".to_string(),
            proof_bytes: hash_result,
            public_inputs: [parent_id, child_id].concat(),
            verification_key: circuit.proof_system.to_bytes(),
            timestamp: SystemTime::now(),
        })
    }
}

impl ProofSystem {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            ProofSystem::Groth16 => b"groth16".to_vec(),
            ProofSystem::PLONK => b"plonk".to_vec(),
            ProofSystem::Halo2 => b"halo2".to_vec(),
            ProofSystem::STARKs => b"starks".to_vec(),
            ProofSystem::Nova => b"nova".to_vec(),
            ProofSystem::Bulletproofs => b"bulletproofs".to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_circuit() -> ZKCircuit {
        ZKCircuit {
            circuit_type: "test".to_string(),
            proof_system: ProofSystem::PLONK,
        }
    }

    #[test]
    fn test_code_integrity_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_code_integrity_proof(b"test_code_hash", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "code_integrity");
        assert!(!proof.proof_bytes.is_empty());
    }

    #[test]
    fn test_authorization_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let perms = vec!["read".to_string(), "write".to_string()];
        let proof = gen
            .generate_authorization_proof(b"agent123", &perms, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "authorization");
    }

    #[test]
    fn test_hardware_attestation_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let attestation = HardwareAttestation {
            platform: AttestationPlatform::TPM,
            attestation_data: vec![1, 2, 3],
            timestamp: SystemTime::now(),
            verified: true,
        };
        let proof = gen
            .generate_hardware_attestation_proof(&attestation, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "hardware_attestation");
    }

    #[test]
    fn test_liveness_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let hash_chain = [42u8; 32];
        let proof = gen
            .generate_liveness_proof(b"agent456", 1, &hash_chain, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "liveness");
    }

    #[test]
    fn test_computation_correctness_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_computation_correctness_proof(b"input", b"output", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "computation_correctness");
    }

    #[test]
    fn test_policy_compliance_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_policy_compliance_proof(b"policy", b"resource", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "policy_compliance");
    }

    #[test]
    fn test_delegation_proof() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let perms = vec!["execute".to_string()];
        let proof = gen
            .generate_delegation_proof(b"parent", b"child", &perms, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "delegation");
    }
}
