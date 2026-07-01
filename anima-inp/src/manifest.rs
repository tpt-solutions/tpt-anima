//! Capability Manifest parser — human-readable agent capability declarations.
//!
//! A manifest describes what an agent can do, its performance characteristics,
//! pricing model, and hardware requirements. This module parses JSON manifests
//! and converts them into INP [`Capability`] objects for negotiation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::intent::{Capability, EconomicTerms, PerformanceProfile};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors that can occur during manifest parsing or validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ManifestError {
    #[error("parse error: {0}")]
    ParseError(String),

    #[error("validation error: {0}")]
    ValidationError(String),

    #[error("missing field: {0}")]
    MissingField(String),
}

// ---------------------------------------------------------------------------
// Manifest data structures
// ---------------------------------------------------------------------------

/// A capability manifest describes an agent's published capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<ManifestCapability>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// A single capability declared within a manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCapability {
    pub action: String,
    pub description: String,
    pub performance: ManifestPerformance,
    pub pricing: ManifestPricing,
    #[serde(default)]
    pub required_hardware: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Performance characteristics of a manifest capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPerformance {
    pub typical_latency_ms: u64,
    pub max_latency_ms: u64,
    pub accuracy: f32,
}

/// Pricing model for a manifest capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ManifestPricing {
    #[serde(rename = "fixed")]
    FixedPrice { credits: f64 },
    #[serde(rename = "per_unit")]
    PerUnit { credits_per_unit: f64, unit: String },
    #[serde(rename = "auction")]
    Auction { max_budget: f64 },
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parses and validates capability manifests.
pub struct ManifestParser;

impl ManifestParser {
    /// Parse a JSON string into a [`CapabilityManifest`].
    pub fn parse(json: &str) -> Result<CapabilityManifest, ManifestError> {
        serde_json::from_str(json).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Validate a manifest's structural integrity.
    ///
    /// Checks:
    /// - `name` is non-empty
    /// - `capabilities` is non-empty
    /// - pricing values are positive
    pub fn validate(manifest: &CapabilityManifest) -> Result<(), ManifestError> {
        if manifest.name.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "manifest name must be non-empty".into(),
            ));
        }
        if manifest.capabilities.is_empty() {
            return Err(ManifestError::ValidationError(
                "manifest must declare at least one capability".into(),
            ));
        }
        for (i, cap) in manifest.capabilities.iter().enumerate() {
            if cap.action.trim().is_empty() {
                return Err(ManifestError::ValidationError(format!(
                    "capability[{i}] action must be non-empty"
                )));
            }
            if cap.performance.typical_latency_ms == 0 {
                return Err(ManifestError::ValidationError(format!(
                    "capability[{i}] typical_latency_ms must be > 0"
                )));
            }
            if cap.performance.max_latency_ms < cap.performance.typical_latency_ms {
                return Err(ManifestError::ValidationError(format!(
                    "capability[{i}] max_latency_ms must be >= typical_latency_ms"
                )));
            }
            if !(0.0..=1.0).contains(&cap.performance.accuracy) {
                return Err(ManifestError::ValidationError(format!(
                    "capability[{i}] accuracy must be between 0.0 and 1.0"
                )));
            }
            match &cap.pricing {
                ManifestPricing::FixedPrice { credits } => {
                    if *credits < 0.0 {
                        return Err(ManifestError::ValidationError(format!(
                            "capability[{i}] fixed price credits must be >= 0"
                        )));
                    }
                }
                ManifestPricing::PerUnit {
                    credits_per_unit, ..
                } => {
                    if *credits_per_unit < 0.0 {
                        return Err(ManifestError::ValidationError(format!(
                            "capability[{i}] per-unit credits must be >= 0"
                        )));
                    }
                }
                ManifestPricing::Auction { max_budget } => {
                    if *max_budget <= 0.0 {
                        return Err(ManifestError::ValidationError(format!(
                            "capability[{i}] auction max_budget must be > 0"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Convert a manifest into INP [`Capability`] objects.
    ///
    /// Each manifest capability becomes one INP capability, all assigned to
    /// the given `provider_id`.
    pub fn to_capabilities(manifest: &CapabilityManifest, provider_id: Uuid) -> Vec<Capability> {
        manifest
            .capabilities
            .iter()
            .map(|cap| {
                let perf = PerformanceProfile {
                    typical_latency: std::time::Duration::from_millis(
                        cap.performance.typical_latency_ms,
                    ),
                    max_latency: std::time::Duration::from_millis(cap.performance.max_latency_ms),
                    typical_accuracy: cap.performance.accuracy,
                    throughput_per_second: 1.0,
                };
                let pricing = match &cap.pricing {
                    ManifestPricing::FixedPrice { credits } => {
                        EconomicTerms::FixedPrice { credits: *credits }
                    }
                    ManifestPricing::PerUnit {
                        credits_per_unit,
                        unit,
                    } => EconomicTerms::PerUnit {
                        credits_per_unit: *credits_per_unit,
                        unit: unit.clone(),
                    },
                    ManifestPricing::Auction { max_budget } => EconomicTerms::Auction {
                        max_budget: *max_budget,
                    },
                };
                Capability::new(vec![cap.action.clone()], perf, pricing, provider_id)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> &'static str {
        r#"{
            "name": "data-processor",
            "version": "1.0.0",
            "description": "High-speed data processing agent",
            "capabilities": [
                {
                    "action": "data_transform",
                    "description": "Transform data between formats",
                    "performance": {
                        "typical_latency_ms": 100,
                        "max_latency_ms": 500,
                        "accuracy": 0.99
                    },
                    "pricing": {
                        "type": "per_unit",
                        "credits_per_unit": 0.01,
                        "unit": "record"
                    },
                    "required_hardware": ["gpu"],
                    "tags": ["etl", "transform"]
                }
            ],
            "metadata": {
                "author": "agent-team-alpha",
                "license": "Apache-2.0"
            }
        }"#
    }

    #[test]
    fn parse_valid_manifest() {
        let manifest = ManifestParser::parse(sample_manifest()).unwrap();
        assert_eq!(manifest.name, "data-processor");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.capabilities.len(), 1);
        assert_eq!(manifest.capabilities[0].action, "data_transform");
        assert_eq!(manifest.metadata.get("author").unwrap(), "agent-team-alpha");
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = ManifestParser::parse("{not valid json");
        assert!(result.is_err());
        match result.unwrap_err() {
            ManifestError::ParseError(_) => {}
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.name = "".to_string();
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn validate_rejects_empty_capabilities() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.capabilities.clear();
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn validate_rejects_zero_typical_latency() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.capabilities[0].performance.typical_latency_ms = 0;
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn validate_rejects_max_lt_typical() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.capabilities[0].performance.typical_latency_ms = 500;
        manifest.capabilities[0].performance.max_latency_ms = 100;
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn validate_rejects_out_of_range_accuracy() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.capabilities[0].performance.accuracy = 1.5;
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn validate_rejects_negative_pricing() {
        let mut manifest: CapabilityManifest = serde_json::from_str(sample_manifest()).unwrap();
        manifest.capabilities[0].pricing = ManifestPricing::FixedPrice { credits: -5.0 };
        assert!(ManifestParser::validate(&manifest).is_err());
    }

    #[test]
    fn to_capabilities_produces_correct_types() {
        let manifest = ManifestParser::parse(sample_manifest()).unwrap();
        let provider = Uuid::new_v4();
        let caps = ManifestParser::to_capabilities(&manifest, provider);
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].action_types, vec!["data_transform"]);
        assert_eq!(caps[0].provider_agent_id, provider);
        assert_eq!(
            caps[0].performance_profile.typical_latency,
            std::time::Duration::from_millis(100)
        );
        assert_eq!(
            caps[0].performance_profile.max_latency,
            std::time::Duration::from_millis(500)
        );
    }

    #[test]
    fn fixed_price_pricing_roundtrip() {
        let manifest_json = r#"{
            "name": "svc",
            "version": "0.1",
            "description": "test",
            "capabilities": [{
                "action": "do_thing",
                "description": "does a thing",
                "performance": { "typical_latency_ms": 10, "max_latency_ms": 50, "accuracy": 0.9 },
                "pricing": { "type": "fixed", "credits": 2.5 }
            }]
        }"#;
        let manifest = ManifestParser::parse(manifest_json).unwrap();
        let caps = ManifestParser::to_capabilities(&manifest, Uuid::new_v4());
        match &caps[0].pricing {
            EconomicTerms::FixedPrice { credits } => assert_eq!(*credits, 2.5),
            other => panic!("expected FixedPrice, got {other:?}"),
        }
    }

    #[test]
    fn auction_pricing_roundtrip() {
        let manifest_json = r#"{
            "name": "svc",
            "version": "0.1",
            "description": "test",
            "capabilities": [{
                "action": "do_thing",
                "description": "does a thing",
                "performance": { "typical_latency_ms": 10, "max_latency_ms": 50, "accuracy": 0.9 },
                "pricing": { "type": "auction", "max_budget": 100.0 }
            }]
        }"#;
        let manifest = ManifestParser::parse(manifest_json).unwrap();
        let caps = ManifestParser::to_capabilities(&manifest, Uuid::new_v4());
        match &caps[0].pricing {
            EconomicTerms::Auction { max_budget } => assert_eq!(*max_budget, 100.0),
            other => panic!("expected Auction, got {other:?}"),
        }
    }

    #[test]
    fn metadata_and_tags_preserved() {
        let manifest = ManifestParser::parse(sample_manifest()).unwrap();
        assert_eq!(manifest.metadata.len(), 2);
        assert_eq!(manifest.capabilities[0].tags, vec!["etl", "transform"]);
        assert_eq!(manifest.capabilities[0].required_hardware, vec!["gpu"]);
    }
}
