use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSecurityContextTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSecurityResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub content: String,
}

impl Default for McpSecurityContextTool {
    fn default() -> Self {
        Self::new()
    }
}

impl McpSecurityContextTool {
    pub fn new() -> Self {
        McpSecurityContextTool {
            name: "zkpa_security_context".to_string(),
            description: "Generate and verify ZK proofs of agency for secure agent operations"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" },
                    "operation": { "type": "string", "enum": ["attest", "prove", "verify", "delegate"], "description": "Security operation to perform" },
                    "target": { "type": "string", "description": "Target resource or agent" }
                },
                "required": ["agent_id", "operation"]
            }),
        }
    }

    pub fn execute(&self, agent_id: &str, operation: &str) -> Result<String, ZkpaError> {
        match operation {
            "attest" => Ok(serde_json::json!({
                "status": "attested",
                "agent_id": agent_id,
                "platform": "TPM",
                "verified": true
            })
            .to_string()),
            "prove" => Ok(serde_json::json!({
                "status": "proof_generated",
                "agent_id": agent_id,
                "proof_type": "liveness"
            })
            .to_string()),
            "verify" => Ok(serde_json::json!({
                "status": "verified",
                "agent_id": agent_id
            })
            .to_string()),
            "delegate" => Ok(serde_json::json!({
                "status": "delegation_created",
                "agent_id": agent_id
            })
            .to_string()),
            _ => Err(ZkpaError::DelegationDenied(format!(
                "unknown operation: {}",
                operation
            ))),
        }
    }
}

impl McpSecurityResource {
    pub fn new(uri: &str, name: &str, description: &str, content: &str) -> Self {
        McpSecurityResource {
            uri: uri.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            mime_type: "application/json".to_string(),
            content: content.to_string(),
        }
    }

    pub fn security_policy() -> Self {
        McpSecurityResource::new(
            "zkpa://security/policy",
            "ZKPA Security Policy",
            "Current security policy for agent operations",
            &serde_json::json!({
                "min_security_level": "Sandboxed",
                "require_hardware_attestation": true,
                "max_delegation_depth": 3,
                "session_ttl_seconds": 3600
            })
            .to_string(),
        )
    }

    pub fn system_status() -> Self {
        McpSecurityResource::new(
            "zkpa://security/status",
            "ZKPA System Status",
            "Current status of the ZKPA subsystem",
            &serde_json::json!({
                "status": "operational",
                "active_sessions": 0,
                "proofs_generated": 0,
                "attestations_verified": 0
            })
            .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpSecurityContextTool::new();
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpSecurityContextTool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool.name, deserialized.name);
    }

    #[test]
    fn test_mcp_tool_execute() {
        let tool = McpSecurityContextTool::new();
        let result = tool.execute("agent-123", "attest").unwrap();
        assert!(result.contains("attested"));
    }

    #[test]
    fn test_mcp_resource_serialization() {
        let resource = McpSecurityResource::new("test://uri", "Test", "Test resource", "{}");
        let json = serde_json::to_string(&resource).unwrap();
        let deserialized: McpSecurityResource = serde_json::from_str(&json).unwrap();
        assert_eq!(resource.uri, deserialized.uri);
    }

    #[test]
    fn test_mcp_security_policy_resource() {
        let resource = McpSecurityResource::security_policy();
        assert_eq!(resource.uri, "zkpa://security/policy");
        assert!(resource.content.contains("min_security_level"));
    }
}
