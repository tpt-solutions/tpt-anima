//! MCP Integration — INP Portion.
//!
//! Exposes the Intent & Negotiation Protocol as MCP tools: intent
//! broadcasting, bid query, and result retrieval.

use serde::{Deserialize, Serialize};

use crate::intent::IntentId;

// ---------------------------------------------------------------------------
// MCP Tool definitions
// ---------------------------------------------------------------------------

/// MCP tool for broadcasting an intent to capability providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBroadcastTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for McpBroadcastTool {
    fn default() -> Self {
        Self {
            name: "inp_broadcast_intent".to_string(),
            description:
                "Broadcast an intent to all matching capability providers for negotiation."
                    .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "The action to perform"
                    },
                    "payload": {
                        "type": "object",
                        "description": "Action-specific payload"
                    },
                    "max_cost": {
                        "type": "number",
                        "description": "Maximum compute credits willing to spend"
                    },
                    "max_latency_ms": {
                        "type": "integer",
                        "description": "Maximum acceptable latency in milliseconds"
                    }
                },
                "required": ["action"]
            }),
        }
    }
}

/// MCP tool for querying bid status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpBidQueryTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for McpBidQueryTool {
    fn default() -> Self {
        Self {
            name: "inp_query_bids".to_string(),
            description: "Query the current bids for an active intent negotiation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "intent_id": {
                        "type": "string",
                        "description": "UUID of the intent to query"
                    }
                },
                "required": ["intent_id"]
            }),
        }
    }
}

/// MCP tool for retrieving execution results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResultRetrievalTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl Default for McpResultRetrievalTool {
    fn default() -> Self {
        Self {
            name: "inp_get_result".to_string(),
            description: "Retrieve the execution result for a completed intent.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "intent_id": {
                        "type": "string",
                        "description": "UUID of the intent"
                    }
                },
                "required": ["intent_id"]
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP Resource definitions
// ---------------------------------------------------------------------------

/// MCP resource representing the INP negotiation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNegotiationResource {
    pub uri: String,
    pub name: String,
    pub active_intents: usize,
    pub total_bids: usize,
}

impl McpNegotiationResource {
    pub fn overview(active_intents: usize, total_bids: usize) -> Self {
        Self {
            uri: "mcp://resources/inp/negotiation".to_string(),
            name: "INP Negotiation State".to_string(),
            active_intents,
            total_bids,
        }
    }
}

/// MCP resource representing a specific negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpIntentResource {
    pub uri: String,
    pub name: String,
    pub intent_id: String,
    pub state: String,
}

impl McpIntentResource {
    pub fn for_intent(intent_id: &IntentId, state: &str) -> Self {
        Self {
            uri: format!("mcp://resources/inp/intent/{intent_id}"),
            name: format!("Intent {intent_id}"),
            intent_id: intent_id.to_string(),
            state: state.to_string(),
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
    fn broadcast_tool_has_correct_name() {
        let tool = McpBroadcastTool::default();
        assert_eq!(tool.name, "inp_broadcast_intent");
    }

    #[test]
    fn bid_query_tool_has_correct_name() {
        let tool = McpBidQueryTool::default();
        assert_eq!(tool.name, "inp_query_bids");
    }

    #[test]
    fn result_tool_has_correct_name() {
        let tool = McpResultRetrievalTool::default();
        assert_eq!(tool.name, "inp_get_result");
    }

    #[test]
    fn negotiation_resource() {
        let resource = McpNegotiationResource::overview(5, 23);
        assert_eq!(resource.uri, "mcp://resources/inp/negotiation");
        assert_eq!(resource.active_intents, 5);
        assert_eq!(resource.total_bids, 23);
    }

    #[test]
    fn intent_resource() {
        let id = uuid::Uuid::new_v4();
        let resource = McpIntentResource::for_intent(&id, "CollectingBids");
        assert!(resource.uri.contains(&id.to_string()));
        assert_eq!(resource.state, "CollectingBids");
    }

    #[test]
    fn serialize_tools() {
        let tool = McpBroadcastTool::default();
        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpBroadcastTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, tool.name);
    }
}
