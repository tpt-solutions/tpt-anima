use crate::TelemetryCollector;

/// Handle an HTTP request path and return a JSON response body.
///
/// Returns `None` for unknown paths.
pub fn handle_request(path: &str, collector: &TelemetryCollector) -> Option<String> {
    match path {
        "/api/telemetry/recent" => {
            let events = collector.recent_events(100);
            Some(serde_json::to_string(&events).unwrap_or_else(|_| "[]".into()))
        }
        "/api/telemetry/count" => {
            let counts = collector.count_by_layer();
            Some(serde_json::to_string(&counts).unwrap_or_else(|_| "{}".into()))
        }
        "/api/health" => Some(r#"{"status":"ok"}"#.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SseEvent, TelemetryEvent};

    #[test]
    fn health_endpoint_returns_ok() {
        let collector = TelemetryCollector::new();
        let result = handle_request("/api/health", &collector);
        assert!(result.is_some());
        let body = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["status"], "ok");
    }

    #[test]
    fn recent_events_endpoint_returns_json_array() {
        let mut collector = TelemetryCollector::new();
        collector.record(TelemetryEvent::SSE(SseEvent::MemoryAccessed {
            node_id: "n1".into(),
            strength: 0.8,
        }));
        collector.record(TelemetryEvent::SSE(SseEvent::MemoryPruned { count: 2 }));

        let result = handle_request("/api/telemetry/recent", &collector).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn count_endpoint_returns_layer_counts() {
        let mut collector = TelemetryCollector::new();
        collector.record(TelemetryEvent::SSE(SseEvent::MemoryAccessed {
            node_id: "n1".into(),
            strength: 0.8,
        }));
        collector.record(TelemetryEvent::SSE(SseEvent::MemoryPruned { count: 1 }));

        let result = handle_request("/api/telemetry/count", &collector).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["sse"], 2);
        assert_eq!(parsed["inp"], 0);
    }

    #[test]
    fn unknown_path_returns_none() {
        let collector = TelemetryCollector::new();
        assert!(handle_request("/unknown", &collector).is_none());
        assert!(handle_request("/", &collector).is_none());
        assert!(handle_request("/api/telemetry/unknown", &collector).is_none());
    }

    #[test]
    fn empty_collector_returns_empty_arrays() {
        let collector = TelemetryCollector::new();
        let result = handle_request("/api/telemetry/recent", &collector).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.as_array().unwrap().is_empty());
    }
}
