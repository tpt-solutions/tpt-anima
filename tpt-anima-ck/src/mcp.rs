//! MCP (Model Context Protocol) integration for the Cognitive Kernel.

#[allow(unused_imports)]
use crate::types::*;
use serde::{Deserialize, Serialize};

// ── MCP Compute Allocation Tool ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpComputeAllocationTool {
    pub name: String,
    pub description: String,
    pub input_schema: McpSchema,
}

impl McpComputeAllocationTool {
    pub fn new() -> Self {
        Self {
            name: "compute_allocation".into(),
            description: "Query or modify the current compute allocation across CPU/GPU/NPU".into(),
            input_schema: McpSchema {
                schema_type: "object".into(),
                properties: vec![McpSchemaProperty {
                    name: "unit".into(),
                    description: "Compute unit to query: cpu, gpu, or npu".into(),
                    schema_type: "string".into(),
                }],
            },
        }
    }
}

impl Default for McpComputeAllocationTool {
    fn default() -> Self {
        Self::new()
    }
}

// ── MCP Inference Status Tool ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInferenceStatusTool {
    pub name: String,
    pub description: String,
    pub input_schema: McpSchema,
}

impl McpInferenceStatusTool {
    pub fn new() -> Self {
        Self {
            name: "inference_status".into(),
            description: "Get the status and health metrics of a running inference".into(),
            input_schema: McpSchema {
                schema_type: "object".into(),
                properties: vec![McpSchemaProperty {
                    name: "inference_id".into(),
                    description: "UUID of the inference to inspect".into(),
                    schema_type: "string".into(),
                }],
            },
        }
    }
}

impl Default for McpInferenceStatusTool {
    fn default() -> Self {
        Self::new()
    }
}

// ── MCP Compute Resource ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpComputeResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

impl McpComputeResource {
    pub fn new() -> Self {
        Self {
            uri: "anima://compute/status".into(),
            name: "compute_status".into(),
            description: "Real-time compute utilization and thermal metrics".into(),
            mime_type: "application/json".into(),
        }
    }
}

impl Default for McpComputeResource {
    fn default() -> Self {
        Self::new()
    }
}

// ── Schema types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Vec<McpSchemaProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchemaProperty {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub schema_type: String,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_allocation_tool_defaults() {
        let tool = McpComputeAllocationTool::new();
        assert_eq!(tool.name, "compute_allocation");
        assert!(!tool.description.is_empty());
        assert_eq!(tool.input_schema.schema_type, "object");
        assert_eq!(tool.input_schema.properties.len(), 1);
    }

    #[test]
    fn inference_status_tool_defaults() {
        let tool = McpInferenceStatusTool::new();
        assert_eq!(tool.name, "inference_status");
        assert!(!tool.description.is_empty());
        assert_eq!(tool.input_schema.properties[0].name, "inference_id");
    }

    #[test]
    fn compute_resource_defaults() {
        let resource = McpComputeResource::new();
        assert_eq!(resource.uri, "anima://compute/status");
        assert_eq!(resource.mime_type, "application/json");
    }

    #[test]
    fn mcp_tool_serialization_roundtrip() {
        let tool = McpComputeAllocationTool::new();
        let json = serde_json::to_string(&tool).unwrap();
        let back: McpComputeAllocationTool = serde_json::from_str(&json).unwrap();
        assert_eq!(tool.name, back.name);
        assert_eq!(tool.description, back.description);
    }

    #[test]
    fn mcp_resource_serialization_roundtrip() {
        let resource = McpComputeResource::new();
        let json = serde_json::to_string(&resource).unwrap();
        let back: McpComputeResource = serde_json::from_str(&json).unwrap();
        assert_eq!(resource.uri, back.uri);
        assert_eq!(resource.name, back.name);
        assert_eq!(resource.mime_type, back.mime_type);
    }

    #[test]
    fn mcp_schema_property_serialization() {
        let prop = McpSchemaProperty {
            name: "test".into(),
            description: "a test prop".into(),
            schema_type: "string".into(),
        };
        let json = serde_json::to_string(&prop).unwrap();
        let back: McpSchemaProperty = serde_json::from_str(&json).unwrap();
        assert_eq!(prop.name, back.name);
    }
}
