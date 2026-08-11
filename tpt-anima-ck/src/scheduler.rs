//! Attention-weight inference scheduler for the Cognitive Kernel.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

// ── Job status ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Preempted,
    Completed,
    Failed,
    Cancelled,
}

// ── Inference job ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceJob {
    pub inference: Inference,
    pub status: JobStatus,
    pub submitted_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub attention_weight: f32,
}

// ── Scheduler ───────────────────────────────────────────────────────────────

pub struct InferenceScheduler {
    jobs: Vec<InferenceJob>,
}

impl InferenceScheduler {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Submit a new inference job. Returns the assigned job index.
    pub fn submit(&mut self, inference: Inference, attention_weight: f32) -> usize {
        let job = InferenceJob {
            inference,
            status: JobStatus::Pending,
            submitted_at: SystemTime::now(),
            started_at: None,
            attention_weight,
        };
        self.jobs.push(job);
        self.jobs.len() - 1
    }

    /// Schedule the next job to run.
    /// Selects the best ready job (Pending or Preempted) with the highest
    /// effective score: `priority_value * 2.0 + attention_weight`. If multiple
    /// jobs tie, the earliest-submitted job wins (stable ordering).
    ///
    /// Returns a reference to the now-running job, or `None` if nothing is
    /// ready.
    pub fn schedule_next(&mut self) -> Option<&InferenceJob> {
        let best = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| j.status == JobStatus::Pending || j.status == JobStatus::Preempted)
            .max_by(|(_, a), (_, b)| {
                let sa = a.inference.priority as u8 as f32 * 2.0 + a.attention_weight;
                let sb = b.inference.priority as u8 as f32 * 2.0 + b.attention_weight;
                sa.partial_cmp(&sb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.submitted_at.cmp(&b.submitted_at).reverse())
            })
            .map(|(idx, _)| idx)?;

        self.jobs[best].status = JobStatus::Running;
        self.jobs[best].started_at = Some(SystemTime::now());
        Some(&self.jobs[best])
    }

    /// Preempt a running job, returning it to `Pending`.
    ///
    /// Returns `true` if the job existed and was running.
    pub fn preempt(&mut self, job_index: usize) -> bool {
        match self.jobs.get_mut(job_index) {
            Some(job) if job.status == JobStatus::Running => {
                job.status = JobStatus::Preempted;
                true
            }
            _ => false,
        }
    }

    /// Cancel a job by index. Returns `true` if it was in a cancellable state
    /// (Pending or Preempted).
    pub fn cancel(&mut self, job_index: usize) -> bool {
        match self.jobs.get_mut(job_index) {
            Some(job) if job.status == JobStatus::Pending || job.status == JobStatus::Preempted => {
                job.status = JobStatus::Cancelled;
                true
            }
            _ => false,
        }
    }

    /// Returns an iterator over all active (Running or Preempted) jobs.
    pub fn active_jobs(&self) -> Vec<&InferenceJob> {
        self.jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Preempted)
            .collect()
    }

    /// Number of jobs still pending.
    pub fn pending_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == JobStatus::Pending)
            .count()
    }

    /// Compute aggregate KV-cache pressure across all running jobs.
    ///
    /// Pressure is the sum of `cache_size_tokens` for every running job,
    /// normalised by the maximum cache size among them (or 0.0 if none).
    pub fn kv_cache_pressure(&self) -> f32 {
        let running: Vec<_> = self
            .jobs
            .iter()
            .filter(|j| j.status == JobStatus::Running)
            .collect();

        if running.is_empty() {
            return 0.0;
        }

        let total: u64 = running
            .iter()
            .map(|j| j.inference.kv_cache_config.cache_size_tokens)
            .sum();
        let max_single = running
            .iter()
            .map(|j| j.inference.kv_cache_config.cache_size_tokens)
            .max()
            .unwrap_or(1);

        total as f32 / max_single as f32
    }
}

impl Default for InferenceScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_inference(priority: InferencePriority, cache_tokens: u64) -> Inference {
        Inference {
            id: Uuid::new_v4(),
            model_id: "test-model".into(),
            resource_requirements: ResourceRequest {
                cpu_cores: 1,
                gpu_count: 0,
                vram_mb: 0,
                ram_mb: 1_000,
                nvme_gb: 0,
            },
            kv_cache_config: KVCacheConfig {
                cache_size_tokens: cache_tokens,
                eviction_policy: EvictionPolicy::LRU,
                prefetch_depth: 0,
            },
            circuit_breaker_settings: CircuitBreakerConfig {
                entropy_threshold: 0.5,
                max_repetition_count: 3,
                causal_violation_tolerance: 1,
            },
            priority,
        }
    }

    #[test]
    fn submit_and_pending_count() {
        let mut sched = InferenceScheduler::new();
        assert_eq!(sched.pending_count(), 0);

        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        sched.submit(make_inference(InferencePriority::Low, 200), 0.3);
        assert_eq!(sched.pending_count(), 2);
    }

    #[test]
    fn schedule_next_picks_highest_priority() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Low, 100), 0.5);
        sched.submit(make_inference(InferencePriority::Critical, 100), 0.5);
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);

        let job = sched.schedule_next().unwrap();
        assert_eq!(job.inference.priority, InferencePriority::Critical);
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());
        assert_eq!(sched.pending_count(), 2);
    }

    #[test]
    fn schedule_next_respects_attention_weight() {
        let mut sched = InferenceScheduler::new();
        // Same priority, different attention weights
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.2);
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.9);

        let job = sched.schedule_next().unwrap();
        assert_eq!(job.attention_weight, 0.9);
    }

    #[test]
    fn schedule_next_falls_back_to_fifo_for_equal_score() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);

        let job = sched.schedule_next().unwrap();
        // First submitted should win (same priority + same attention weight)
        assert_eq!(job.inference.priority, InferencePriority::Normal);
        assert_eq!(sched.pending_count(), 1);
    }

    #[test]
    fn schedule_next_returns_none_when_empty() {
        let mut sched = InferenceScheduler::new();
        assert!(sched.schedule_next().is_none());
    }

    #[test]
    fn preempt_and_reschedule() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        sched.submit(make_inference(InferencePriority::Low, 100), 0.5);

        let idx = 0;
        sched.schedule_next(); // Normal is now Running

        assert!(sched.preempt(idx));
        assert_eq!(sched.jobs[idx].status, JobStatus::Preempted);
        assert_eq!(sched.pending_count(), 1); // Only Low is still Pending

        let job = sched.schedule_next().unwrap();
        assert_eq!(job.inference.priority, InferencePriority::Normal);
    }

    #[test]
    fn preempt_non_running_fails() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        assert!(!sched.preempt(0)); // Still Pending, not Running
    }

    #[test]
    fn cancel_pending_job() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        assert!(sched.cancel(0));
        assert_eq!(sched.jobs[0].status, JobStatus::Cancelled);
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn cancel_running_job_fails() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        sched.schedule_next();
        assert!(!sched.cancel(0)); // Running, not cancellable
    }

    #[test]
    fn active_jobs_filter() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 100), 0.5);
        sched.submit(make_inference(InferencePriority::High, 100), 0.5);
        sched.schedule_next(); // Highest priority runs
        sched.submit(make_inference(InferencePriority::Low, 100), 0.5);

        let active = sched.active_jobs();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].status, JobStatus::Running);
    }

    #[test]
    fn kv_cache_pressure_empty() {
        let sched = InferenceScheduler::new();
        assert_eq!(sched.kv_cache_pressure(), 0.0);
    }

    #[test]
    fn kv_cache_pressure_single_job() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Normal, 5_000), 0.5);
        sched.schedule_next();
        // Single running job: pressure = 5000 / 5000 = 1.0
        assert_eq!(sched.kv_cache_pressure(), 1.0);
    }

    #[test]
    fn kv_cache_pressure_multiple_jobs() {
        let mut sched = InferenceScheduler::new();
        sched.submit(make_inference(InferencePriority::Critical, 10_000), 0.5);
        sched.submit(make_inference(InferencePriority::High, 3_000), 0.5);
        sched.schedule_next(); // Critical
        sched.schedule_next(); // High

        // Total = 13_000, max = 10_000 → pressure = 1.3
        let pressure = sched.kv_cache_pressure();
        assert!((pressure - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn default_impl() {
        let sched = InferenceScheduler::default();
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn cancel_out_of_bounds() {
        let mut sched = InferenceScheduler::new();
        assert!(!sched.cancel(99));
    }

    #[test]
    fn preempt_out_of_bounds() {
        let mut sched = InferenceScheduler::new();
        assert!(!sched.preempt(99));
    }
}
