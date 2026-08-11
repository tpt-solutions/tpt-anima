//! Async Proof Generation — non-blocking ZK proof generation for inference.
//!
//! Provides a task-queue-based approach to proof generation that does not block
//! the inference pipeline. Uses tokio for async execution with bounded concurrency.

use crate::types::*;
use crate::ProofGenerator;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Semaphore};

// ── Async Proof Job ───────────────────────────────────────────────────────────

/// A pending proof generation job.
#[derive(Debug, Clone)]
pub struct ProofJob {
    pub job_id: u64,
    pub proof_type: String,
    pub data: ProofData,
    pub circuit: ZKCircuit,
}

/// Data required for different proof types.
#[derive(Debug, Clone)]
pub enum ProofData {
    CodeIntegrity {
        code_hash: Vec<u8>,
    },
    Authorization {
        agent_id: Vec<u8>,
        permissions: Vec<String>,
    },
    PolicyCompliance {
        policy_hash: Vec<u8>,
        resource_hash: Vec<u8>,
    },
    HardwareAttestation {
        attestation: HardwareAttestation,
    },
    Liveness {
        agent_id: Vec<u8>,
        sequence_num: u64,
        hash_chain: [u8; 32],
    },
    ComputationCorrectness {
        input_hash: Vec<u8>,
        output_hash: Vec<u8>,
    },
    Delegation {
        parent_id: Vec<u8>,
        child_id: Vec<u8>,
        permissions: Vec<String>,
    },
}

/// Result of an async proof generation job.
#[derive(Debug, Clone)]
pub struct ProofResult {
    pub job_id: u64,
    pub proof: Result<ZKProof, ZkpaError>,
}

// ── AsyncProofGenerator ─────────────────────────────────────────────────────

/// Async proof generator that processes proofs without blocking inference.
pub struct AsyncProofGenerator {
    tx: mpsc::Sender<ProofJob>,
    rx: Mutex<mpsc::Receiver<ProofResult>>,
    /// Maximum concurrent proof generation tasks
    concurrency_limit: Arc<Semaphore>,
    current_job_id: Arc<Mutex<u64>>,
}

impl AsyncProofGenerator {
    /// Create a new async proof generator with specified concurrency limit.
    /// Higher concurrency uses more CPU but reduces proof backlog.
    pub fn new(max_concurrent: usize) -> Self {
        let (tx, mut rx_internal) = mpsc::channel::<ProofJob>(128);
        let (tx_result, rx_result) = mpsc::channel::<ProofResult>(128);
        let concurrency_limit = Arc::new(Semaphore::new(max_concurrent));
        let current_job_id = Arc::new(Mutex::new(0u64));
        let tx_result_clone = tx_result.clone();
        let concurrency_limit_clone = concurrency_limit.clone();

        // Spawn the worker task
        let _tx_for_join = tx.clone();
        tokio::spawn(async move {
            while let Some(job) = rx_internal.recv().await {
                let permit = concurrency_limit_clone
                    .clone()
                    .acquire_owned()
                    .await
                    .unwrap();
                let tx_result_inner = tx_result_clone.clone();
                let generator = ProofGenerator::new();

                tokio::spawn(async move {
                    let _permit = permit; // Hold the semaphore permit
                    let result = ProofResult {
                        job_id: job.job_id,
                        proof: Self::generate_proof(
                            &generator,
                            &job.proof_type,
                            &job.data,
                            &job.circuit,
                        ),
                    };
                    let _ = tx_result_inner.send(result).await;
                });
            }
        });

        Self {
            tx,
            rx: Mutex::new(rx_result),
            concurrency_limit,
            current_job_id,
        }
    }

    /// Number of proof-generation slots currently free (bounded by `max_concurrent`).
    pub fn available_permits(&self) -> usize {
        self.concurrency_limit.available_permits()
    }

    /// Submit a proof generation job without blocking.
    /// Returns the job ID for result tracking.
    pub async fn submit_proof(
        &self,
        data: ProofData,
        proof_type: &str,
        circuit: ZKCircuit,
    ) -> Result<u64, ProofSubmitError> {
        let mut job_id_lock = self.current_job_id.lock().await;
        *job_id_lock = job_id_lock.wrapping_add(1);
        let job_id = *job_id_lock;
        drop(job_id_lock); // Release the mutex early

        let job = ProofJob {
            job_id,
            proof_type: proof_type.to_string(),
            data,
            circuit,
        };

        self.tx
            .send(job)
            .await
            .map(|_| job_id)
            .map_err(|_| ProofSubmitError::QueueFull)
    }

    /// Try to receive a completed proof result (non-blocking).
    pub async fn try_recv_result(&self) -> Option<ProofResult> {
        self.rx.lock().await.try_recv().ok()
    }

    /// Receive a completed proof result (blocking).
    pub async fn recv_result(&self) -> ProofResult {
        self.rx.lock().await.recv().await.unwrap()
    }

    /// Generate proof synchronously (used internally by worker task).
    fn generate_proof(
        generator: &ProofGenerator,
        proof_type: &str,
        data: &ProofData,
        circuit: &ZKCircuit,
    ) -> Result<ZKProof, ZkpaError> {
        match proof_type {
            "code_integrity" => {
                if let ProofData::CodeIntegrity { code_hash } = data {
                    generator.generate_code_integrity_proof(code_hash, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "authorization" => {
                if let ProofData::Authorization {
                    agent_id,
                    permissions,
                } = data
                {
                    generator.generate_authorization_proof(agent_id, permissions, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "policy_compliance" => {
                if let ProofData::PolicyCompliance {
                    policy_hash,
                    resource_hash,
                } = data
                {
                    generator.generate_policy_compliance_proof(policy_hash, resource_hash, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "hardware_attestation" => {
                if let ProofData::HardwareAttestation { attestation } = data {
                    generator.generate_hardware_attestation_proof(attestation, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "liveness" => {
                if let ProofData::Liveness {
                    agent_id,
                    sequence_num,
                    hash_chain,
                } = data
                {
                    generator.generate_liveness_proof(agent_id, *sequence_num, hash_chain, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "computation_correctness" => {
                if let ProofData::ComputationCorrectness {
                    input_hash,
                    output_hash,
                } = data
                {
                    generator.generate_computation_correctness_proof(
                        input_hash,
                        output_hash,
                        circuit,
                    )
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            "delegation" => {
                if let ProofData::Delegation {
                    parent_id,
                    child_id,
                    permissions,
                } = data
                {
                    generator.generate_delegation_proof(parent_id, child_id, permissions, circuit)
                } else {
                    Err(ZkpaError::ProofGenerationFailed(format!(
                        "Invalid data for {:?}",
                        proof_type
                    )))
                }
            }
            _ => Err(ZkpaError::ProofGenerationFailed(format!(
                "Unknown proof type: {}",
                proof_type
            ))),
        }
    }
}

// Re-export for internal use
use std::sync::Arc as StdArc;
use std::sync::Mutex as StdMutex;

// ── Batch Proof Generator ─────────────────────────────────────────────────────

/// Batch proof generator for high-throughput scenarios.
/// Groups multiple proof requests for efficient processing.
pub struct BatchProofGenerator {
    /// Batch size for grouping proofs
    batch_size: usize,
    /// Channel for batch collection
    pending: Vec<PendingProof>,
}

struct PendingProof {
    job: ProofJob,
    responder: tokio::sync::oneshot::Sender<Result<ZKProof, ZkpaError>>,
}

impl BatchProofGenerator {
    /// Create a new batch proof generator.
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            pending: Vec::new(),
        }
    }

    /// Add a proof to the pending batch.
    pub fn add_to_batch(
        &mut self,
        job: ProofJob,
        responder: tokio::sync::oneshot::Sender<Result<ZKProof, ZkpaError>>,
    ) {
        self.pending.push(PendingProof { job, responder });
    }

    /// Check if batch is ready for processing.
    pub fn is_batch_ready(&self) -> bool {
        self.pending.len() >= self.batch_size || !self.pending.is_empty() && self.should_flush()
    }

    /// Generate proofs for every pending job and send each result to its responder,
    /// clearing the batch. Returns the number of proofs processed.
    pub fn flush(&mut self, generator: &ProofGenerator) -> usize {
        let flushed = std::mem::take(&mut self.pending);
        let count = flushed.len();
        for pending in flushed {
            let result = AsyncProofGenerator::generate_proof(
                generator,
                &pending.job.proof_type,
                &pending.job.data,
                &pending.job.circuit,
            );
            let _ = pending.responder.send(result);
        }
        count
    }

    fn should_flush(&self) -> bool {
        // Flush if batch contains only low-priority proofs or specific conditions
        // This is a placeholder for more sophisticated batching logic
        self.pending.len() == self.batch_size
    }
}

// ── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ProofSubmitError {
    #[error("proof queue is full")]
    QueueFull,
    #[error("proof generation failed: {0}")]
    GenerationFailed(String),
}

// ── Proof Cache ───────────────────────────────────────────────────────────────

use std::collections::HashMap;

/// LRU cache for frequently-used proofs to avoid regeneration.
pub struct ProofCache {
    cache: StdArc<StdMutex<HashMap<String, ZKProof>>>,
    max_entries: usize,
}

impl ProofCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: StdArc::new(StdMutex::new(HashMap::new())),
            max_entries,
        }
    }

    /// Get a cached proof by cache key.
    pub fn get(&self, key: &str) -> Option<ZKProof> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    /// Insert a proof into the cache.
    pub fn insert(&self, key: String, proof: ZKProof) {
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= self.max_entries {
            // Simple eviction: clear oldest half
            let keys: Vec<_> = cache.keys().cloned().collect();
            for key in keys.into_iter().take(self.max_entries / 2) {
                cache.remove(&key);
            }
        }
        cache.insert(key, proof);
    }

    /// Get cache hit rate statistics.
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        (cache.len(), self.max_entries)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_circuit() -> ZKCircuit {
        ZKCircuit {
            circuit_type: "test".to_string(),
            proof_system: ProofSystem::STARKs,
        }
    }

    #[test]
    fn test_proof_data_variants() {
        let _code = ProofData::CodeIntegrity {
            code_hash: vec![1, 2, 3],
        };
        let _auth = ProofData::Authorization {
            agent_id: vec![1],
            permissions: vec!["read".to_string()],
        };
        let _policy = ProofData::PolicyCompliance {
            policy_hash: vec![1],
            resource_hash: vec![2],
        };
    }

    #[test]
    fn batch_flush_resolves_pending_responders() {
        let mut batch = BatchProofGenerator::new(2);
        let (tx1, mut rx1) = tokio::sync::oneshot::channel();
        let (tx2, mut rx2) = tokio::sync::oneshot::channel();

        batch.add_to_batch(
            ProofJob {
                job_id: 1,
                proof_type: "code_integrity".to_string(),
                data: ProofData::CodeIntegrity {
                    code_hash: vec![1, 2, 3],
                },
                circuit: test_circuit(),
            },
            tx1,
        );
        assert!(!batch.is_batch_ready());

        batch.add_to_batch(
            ProofJob {
                job_id: 2,
                proof_type: "code_integrity".to_string(),
                data: ProofData::CodeIntegrity {
                    code_hash: vec![4, 5, 6],
                },
                circuit: test_circuit(),
            },
            tx2,
        );
        assert!(batch.is_batch_ready());

        let generator = ProofGenerator::new();
        let flushed = batch.flush(&generator);
        assert_eq!(flushed, 2);
        assert!(rx1.try_recv().unwrap().is_ok());
        assert!(rx2.try_recv().unwrap().is_ok());
    }

    #[test]
    fn test_proof_submit_error_display() {
        let err = ProofSubmitError::QueueFull;
        assert_eq!(err.to_string(), "proof queue is full");
    }

    #[test]
    fn test_proof_cache_insert_and_get() {
        let cache = ProofCache::new(10);
        let proof = ZKProof {
            proof_type: "test".to_string(),
            proof_bytes: vec![1, 2, 3],
            public_inputs: vec![],
            verification_key: vec![],
            timestamp: std::time::SystemTime::now(),
        };

        cache.insert("key1".to_string(), proof.clone());
        assert!(cache.get("key1").is_some());
        assert_eq!(cache.get("key1").unwrap().proof_type, "test");
        assert!(cache.get("nonexistent").is_none());

        let (entries, max) = cache.stats();
        assert_eq!(entries, 1);
        assert_eq!(max, 10);
    }

    #[test]
    fn test_proof_cache_eviction() {
        let cache = ProofCache::new(4);
        for i in 0..6 {
            let proof = ZKProof {
                proof_type: "test".to_string(),
                proof_bytes: vec![i],
                public_inputs: vec![],
                verification_key: vec![],
                timestamp: std::time::SystemTime::now(),
            };
            cache.insert(format!("key{}", i), proof);
        }
        // After inserting 6 entries into a 4-entry cache, eviction should have occurred
        let (entries, _) = cache.stats();
        assert!(entries <= 4);
    }
}
