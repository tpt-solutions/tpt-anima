use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use crate::{MemoryTopographyData, SwarmMapData, TelemetryCollector, TelemetryEvent};

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
pub struct TelemetryServer {
    collector: TelemetryCollector,
    tx: broadcast::Sender<BroadcastMessage>,
}

impl TelemetryServer {
    /// Create a new server with the given broadcast channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            collector: TelemetryCollector::new(),
            tx,
        }
    }

    /// Register a new client and return its receiver.
    pub fn add_client(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.tx.subscribe()
    }

    /// Broadcast a telemetry event to all connected clients.
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

    /// Record an event in the collector and broadcast it.
    pub fn record_and_broadcast(&self, event: TelemetryEvent) {
        self.broadcast_event(event);
    }

    /// Get a reference to the collector.
    pub fn collector(&self) -> &TelemetryCollector {
        &self.collector
    }

    /// Get a mutable reference to the collector.
    pub fn collector_mut(&mut self) -> &mut TelemetryCollector {
        &mut self.collector
    }

    /// Return the number of active subscribers.
    pub fn client_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Handle a single WebSocket connection.
    pub async fn handle_connection(
        stream: TcpStream,
        mut rx: broadcast::Receiver<BroadcastMessage>,
    ) {
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket handshake failed: {}", e);
                return;
            }
        };

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Forward broadcast messages to the client
        let send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                match serde_json::to_string(&msg) {
                    Ok(json) => {
                        if ws_sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Drop client when they disconnect
        let recv_task =
            tokio::spawn(async move { while let Some(Ok(_)) = ws_receiver.next().await {} });

        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
        }
    }

    /// Start the WebSocket server on the given address.
    pub async fn start(self, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Observatory WebSocket server listening on ws://{}", addr);

        let server = std::sync::Arc::new(tokio::sync::Mutex::new(self));

        loop {
            let (stream, peer) = listener.accept().await?;
            println!("New client connected from {}", peer);

            let rx = {
                let s = server.lock().await;
                s.add_client()
            };

            tokio::spawn(Self::handle_connection(stream, rx));
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

        // Non-blocking check
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
}
