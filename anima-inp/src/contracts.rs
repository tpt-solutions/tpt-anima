//! Semantic Contracts for the Intent & Negotiation Protocol.
//!
//! Self-describing, version-tolerant payload envelopes validated against
//! capability declarations.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SemanticContract
// ---------------------------------------------------------------------------

/// A self-describing payload envelope that replaces rigid JSON schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContract {
    /// SemVer version of this contract format.
    pub schema_version: String,
    /// Unique contract identifier.
    pub contract_id: String,
    /// The action this contract describes.
    pub action: String,
    /// MIME type of the payload.
    pub payload_type: String,
    /// JSON Schema or equivalent describing the payload structure.
    pub payload_schema: serde_json::Value,
    /// The actual payload data.
    pub payload: serde_json::Value,
    /// Required capabilities a provider must have.
    pub required_capabilities: Vec<String>,
    /// Quality constraints encoded in the contract.
    pub qos_requirements: ContractQoS,
}

impl SemanticContract {
    pub fn new(
        action: String,
        payload_type: String,
        payload_schema: serde_json::Value,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            contract_id: uuid::Uuid::new_v4().to_string(),
            action,
            payload_type,
            payload_schema,
            payload,
            required_capabilities: Vec::new(),
            qos_requirements: ContractQoS::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// ContractQoS
// ---------------------------------------------------------------------------

/// QoS requirements encoded in a semantic contract.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractQoS {
    pub max_latency_ms: Option<u64>,
    pub min_accuracy: Option<f32>,
    pub max_cost_credits: Option<f64>,
    pub required_compliance: Vec<String>,
}

// ---------------------------------------------------------------------------
// ContractValidator
// ---------------------------------------------------------------------------

/// Validates contracts against capability declarations.
pub struct ContractValidator;

impl ContractValidator {
    /// Validate that a contract's payload matches its declared schema.
    pub fn validate_payload(contract: &SemanticContract) -> Result<(), ContractError> {
        // Basic structural validation
        if contract.schema_version.is_empty() {
            return Err(ContractError::InvalidSchemaVersion {
                version: contract.schema_version.clone(),
            });
        }

        if contract.payload_type.is_empty() {
            return Err(ContractError::MissingPayloadType);
        }

        // Validate payload against schema if schema is not empty
        if !contract.payload_schema.is_null() {
            Self::validate_against_schema(&contract.payload, &contract.payload_schema)?;
        }

        Ok(())
    }

    /// Validate that a contract's requirements are met by a capability.
    pub fn validate_against_capability(
        contract: &SemanticContract,
        capability_capabilities: &[String],
    ) -> Result<(), ContractError> {
        for required in &contract.required_capabilities {
            if !capability_capabilities.contains(required) {
                return Err(ContractError::CapabilityMismatch {
                    required: required.clone(),
                });
            }
        }
        Ok(())
    }

    /// Simple schema validation (checks required fields exist).
    fn validate_against_schema(
        payload: &serde_json::Value,
        schema: &serde_json::Value,
    ) -> Result<(), ContractError> {
        if let Some(required_fields) = schema.get("required").and_then(|v| v.as_array()) {
            for field in required_fields {
                if let Some(field_name) = field.as_str() {
                    if payload.get(field_name).is_none() {
                        return Err(ContractError::MissingField {
                            field: field_name.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Check backward compatibility between two contract versions.
    pub fn check_compatibility(
        old: &SemanticContract,
        new: &SemanticContract,
    ) -> Result<(), ContractError> {
        if old.action != new.action {
            return Err(ContractError::IncompatibleAction {
                old_action: old.action.clone(),
                new_action: new.action.clone(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ContractEnvelope
// ---------------------------------------------------------------------------

/// A versioned wrapper for contract payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEnvelope {
    pub version: String,
    pub contract: SemanticContract,
    pub signature: Option<String>,
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("invalid schema version: {version}")]
    InvalidSchemaVersion { version: String },

    #[error("missing payload type")]
    MissingPayloadType,

    #[error("missing required field: {field}")]
    MissingField { field: String },

    #[error("capability mismatch: required capability '{required}' not provided")]
    CapabilityMismatch { required: String },

    #[error("incompatible action: old={old_action}, new={new_action}")]
    IncompatibleAction {
        old_action: String,
        new_action: String,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_creation() {
        let contract = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({"type": "object", "required": ["input"]}),
            serde_json::json!({"input": "hello"}),
        );
        assert_eq!(contract.schema_version, "1.0.0");
        assert_eq!(contract.action, "compute");
    }

    #[test]
    fn validate_payload_success() {
        let contract = SemanticContract::new(
            "test".to_string(),
            "text/plain".to_string(),
            serde_json::json!({"type": "object", "required": ["data"]}),
            serde_json::json!({"data": "value"}),
        );
        assert!(ContractValidator::validate_payload(&contract).is_ok());
    }

    #[test]
    fn validate_payload_missing_field() {
        let contract = SemanticContract::new(
            "test".to_string(),
            "text/plain".to_string(),
            serde_json::json!({"type": "object", "required": ["data"]}),
            serde_json::json!({}),
        );
        let result = ContractValidator::validate_payload(&contract);
        assert!(result.is_err());
    }

    #[test]
    fn validate_against_capability_success() {
        let mut contract = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        contract.required_capabilities = vec!["gpu".to_string()];

        let caps = vec!["gpu".to_string(), "cpu".to_string()];
        assert!(ContractValidator::validate_against_capability(&contract, &caps).is_ok());
    }

    #[test]
    fn validate_against_capability_mismatch() {
        let mut contract = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        contract.required_capabilities = vec!["tpu".to_string()];

        let caps = vec!["gpu".to_string(), "cpu".to_string()];
        assert!(ContractValidator::validate_against_capability(&contract, &caps).is_err());
    }

    #[test]
    fn compatibility_check() {
        let old = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        let new = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        assert!(ContractValidator::check_compatibility(&old, &new).is_ok());
    }

    #[test]
    fn incompatible_actions() {
        let old = SemanticContract::new(
            "compute".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        let new = SemanticContract::new(
            "transfer".to_string(),
            "application/json".to_string(),
            serde_json::json!({}),
            serde_json::json!({}),
        );
        assert!(ContractValidator::check_compatibility(&old, &new).is_err());
    }
}
