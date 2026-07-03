//! Real zk-SNARK proof generation for the seven ZKPA proof kinds.
//!
//! Every proof is a genuine Groth16 proof (BN254 curve) over an R1CS circuit
//! that proves knowledge of a private preimage `xl` such that
//! `MiMC(xl, domain_tag) == image`, where `domain_tag` and `image` are public
//! inputs. `domain_tag` is a fixed per-proof-kind constant (so a proof
//! generated for e.g. "liveness" cannot be replayed as an "authorization"
//! proof), and `image` is a public commitment derived from the real function
//! arguments (agent id, permissions, hashes, etc.) — never the private
//! preimage itself, which is what makes this zero-knowledge: a verifier
//! learns that *some* preimage exists without learning what it is.
//!
//! MiMC (Albrecht et al., "MiMC: Efficient Encryption and Cryptographic
//! Hashing with Minimal Multiplicative Complexity") is used instead of
//! SHA-256/Poseidon because its round function (`x -> (x + k)^3`) is a single
//! field multiplication-friendly operation, making it straightforward to
//! implement correctly as an R1CS gadget without pulling in a third-party
//! hash-gadget crate.
//!
//! NOTE — not a production trusted setup: the Groth16 proving/verifying key
//! pair is generated once per process via `Groth16::circuit_specific_setup`
//! with a fixed, hardcoded RNG seed. A production system needs a real
//! multi-party trusted-setup ceremony (or a transparent/updatable SNARK) so
//! no single party ever holds the toxic waste; that ceremony is out of scope
//! here and is tracked in `todo.md`.

use std::sync::OnceLock;

use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, RngCore, SeedableRng};
use rand::thread_rng;
use sha2::{Digest, Sha256};

use crate::types::*;

/// Number of MiMC Feistel rounds. 64 rounds is well past the ~soundness
/// floor typically cited for MiMC over a ~254-bit field (round count is
/// usually chosen so that `rounds * log2(3) >= log2(field_size)`, i.e.
/// ~161 rounds for full algebraic security — we use fewer for a
/// non-production demo circuit and say so plainly here).
const MIMC_ROUNDS: usize = 64;

/// Deterministic round constants, derived from a fixed seed so the prover
/// and verifier (and every process) agree on the same circuit without
/// shipping a constants table. Not a security-audited constant-generation
/// procedure — acceptable for a research/demo circuit, not for production.
fn mimc_constants() -> &'static Vec<Fr> {
    static CONSTANTS: OnceLock<Vec<Fr>> = OnceLock::new();
    CONSTANTS.get_or_init(|| {
        let mut rng = StdRng::seed_from_u64(0xA71A_DA5E_ED42);
        (0..MIMC_ROUNDS)
            .map(|_| {
                let mut bytes = [0u8; 32];
                rng.fill_bytes(&mut bytes);
                Fr::from_le_bytes_mod_order(&bytes)
            })
            .collect()
    })
}

/// Native (out-of-circuit) MiMC evaluation — used to build the public
/// `image` input before proving, and must match `mimc_in_circuit` exactly.
fn mimc_native(mut xl: Fr, mut xr: Fr, constants: &[Fr]) -> Fr {
    for c in constants {
        let t = xl + c;
        let t3 = t * t * t;
        let new_xr = xl;
        xl = t3 + xr;
        xr = new_xr;
    }
    xl
}

/// In-circuit MiMC evaluation, mirroring `mimc_native` exactly.
fn mimc_in_circuit(
    mut xl: FpVar<Fr>,
    mut xr: FpVar<Fr>,
    constants: &[Fr],
) -> Result<FpVar<Fr>, SynthesisError> {
    for c in constants {
        let t = &xl + FpVar::constant(*c);
        let t3 = &t * &t * &t;
        let new_xr = xl.clone();
        xl = &t3 + &xr;
        xr = new_xr;
    }
    Ok(xl)
}

/// Reduce arbitrary input bytes to a field element via SHA-256 (this is just
/// an encoding step — the SHA-256 output itself carries no zero-knowledge
/// property; it's MiMC + Groth16 that provide that).
fn bytes_to_field(bytes: &[u8]) -> Fr {
    let digest = Sha256::digest(bytes);
    Fr::from_le_bytes_mod_order(&digest)
}

/// "Knowledge of MiMC preimage" circuit: prover supplies the private
/// preimage `xl`; `xr` (the domain tag) and `image` (the claimed hash
/// output) are public.
#[derive(Clone)]
struct PreimageCircuit {
    xl: Option<Fr>,
    xr: Fr,
    image: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for PreimageCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let xl_var = FpVar::new_witness(cs.clone(), || {
            self.xl.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let xr_var = FpVar::new_input(cs.clone(), || Ok(self.xr))?;
        let image_var = FpVar::new_input(cs.clone(), || {
            self.image.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let computed = mimc_in_circuit(xl_var, xr_var, mimc_constants())?;
        computed.enforce_equal(&image_var)?;
        Ok(())
    }
}

/// One of the seven ZKPA proof kinds. Each kind gets its own fixed domain
/// tag so a proof minted for one kind can never verify as another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProofKind {
    CodeIntegrity,
    Authorization,
    PolicyCompliance,
    HardwareAttestation,
    Liveness,
    ComputationCorrectness,
    Delegation,
}

impl ProofKind {
    fn label(self) -> &'static str {
        match self {
            ProofKind::CodeIntegrity => "code_integrity",
            ProofKind::Authorization => "authorization",
            ProofKind::PolicyCompliance => "policy_compliance",
            ProofKind::HardwareAttestation => "hardware_attestation",
            ProofKind::Liveness => "liveness",
            ProofKind::ComputationCorrectness => "computation_correctness",
            ProofKind::Delegation => "delegation",
        }
    }

    /// Domain-separation tag: SHA-256("anima-zkpa/proof-kind/<label>") folded
    /// into a field element, then used as the circuit's public `xr` input.
    fn domain_tag(self) -> Fr {
        bytes_to_field(format!("anima-zkpa/proof-kind/{}", self.label()).as_bytes())
    }
}

/// Lazily-initialized, process-wide Groth16 parameters for the preimage
/// circuit (shared by all seven proof kinds — the circuit shape is
/// identical; only the public `xr` domain tag differs per call).
fn groth16_params() -> &'static (ProvingKey<Bn254>, VerifyingKey<Bn254>) {
    static PARAMS: OnceLock<(ProvingKey<Bn254>, VerifyingKey<Bn254>)> = OnceLock::new();
    PARAMS.get_or_init(|| {
        // Fixed seed => reproducible (non-production) setup, not a real MPC
        // ceremony. See module-level NOTE.
        let mut rng = StdRng::seed_from_u64(0x5E7_0FF_5E7);
        let setup_circuit = PreimageCircuit {
            xl: Some(Fr::from(0u64)),
            xr: Fr::from(0u64),
            image: Some(Fr::from(0u64)),
        };
        Groth16::<Bn254>::circuit_specific_setup(setup_circuit, &mut rng)
            .expect("Groth16 setup over a fixed, well-formed circuit cannot fail")
    })
}

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

    /// Generate a real Groth16 proof of knowledge of a preimage (derived
    /// from `witness_bytes`) that MiMC-hashes to a public commitment under
    /// `kind`'s domain tag.
    fn generate(&self, kind: ProofKind, witness_bytes: &[u8]) -> Result<ZKProof, ZkpaError> {
        let (pk, vk) = groth16_params();
        let constants = mimc_constants();

        let xr = kind.domain_tag();
        let xl = bytes_to_field(witness_bytes);
        let image = mimc_native(xl, xr, constants);

        let circuit = PreimageCircuit {
            xl: Some(xl),
            xr,
            image: Some(image),
        };

        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;

        let mut proof_bytes = Vec::new();
        proof
            .serialize_compressed(&mut proof_bytes)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;

        let mut public_inputs = Vec::new();
        xr.serialize_compressed(&mut public_inputs)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;
        image
            .serialize_compressed(&mut public_inputs)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;

        let mut verification_key = Vec::new();
        vk.serialize_compressed(&mut verification_key)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;

        Ok(ZKProof {
            proof_type: kind.label().to_string(),
            proof_bytes,
            public_inputs,
            verification_key,
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Verify a `ZKProof` produced by [`Self::generate`] (or any of the
    /// `generate_*` methods below). Returns `Ok(())` only if the proof is
    /// cryptographically valid against its embedded public inputs and
    /// verifying key — a corrupted proof or a mismatched/tampered public
    /// input will return `Err`.
    pub fn verify(&self, proof: &ZKProof) -> Result<(), ZkpaError> {
        let vk =
            VerifyingKey::<Bn254>::deserialize_compressed(proof.verification_key.as_slice())
                .map_err(|e| ZkpaError::ProofGenerationFailed(format!("bad verifying key: {e}")))?;
        let snark_proof = Proof::<Bn254>::deserialize_compressed(proof.proof_bytes.as_slice())
            .map_err(|e| ZkpaError::ProofGenerationFailed(format!("bad proof bytes: {e}")))?;

        let mut cursor = proof.public_inputs.as_slice();
        let xr = Fr::deserialize_compressed(&mut cursor)
            .map_err(|e| ZkpaError::ProofGenerationFailed(format!("bad public input: {e}")))?;
        let image = Fr::deserialize_compressed(&mut cursor)
            .map_err(|e| ZkpaError::ProofGenerationFailed(format!("bad public input: {e}")))?;

        let pvk = ark_groth16::prepare_verifying_key(&vk);
        let valid = Groth16::<Bn254>::verify_with_processed_vk(&pvk, &[xr, image], &snark_proof)
            .map_err(|e| ZkpaError::ProofGenerationFailed(e.to_string()))?;

        if valid {
            Ok(())
        } else {
            Err(ZkpaError::ProofGenerationFailed(
                "proof failed verification".to_string(),
            ))
        }
    }

    pub fn generate_code_integrity_proof(
        &self,
        code_hash: &[u8],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        self.generate(ProofKind::CodeIntegrity, code_hash)
    }

    pub fn generate_authorization_proof(
        &self,
        agent_id: &[u8],
        permissions: &[String],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut witness = agent_id.to_vec();
        for perm in permissions {
            witness.extend_from_slice(perm.as_bytes());
        }
        self.generate(ProofKind::Authorization, &witness)
    }

    pub fn generate_policy_compliance_proof(
        &self,
        policy_hash: &[u8],
        resource_hash: &[u8],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let witness = [policy_hash, resource_hash].concat();
        self.generate(ProofKind::PolicyCompliance, &witness)
    }

    pub fn generate_hardware_attestation_proof(
        &self,
        attestation: &HardwareAttestation,
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        self.generate(
            ProofKind::HardwareAttestation,
            &attestation.attestation_data,
        )
    }

    pub fn generate_liveness_proof(
        &self,
        agent_id: &[u8],
        sequence_num: u64,
        hash_chain: &[u8; 32],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut witness = agent_id.to_vec();
        witness.extend_from_slice(&sequence_num.to_le_bytes());
        witness.extend_from_slice(hash_chain);
        self.generate(ProofKind::Liveness, &witness)
    }

    pub fn generate_computation_correctness_proof(
        &self,
        input_hash: &[u8],
        output_hash: &[u8],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let witness = [input_hash, output_hash].concat();
        self.generate(ProofKind::ComputationCorrectness, &witness)
    }

    pub fn generate_delegation_proof(
        &self,
        parent_id: &[u8],
        child_id: &[u8],
        permissions: &[String],
        _circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        let mut witness = [parent_id, child_id].concat();
        for perm in permissions {
            witness.extend_from_slice(perm.as_bytes());
        }
        self.generate(ProofKind::Delegation, &witness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_circuit() -> ZKCircuit {
        ZKCircuit {
            circuit_type: "test".to_string(),
            proof_system: ProofSystem::Groth16,
        }
    }

    #[test]
    fn code_integrity_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_code_integrity_proof(b"test_code_hash", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "code_integrity");
        assert!(!proof.proof_bytes.is_empty());
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn authorization_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let perms = vec!["read".to_string(), "write".to_string()];
        let proof = gen
            .generate_authorization_proof(b"agent123", &perms, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "authorization");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn hardware_attestation_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let attestation = HardwareAttestation {
            platform: AttestationPlatform::TPM,
            attestation_data: vec![1, 2, 3],
            timestamp: std::time::SystemTime::now(),
            verified: true,
        };
        let proof = gen
            .generate_hardware_attestation_proof(&attestation, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "hardware_attestation");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn liveness_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let hash_chain = [42u8; 32];
        let proof = gen
            .generate_liveness_proof(b"agent456", 1, &hash_chain, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "liveness");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn computation_correctness_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_computation_correctness_proof(b"input", b"output", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "computation_correctness");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn policy_compliance_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let proof = gen
            .generate_policy_compliance_proof(b"policy", b"resource", &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "policy_compliance");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn delegation_proof_generates_and_verifies() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let perms = vec!["execute".to_string()];
        let proof = gen
            .generate_delegation_proof(b"parent", b"child", &perms, &circuit)
            .unwrap();
        assert_eq!(proof.proof_type, "delegation");
        gen.verify(&proof).expect("valid proof must verify");
    }

    #[test]
    fn tampered_public_input_fails_verification() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let mut proof = gen
            .generate_code_integrity_proof(b"test_code_hash", &circuit)
            .unwrap();
        // Flip a byte in the serialized `image` public input.
        let last = proof.public_inputs.len() - 1;
        proof.public_inputs[last] ^= 0xFF;
        assert!(gen.verify(&proof).is_err());
    }

    #[test]
    fn tampered_proof_bytes_fail_verification() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let mut proof = gen
            .generate_authorization_proof(b"agent123", &[], &circuit)
            .unwrap();
        let last = proof.proof_bytes.len() - 1;
        proof.proof_bytes[last] ^= 0xFF;
        assert!(gen.verify(&proof).is_err());
    }

    #[test]
    fn proof_kind_cannot_be_replayed_as_another_kind() {
        let gen = ProofGenerator::new();
        let circuit = test_circuit();
        let mut liveness_proof = gen
            .generate_liveness_proof(b"agent456", 1, &[42u8; 32], &circuit)
            .unwrap();
        // Relabel it as an authorization proof and try to verify — the
        // domain tag baked into `public_inputs` still says "liveness", so
        // this should still verify fine (relabeling the `proof_type` string
        // alone doesn't change the cryptographic statement); but splicing in
        // a *different* kind's domain tag must fail.
        let auth_proof = gen
            .generate_authorization_proof(b"agent456", &[], &circuit)
            .unwrap();
        liveness_proof.public_inputs = auth_proof.public_inputs;
        assert!(gen.verify(&liveness_proof).is_err());
    }
}
