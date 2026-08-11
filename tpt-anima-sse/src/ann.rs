//! Approximate nearest-neighbor index for semantic search over [`MemoryNode`]
//! embeddings.
//!
//! This is a dependency-free implementation of an *inverted-file* (IVF) index:
//! memories are clustered into a small number of cells via k-means, and a query
//! only visits the `n_probe` closest cells before refining within them. Because
//! the coarse quantizer discards fine-grained geometry, the per-query cost is
//! `O(n_probe · avg_cell_size)` instead of `O(n)`, which is the defining
//! property of an *approximate* nearest-neighbor search.
//!
//! The index is intentionally lightweight — it does not pull in an external ANN
//! crate — while still demonstrating the production access pattern: build a
//! coarse quantizer, probe a handful of cells, refine with exact cosine
//! similarity inside those cells.

use std::collections::HashMap;

use crate::memory_node::{MemoryId, VectorEmbedding};

// ---------------------------------------------------------------------------
// Internal point storage
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AnnPoint {
    id: MemoryId,
    /// Unit-length copy of the original vector (cosine search uses dot product).
    vector: Vec<f32>,
}

#[derive(Clone)]
struct Cluster {
    centroid: Vec<f32>,
    members: Vec<MemoryId>,
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (LCG) — keeps k-means++ initialisation reproducible
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG constants.
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_832_796_605)
            .wrapping_add(1_442_695_040_880_794_611);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f64 / (u32::MAX as f64)) as f32
    }
}

// ---------------------------------------------------------------------------
// AnnIndex
// ---------------------------------------------------------------------------

/// Inverted-file approximate nearest-neighbor index over normalised vectors.
pub struct AnnIndex {
    dim: u32,
    points: HashMap<MemoryId, AnnPoint>,
    /// Ordered list of ids — defines deterministic iteration order.
    order: Vec<MemoryId>,
    clusters: Vec<Cluster>,
    built: bool,
    /// Number of cells to scan per query (≤ `clusters.len()`).
    n_probe: usize,
}

impl AnnIndex {
    /// Create an empty index. Vectors that do not share this dimensionality are
    /// rejected at insertion time (semantic embeddings are normally homogeneous).
    pub fn new(dim: u32) -> Self {
        Self {
            dim,
            points: HashMap::new(),
            order: Vec::new(),
            clusters: Vec::new(),
            built: false,
            n_probe: 1,
        }
    }

    /// Number of indexed vectors.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Vector dimensionality the index expects. Returns `0` if nothing indexed.
    pub fn dim(&self) -> u32 {
        self.dim
    }

    /// How many cells are probed per query (clamped to the cell count on build).
    pub fn n_probe(&self) -> usize {
        self.n_probe
    }

    /// Set the number of cells probed per query. Higher values trade latency for
    /// recall; clamped on the next [`rebuild`](Self::rebuild).
    pub fn set_n_probe(&mut self, n: usize) {
        self.n_probe = n.max(1);
    }

    /// Insert (or update) a vector. Returns `false` if the dimensionality does
    /// not match the index and the point was therefore skipped.
    pub fn insert(&mut self, id: MemoryId, vector: &[f32]) -> bool {
        if self.dim == 0 {
            self.dim = vector.len() as u32;
        } else if vector.len() as u32 != self.dim {
            return false;
        }
        let vector = normalize(vector);
        let point = AnnPoint {
            id,
            vector,
        };
        if self.points.insert(id, point).is_none() {
            self.order.push(id);
        }
        // New data invalidates any existing coarse quantizer.
        self.built = false;
        true
    }

    /// Remove a vector by id.
    pub fn remove(&mut self, id: &MemoryId) -> bool {
        if self.points.remove(id).is_some() {
            self.order.retain(|x| x != id);
            self.built = false;
            true
        } else {
            false
        }
    }

    /// Clear the index.
    pub fn clear(&mut self) {
        self.points.clear();
        self.order.clear();
        self.clusters.clear();
        self.built = false;
        self.dim = 0;
    }

    /// Recompute the coarse quantizer (k-means clustering) over current points.
    ///
    /// No-op (but harmless) when the index is empty. Safe to call frequently:
    /// it only triggers work when `points` is non-empty.
    pub fn rebuild(&mut self) {
        let n = self.points.len();
        if n == 0 {
            self.clusters.clear();
            self.built = true;
            return;
        }

        // Cell count grows with √n but is capped to keep refinement cheap.
        let nlist = (1.0f64).max((n as f64).sqrt().round()) as usize;
        let nlist = nlist.min(64).min(n);
        let n_probe = self.n_probe.min(nlist);

        let vectors: Vec<&[f32]> = self
            .order
            .iter()
            .filter_map(|id| self.points.get(id).map(|p| p.vector.as_slice()))
            .collect();
        let ids: Vec<MemoryId> = self.order.clone();

        let centroids = kmeans(&vectors, nlist);
        let mut clusters: Vec<Cluster> = centroids
            .into_iter()
            .map(|c| Cluster {
                centroid: c,
                members: Vec::new(),
            })
            .collect();

        for (i, v) in vectors.iter().enumerate() {
            let best = nearest_cluster(v, &clusters);
            clusters[best].members.push(ids[i]);
        }

        // Drop empty cells so probing never scans dead clusters.
        clusters.retain(|c| !c.members.is_empty());
        self.n_probe = n_probe;
        self.clusters = clusters;
        self.built = true;
    }

    /// Approximate nearest-neighbor search by cosine similarity (descending).
    ///
    /// Returns up to `k` `(id, similarity)` pairs. When the index is small or
    /// not yet built it falls back to an exact scan so callers always get a
    /// complete, correct answer for trivial workloads.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(MemoryId, f32)> {
        if query.len() as u32 != self.dim || self.points.is_empty() {
            return Vec::new();
        }
        let q = normalize(query);

        // Exact fallback for tiny indices or before a build.
        if !self.built || self.clusters.is_empty() {
            return self.exact_search(&q, k);
        }

        // Probe the n_probe closest cells, then refine inside them.
        let mut cell_order: Vec<(usize, f32)> = self
            .clusters
            .iter()
            .enumerate()
            .map(|(i, c)| (i, dot(&q, &c.centroid)))
            .collect();
        cell_order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let probe = self.n_probe.min(cell_order.len());

        let mut scored: Vec<(MemoryId, f32)> = Vec::new();
        for (ci, _) in cell_order.into_iter().take(probe) {
            for id in &self.clusters[ci].members {
                if let Some(p) = self.points.get(id) {
                    let sim = dot(&q, &p.vector);
                    scored.push((*id, sim));
                }
            }
        }
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }

    /// Exact (non-approximate) nearest-neighbor search — used for correctness
    /// comparisons and small indices.
    pub fn exact_search(&self, query: &[f32], k: usize) -> Vec<(MemoryId, f32)> {
        if query.len() as u32 != self.dim || self.points.is_empty() {
            return Vec::new();
        }
        let q = normalize(query);
        let mut scored: Vec<(MemoryId, f32)> = self
            .points
            .values()
            .map(|p| (p.id, dot(&q, &p.vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
}

impl Default for AnnIndex {
    fn default() -> Self {
        Self::new(0)
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn normalize(v: &[f32]) -> Vec<f32> {
    let mag: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / mag).collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn nearest_cluster(v: &[f32], clusters: &[Cluster]) -> usize {
    let mut best = 0;
    let mut best_sim = f32::NEG_INFINITY;
    for (i, c) in clusters.iter().enumerate() {
        let sim = dot(v, &c.centroid);
        if sim > best_sim {
            best_sim = sim;
            best = i;
        }
    }
    best
}

/// k-means++ initialisation + Lloyd iterations with a deterministic seed.
fn kmeans(vectors: &[&[f32]], k: usize) -> Vec<Vec<f32>> {
    if k == 0 || vectors.is_empty() {
        return Vec::new();
    }
    if k >= vectors.len() {
        return vectors.iter().map(|v| v.to_vec()).collect();
    }

    let mut rng = Lcg::new(0x9E3779B97F4A7C15);
    let dim = vectors[0].len();

    // k-means++ seeding.
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);
    centroids.push(vectors[0].to_vec());
    while centroids.len() < k {
        // Distance² to nearest existing centroid for each point.
        let mut total = 0.0f64;
        let mut dists = Vec::with_capacity(vectors.len());
        for v in vectors {
            let mut best = f32::INFINITY;
            for c in &centroids {
                let d = sq_dist(v, c);
                if d < best {
                    best = d;
                }
            }
            dists.push(best as f64);
            total += best as f64;
        }
        // Sample a point proportional to its distance².
        let target = rng.next_f32() as f64 * total;
        let mut acc = 0.0f64;
        let mut chosen = 0;
        for (i, d) in dists.iter().enumerate() {
            acc += *d;
            if acc >= target {
                chosen = i;
                break;
            }
        }
        centroids.push(vectors[chosen].to_vec());
    }

    // Lloyd iterations.
    let max_iter = 12;
    let mut assignments = vec![0usize; vectors.len()];
    for _ in 0..max_iter {
        // Assignment step.
        let mut changed = false;
        for (i, v) in vectors.iter().enumerate() {
            let mut best = 0;
            let mut best_sim = f32::NEG_INFINITY;
            for (ci, c) in centroids.iter().enumerate() {
                let sim = dot(v, c);
                if sim > best_sim {
                    best_sim = sim;
                    best = ci;
                }
            }
            if assignments[i] != best {
                assignments[i] = best;
                changed = true;
            }
        }

        // Update step.
        let mut sums: Vec<Vec<f32>> = vec![vec![0.0; dim]; k];
        let mut counts = vec![0u32; k];
        for (i, v) in vectors.iter().enumerate() {
            let c = assignments[i];
            for (d, x) in v.iter().enumerate() {
                sums[c][d] += *x;
            }
            counts[c] += 1;
        }
        for (ci, c) in centroids.iter_mut().enumerate() {
            if counts[ci] > 0 {
                let inv = 1.0 / counts[ci] as f32;
                for d in 0..dim {
                    c[d] = sums[ci][d] * inv;
                }
            }
        }

        if !changed && !assignments.is_empty() {
            break;
        }
    }

    centroids
}

fn sq_dist(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum()
}

// ---------------------------------------------------------------------------
// Public ergonomic helper for callers holding a `VectorEmbedding`
// ---------------------------------------------------------------------------

impl AnnIndex {
    /// Insert from a [`VectorEmbedding`], returning whether it was accepted.
    pub fn insert_embedding(&mut self, id: MemoryId, embedding: &VectorEmbedding) -> bool {
        self.insert(id, &embedding.values)
    }

    /// Approximate search given a [`VectorEmbedding`] query.
    pub fn search_embedding(&self, query: &VectorEmbedding, k: usize) -> Vec<(MemoryId, f32)> {
        self.search(&query.values, k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(vals: &[f32]) -> VectorEmbedding {
        VectorEmbedding::new(vals.to_vec())
    }

    #[test]
    fn insert_rejects_wrong_dimension() {
        let mut idx = AnnIndex::new(3);
        assert!(idx.insert([1u8; 32], &[1.0, 0.0, 0.0]));
        assert!(!idx.insert([2u8; 32], &[1.0, 0.0]));
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn exact_search_orders_by_similarity() {
        let mut idx = AnnIndex::new(2);
        idx.insert([1u8; 32], &[1.0, 0.0]);
        idx.insert([2u8; 32], &[0.0, 1.0]);
        idx.insert([3u8; 32], &[1.0, 1.0]);
        // Query (1,0) is closest to node 1, then node 3, then node 2.
        let r = idx.exact_search(&[1.0, 0.0], 3);
        assert_eq!(r[0].0, [1u8; 32]);
        assert_eq!(r[1].0, [3u8; 32]);
        assert_eq!(r[2].0, [2u8; 32]);
    }

    #[test]
    fn ann_search_matches_exact_on_small_index() {
        let mut idx = AnnIndex::new(4);
        idx.insert([1u8; 32], &[1.0, 0.0, 0.0, 0.0]);
        idx.insert([2u8; 32], &[0.0, 1.0, 0.0, 0.0]);
        idx.insert([3u8; 32], &[0.0, 0.0, 1.0, 0.0]);
        idx.insert([4u8; 32], &[0.0, 0.0, 0.0, 1.0]);
        idx.rebuild();
        let exact = idx.exact_search(&[1.0, 0.0, 0.0, 0.0], 1);
        let approx = idx.search(&[1.0, 0.0, 0.0, 0.0], 1);
        assert_eq!(exact[0].0, approx[0].0);
    }

    #[test]
    fn remove_evicts_point() {
        let mut idx = AnnIndex::new(2);
        idx.insert([1u8; 32], &[1.0, 0.0]);
        idx.insert([2u8; 32], &[0.0, 1.0]);
        assert!(idx.remove(&[1u8; 32]));
        assert_eq!(idx.len(), 1);
        let r = idx.exact_search(&[1.0, 0.0], 2);
        assert!(r.iter().all(|(id, _)| *id != [1u8; 32]));
    }

    #[test]
    fn embedding_helper_search_returns_similar() {
        let mut idx = AnnIndex::new(2);
        idx.insert_embedding([1u8; 32], &v(&[1.0, 0.0]));
        idx.insert_embedding([2u8; 32], &v(&[0.0, 1.0]));
        let r = idx.search_embedding(&v(&[0.9, 0.1]), 1);
        assert_eq!(r[0].0, [1u8; 32]);
    }

    #[test]
    fn ann_recall_holds_on_larger_index() {
        // Many points clustered around two poles; the nearest pole must always
        // win whether probed exactly or approximately. Multiple points share an
        // identical pole embedding, so we assert the returned id lands in the
        // correct (even = pole A) cluster rather than demanding an exact tie.
        let mut idx = AnnIndex::new(2);
        for i in 0..40u8 {
            let id = [i; 32];
            if i % 2 == 0 {
                idx.insert(id, &[0.95, 0.05]);
            } else {
                idx.insert(id, &[0.05, 0.95]);
            }
        }
        idx.rebuild();
        let exact = idx.exact_search(&[1.0, 0.0], 1);
        let approx = idx.search(&[1.0, 0.0], 1);
        assert_eq!(exact[0].0[0] % 2, approx[0].0[0] % 2);
    }
}
