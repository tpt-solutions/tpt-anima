use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::sync::{broadcast, Mutex};

use crate::{routes, MemoryTopographyData, SwarmMapData, TelemetryCollector, TelemetryEvent};

/// A message type that can be broadcast to WebSocket clients.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum BroadcastMessage {
    #[serde(rename = "telemetry_event")]
    TelemetryEvent(TelemetryEvent),
    #[serde(rename = "swarm_map")]
    SwarmMap(SwarmMapData),
    #[serde(rename = "memory_topography")]
    MemoryTopography(MemoryTopographyData),
}

/// Server managing WebSocket clients and broadcasting telemetry.
///
/// Cheap to clone: the collector and broadcast channel are shared via `Arc`,
/// so the same handle can be held by the axum router, a background task that
/// drives real subsystem activity, and tests, all pointing at the same state.
#[derive(Clone)]
pub struct TelemetryServer {
    collector: Arc<Mutex<TelemetryCollector>>,
    tx: broadcast::Sender<BroadcastMessage>,
}

impl TelemetryServer {
    /// Create a new server with the given broadcast channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            collector: Arc::new(Mutex::new(TelemetryCollector::new())),
            tx,
        }
    }

    /// Register a new client and return its receiver.
    pub fn add_client(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.tx.subscribe()
    }

    /// Broadcast a telemetry event to all connected clients (does not record it).
    pub fn broadcast_event(&self, event: TelemetryEvent) {
        let _ = self.tx.send(BroadcastMessage::TelemetryEvent(event));
    }

    /// Broadcast a swarm map snapshot to all connected clients.
    pub fn broadcast_swarm_map(&self, data: &SwarmMapData) {
        let _ = self.tx.send(BroadcastMessage::SwarmMap(data.clone()));
    }

    /// Broadcast a memory topography snapshot to all connected clients.
    pub fn broadcast_memory_topography(&self, data: &MemoryTopographyData) {
        let _ = self
            .tx
            .send(BroadcastMessage::MemoryTopography(data.clone()));
    }

    /// Record an event in the collector and broadcast it to connected clients.
    pub async fn record_and_broadcast(&self, event: TelemetryEvent) {
        self.collector.lock().await.record(event.clone());
        self.broadcast_event(event);
    }

    /// Run a closure against the current collector state (used by HTTP handlers
    /// and tests that need synchronous read access to `TelemetryCollector`).
    pub async fn with_collector<T>(&self, f: impl FnOnce(&TelemetryCollector) -> T) -> T {
        let guard = self.collector.lock().await;
        f(&guard)
    }

    /// Return the number of active subscribers.
    pub fn client_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Build the axum router exposing the HTTP + WebSocket API.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/api/health", get(health_handler))
            .route("/api/telemetry/recent", get(recent_handler))
            .route("/api/telemetry/count", get(count_handler))
            .route("/ws/telemetry", get(ws_handler))
            .with_state(self.clone())
    }

    /// Bind and serve the HTTP + WebSocket API on `addr` until the process exits.
    pub async fn start(&self, addr: &str) -> std::io::Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, self.router()).await
    }
}

async fn respond_json(server: &TelemetryServer, path: &str) -> impl IntoResponse {
    let body = server
        .with_collector(|c| routes::handle_request(path, c))
        .await
        .unwrap_or_else(|| "null".to_string());
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
}

async fn health_handler(State(server): State<TelemetryServer>) -> impl IntoResponse {
    respond_json(&server, "/api/health").await
}

async fn recent_handler(State(server): State<TelemetryServer>) -> impl IntoResponse {
    respond_json(&server, "/api/telemetry/recent").await
}

async fn count_handler(State(server): State<TelemetryServer>) -> impl IntoResponse {
    respond_json(&server, "/api/telemetry/count").await
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(server): State<TelemetryServer>,
) -> impl IntoResponse {
    let rx = server.add_client();
    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<BroadcastMessage>) {
    while let Ok(msg) = rx.recv().await {
        match serde_json::to_string(&msg) {
            Ok(json) => {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InpEvent, SseEvent};

    #[test]
    fn server_creates_with_zero_clients() {
        let server = TelemetryServer::new(16);
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn broadcast_event_serializes_correctly() {
        let server = TelemetryServer::new(16);
        let mut rx = server.add_client();

        let event = TelemetryEvent::SSE(SseEvent::MemoryAccessed {
            node_id: "test-node".into(),
            strength: 0.5,
        });
        server.broadcast_event(event.clone());

        let msg = rx.try_recv().expect("should receive broadcast");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("telemetry_event"));
        assert!(json.contains("test-node"));
    }

    #[test]
    fn broadcast_swarm_map_serializes() {
        let server = TelemetryServer::new(16);
        let mut rx = server.add_client();

        let mut map = SwarmMapData::new();
        map.add_node(crate::SwarmNode {
            id: "n1".into(),
            label: "Node1".into(),
            status: "active".into(),
            agent_type: "cognitive".into(),
        });
        server.broadcast_swarm_map(&map);

        let msg = rx.try_recv().expect("should receive swarm map");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("swarm_map"));
    }

    #[test]
    fn broadcast_memory_topography_serializes() {
        let server = TelemetryServer::new(16);
        let mut rx = server.add_client();

        let mut topo = MemoryTopographyData::new();
        topo.add_node(crate::TopoNode {
            id: "m1".into(),
            strength: 0.9,
            modality: "text".into(),
            label: "Fact".into(),
        });
        server.broadcast_memory_topography(&topo);

        let msg = rx.try_recv().expect("should receive topo");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("memory_topography"));
    }

    #[test]
    fn multiple_clients_receive_broadcast() {
        let server = TelemetryServer::new(16);
        let mut rx1 = server.add_client();
        let mut rx2 = server.add_client();
        let mut rx3 = server.add_client();

        assert_eq!(server.client_count(), 3);

        let event = TelemetryEvent::INP(InpEvent::IntentBroadcast {
            intent_id: "i1".into(),
            action: "test".into(),
        });
        server.broadcast_event(event);

        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
        assert!(rx3.try_recv().is_ok());
    }

    #[tokio::test]
    async fn record_and_broadcast_updates_collector_and_clients() {
        let server = TelemetryServer::new(16);
        let mut rx = server.add_client();

        server
            .record_and_broadcast(TelemetryEvent::SSE(SseEvent::MemoryPruned { count: 2 }))
            .await;

        assert!(rx.try_recv().is_ok());
        let count = server.with_collector(|c| c.events().len()).await;
        assert_eq!(count, 1);
    }
}
