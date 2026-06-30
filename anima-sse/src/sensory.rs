//! Sensory Substrate Integration for the Semantic State Engine.
//!
//! Physical sensors (cameras, microphones, LiDAR) are first-class memory
//! citizens. This module defines the ingestion interface and provides
//! pipelines for converting multi-modal input into vector embeddings.

use crate::memory_node::{SensoryModality, SseError, VectorEmbedding};

// ---------------------------------------------------------------------------
// SensoryIngestionInterface trait
// ---------------------------------------------------------------------------

/// Trait for hardware sensor inputs that can be ingested into the SSE.
///
/// Cameras, microphones, LiDAR, and other sensors implement this trait
/// to convert raw sensory data into vector embeddings that can be stored
/// as [`MemoryNode`]s.
pub trait SensoryIngestionInterface {
    /// The type of raw data this sensor produces.
    type RawData;

    /// Convert raw sensor data into a vector embedding.
    fn ingest(&self, data: Self::RawData) -> Result<VectorEmbedding, SensoryError>;

    /// The sensory modality of this sensor's output.
    fn modality(&self) -> SensoryModality;

    /// The embedding dimensions this sensor produces.
    fn dimensions(&self) -> u32;
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SensoryError {
    #[error("ingestion failed: {reason}")]
    IngestionFailed { reason: String },

    #[error("unsupported modality: {0}")]
    UnsupportedModality(String),

    #[error("data format error: {0}")]
    FormatError(String),

    #[error("vector error: {0}")]
    Vector(#[from] SseError),
}

// ---------------------------------------------------------------------------
// Text → Vector pipeline
// ---------------------------------------------------------------------------

/// Simple text-to-vector embedding pipeline.
///
/// Uses a basic bag-of-words approach for demonstration. In production,
/// this would integrate with a proper embedding model (e.g., ONNX runtime).
pub struct TextEmbeddingPipeline {
    pub dimensions: u32,
}

impl TextEmbeddingPipeline {
    pub fn new(dimensions: u32) -> Self {
        Self { dimensions }
    }

    /// Convert text into a fixed-dimension vector via simple hashing.
    pub fn embed(&self, text: &str) -> Result<VectorEmbedding, SensoryError> {
        let mut values = vec![0.0f32; self.dimensions as usize];

        // Simple hash-based embedding: distribute character values across dimensions
        for (i, byte) in text.bytes().enumerate() {
            let idx = (i * 7 + byte as usize) % self.dimensions as usize;
            values[idx] += (byte as f32) / 255.0;
        }

        // Normalise
        let magnitude: f32 = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in values.iter_mut() {
                *v /= magnitude;
            }
        }

        Ok(VectorEmbedding::new(values))
    }
}

impl SensoryIngestionInterface for TextEmbeddingPipeline {
    type RawData = String;

    fn ingest(&self, data: String) -> Result<VectorEmbedding, SensoryError> {
        self.embed(&data)
    }

    fn modality(&self) -> SensoryModality {
        SensoryModality::Textual
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }
}

// ---------------------------------------------------------------------------
// Image → Spatial-Temporal Vector pipeline
// ---------------------------------------------------------------------------

/// Image-to-vector pipeline.
///
/// Converts image data (raw pixel bytes) into a spatial-temporal embedding.
/// In production, this would use a vision model (e.g., CLIP, DINOv2).
pub struct ImageEmbeddingPipeline {
    pub dimensions: u32,
}

impl ImageEmbeddingPipeline {
    pub fn new(dimensions: u32) -> Self {
        Self { dimensions }
    }

    /// Convert raw image bytes into a vector embedding.
    pub fn embed(&self, image_data: &[u8]) -> Result<VectorEmbedding, SensoryError> {
        let mut values = vec![0.0f32; self.dimensions as usize];

        // Spatial pooling: distribute pixel values across dimensions
        for (i, &pixel) in image_data.iter().enumerate() {
            let idx = (i * 13 + pixel as usize) % self.dimensions as usize;
            values[idx] += (pixel as f32) / 255.0;
        }

        let magnitude: f32 = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in values.iter_mut() {
                *v /= magnitude;
            }
        }

        Ok(VectorEmbedding::new(values))
    }
}

impl SensoryIngestionInterface for ImageEmbeddingPipeline {
    type RawData = Vec<u8>;

    fn ingest(&self, data: Vec<u8>) -> Result<VectorEmbedding, SensoryError> {
        self.embed(&data)
    }

    fn modality(&self) -> SensoryModality {
        SensoryModality::Visual
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }
}

// ---------------------------------------------------------------------------
// Audio → Vector pipeline
// ---------------------------------------------------------------------------

/// Audio-to-vector pipeline.
///
/// Converts audio samples into a vector embedding.
/// In production, this would use a speech model (e.g., Whisper, HuBERT).
pub struct AudioEmbeddingPipeline {
    pub dimensions: u32,
}

impl AudioEmbeddingPipeline {
    pub fn new(dimensions: u32) -> Self {
        Self { dimensions }
    }

    /// Convert audio samples (f32 PCM) into a vector embedding.
    pub fn embed(&self, samples: &[f32]) -> Result<VectorEmbedding, SensoryError> {
        let mut values = vec![0.0f32; self.dimensions as usize];

        // Simple spectral feature extraction: distribute sample magnitudes
        for (i, &sample) in samples.iter().enumerate() {
            let idx = (i * 11 + (sample.abs() * 1000.0) as usize) % self.dimensions as usize;
            values[idx] += sample.abs();
        }

        let magnitude: f32 = values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in values.iter_mut() {
                *v /= magnitude;
            }
        }

        Ok(VectorEmbedding::new(values))
    }
}

impl SensoryIngestionInterface for AudioEmbeddingPipeline {
    type RawData = Vec<f32>;

    fn ingest(&self, data: Vec<f32>) -> Result<VectorEmbedding, SensoryError> {
        self.embed(&data)
    }

    fn modality(&self) -> SensoryModality {
        SensoryModality::Audio
    }

    fn dimensions(&self) -> u32 {
        self.dimensions
    }
}

// ---------------------------------------------------------------------------
// SensoryManager — unified ingestion coordinator
// ---------------------------------------------------------------------------

/// Coordinates ingestion from multiple sensory pipelines into a unified
/// embedding space.
pub struct SensoryManager {
    pub text_pipeline: TextEmbeddingPipeline,
    pub image_pipeline: ImageEmbeddingPipeline,
    pub audio_pipeline: AudioEmbeddingPipeline,
}

impl SensoryManager {
    pub fn new(dimensions: u32) -> Self {
        Self {
            text_pipeline: TextEmbeddingPipeline::new(dimensions),
            image_pipeline: ImageEmbeddingPipeline::new(dimensions),
            audio_pipeline: AudioEmbeddingPipeline::new(dimensions),
        }
    }

    /// Ingest text data and return its embedding.
    pub fn ingest_text(&self, text: &str) -> Result<VectorEmbedding, SensoryError> {
        self.text_pipeline.ingest(text.to_string())
    }

    /// Ingest image data and return its embedding.
    pub fn ingest_image(&self, data: &[u8]) -> Result<VectorEmbedding, SensoryError> {
        self.image_pipeline.ingest(data.to_vec())
    }

    /// Ingest audio data and return its embedding.
    pub fn ingest_audio(&self, samples: &[f32]) -> Result<VectorEmbedding, SensoryError> {
        self.audio_pipeline.ingest(samples.to_vec())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_embedding_produces_correct_dimensions() {
        let pipeline = TextEmbeddingPipeline::new(128);
        let embedding = pipeline.embed("hello anima").unwrap();
        assert_eq!(embedding.dimensions, 128);
        assert_eq!(embedding.values.len(), 128);
    }

    #[test]
    fn text_embedding_is_normalised() {
        let pipeline = TextEmbeddingPipeline::new(64);
        let embedding = pipeline.embed("test").unwrap();
        let mag: f32 = embedding.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-5, "magnitude={mag}");
    }

    #[test]
    fn text_embedding_different_inputs_produce_different_vectors() {
        let pipeline = TextEmbeddingPipeline::new(128);
        let a = pipeline.embed("hello").unwrap();
        let b = pipeline.embed("world").unwrap();
        let dot: f32 = a.values.iter().zip(&b.values).map(|(x, y)| x * y).sum();
        assert!(dot.abs() < 1.0, "vectors should not be identical");
    }

    #[test]
    fn image_embedding_produces_correct_dimensions() {
        let pipeline = ImageEmbeddingPipeline::new(256);
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let embedding = pipeline.embed(&data).unwrap();
        assert_eq!(embedding.dimensions, 256);
    }

    #[test]
    fn audio_embedding_produces_correct_dimensions() {
        let pipeline = AudioEmbeddingPipeline::new(128);
        let samples: Vec<f32> = (0..500).map(|i| (i as f32).sin()).collect();
        let embedding = pipeline.embed(&samples).unwrap();
        assert_eq!(embedding.dimensions, 128);
    }

    #[test]
    fn sensory_manager_ingest_text() {
        let manager = SensoryManager::new(64);
        let embedding = manager.ingest_text("hello world").unwrap();
        assert_eq!(embedding.dimensions, 64);
    }

    #[test]
    fn sensory_manager_ingest_image() {
        let manager = SensoryManager::new(64);
        let data = vec![128u8; 100];
        let embedding = manager.ingest_image(&data).unwrap();
        assert_eq!(embedding.dimensions, 64);
    }

    #[test]
    fn sensory_manager_ingest_audio() {
        let manager = SensoryManager::new(64);
        let samples = vec![0.5f32; 100];
        let embedding = manager.ingest_audio(&samples).unwrap();
        assert_eq!(embedding.dimensions, 64);
    }

    #[test]
    fn trait_interface_text() {
        let pipeline = TextEmbeddingPipeline::new(32);
        let embedding = pipeline.ingest("test".to_string()).unwrap();
        assert_eq!(embedding.dimensions, 32);
        assert_eq!(pipeline.modality(), SensoryModality::Textual);
        assert_eq!(pipeline.dimensions(), 32);
    }
}
