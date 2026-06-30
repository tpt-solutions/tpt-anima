//! MCP Integration — SSE Portion.
//!
//! Exposes the Semantic State Engine as MCP resources. Agents "attend" to
//! memory rather than querying it, following the SSE philosophy.

use serde::{Deserialize, Serialize};

use crate::memory_node::MemoryId;

// ---------------------------------------------------------------------------
// MCP Resource definitions
// ---------------------------------------------------------------------------

/// An MCP resource representing a single memory node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMemoryResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// An MCP resource representing the memory substrate overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSubstrateResource {
    pub uri: String,
    pub name: String,
    pub total_nodes: usize,
    pub avg_strength: f32,
    pub modalities: Vec<String>,
}

/// An MCP resource representing a causal chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCausalChainResource {
    pub uri: String,
    pub name: String,
    pub chain_length: usize,
    pub root_node_id: String,
}

// ---------------------------------------------------------------------------
// MCP resource constructors
// ---------------------------------------------------------------------------

impl McpMemoryResource {
    /// Create an MCP resource descriptor for a memory node.
    pub fn for_node(node_id: &MemoryId, label: Option<&str>, mime: &str) -> Self {
        let hex_id = hex::encode(node_id);
        Self {
            uri: format!("mcp://resources/sse/node/{hex_id}"),
            name: label.unwrap_or("unnamed memory").to_string(),
            description: format!("Memory node {hex_id}"),
            mime_type: mime.to_string(),
        }
    }
}

impl McpSubstrateResource {
    /// Create an MCP resource descriptor for the entire SSE substrate.
    pub fn overview(total_nodes: usize, avg_strength: f32, modalities: Vec<String>) -> Self {
        Self {
            uri: "mcp://resources/sse/substrate".to_string(),
            name: "SSE Memory Substrate".to_string(),
            total_nodes,
            avg_strength,
            modalities,
        }
    }
}

impl McpCausalChainResource {
    /// Create an MCP resource descriptor for a causal chain.
    pub fn for_chain(root_id: &MemoryId, chain_length: usize) -> Self {
        let hex_id = hex::encode(root_id);
        Self {
            uri: format!("mcp://resources/sse/causal/{hex_id}"),
            name: format!("Causal chain from {hex_id}"),
            chain_length,
            root_node_id: hex_id,
        }
    }
}

// ---------------------------------------------------------------------------
// MCP Tool definitions
// ---------------------------------------------------------------------------

/// An MCP tool that allows agents to attend to the memory substrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAttendTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for McpAttendTool {
    fn default() -> Self {
        Self {
            name: "sse_attend".to_string(),
            description: "Attend to the memory substrate. Returns the strongest (most relevant) memory nodes, not query results.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of nodes to attend to",
                        "default": 10
                    },
                    "min_strength": {
                        "type": "number",
                        "description": "Minimum strength threshold",
                        "default": 0.0
                    }
                }
            }),
        }
    }
}

/// An MCP tool for attending to a specific node's context chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContextChainTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for McpContextChainTool {
    fn default() -> Self {
        Self {
            name: "sse_context_chain".to_string(),
            description: "Retrieve the causal context chain for a memory node. Follows antecedent links to provide full context.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node_id": {
                        "type": "string",
                        "description": "Hex-encoded memory node ID"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum chain depth",
                        "default": 5
                    }
                },
                "required": ["node_id"]
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_memory_resource_uri_format() {
        let id = [42u8; 32];
        let resource = McpMemoryResource::for_node(&id, Some("test"), "text/plain");
        assert!(resource.uri.starts_with("mcp://resources/sse/node/"));
        assert_eq!(resource.name, "test");
        assert_eq!(resource.mime_type, "text/plain");
    }

    #[test]
    fn mcp_substrate_resource() {
        let resource = McpSubstrateResource::overview(100, 0.75, vec!["text".to_string()]);
        assert_eq!(resource.uri, "mcp://resources/sse/substrate");
        assert_eq!(resource.total_nodes, 100);
    }

    #[test]
    fn mcp_causal_chain_resource() {
        let id = [1u8; 32];
        let resource = McpCausalChainResource::for_chain(&id, 5);
        assert!(resource.uri.starts_with("mcp://resources/sse/causal/"));
        assert_eq!(resource.chain_length, 5);
    }

    #[test]
    fn attend_tool_has_correct_name() {
        let tool = McpAttendTool::default();
        assert_eq!(tool.name, "sse_attend");
    }

    #[test]
    fn context_chain_tool_has_correct_name() {
        let tool = McpContextChainTool::default();
        assert_eq!(tool.name, "sse_context_chain");
    }

    #[test]
    fn serialize_deserialize_resource() {
        let resource = McpMemoryResource::for_node(&[1u8; 32], Some("test"), "text/plain");
        let json = serde_json::to_string(&resource).unwrap();
        let deserialized: McpMemoryResource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uri, resource.uri);
    }
}
