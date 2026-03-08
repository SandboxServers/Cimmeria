//! Server log WebSocket stream.
//!
//! Streams real-time server log output to connected admin clients via a
//! `broadcast` channel fed by the [`BroadcastLayer`](super::broadcast_layer::BroadcastLayer).

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use tokio::sync::broadcast;

use super::broadcast_layer::LogEntry;

/// WebSocket handler for log streams.
///
/// Subscribes to the shared broadcast channel and forwards every log entry
/// to the connected WebSocket client as a JSON text frame.
pub async fn log_ws_handler(
    ws: WebSocketUpgrade,
    Extension(log_tx): Extension<broadcast::Sender<LogEntry>>,
) -> Response {
    let log_rx = log_tx.subscribe();
    ws.on_upgrade(move |socket| handle_log_socket(socket, log_rx))
}

async fn handle_log_socket(mut socket: WebSocket, mut log_rx: broadcast::Receiver<LogEntry>) {
    // Do not use tracing macros here — the BroadcastLayer would re-enter.
    loop {
        tokio::select! {
            // Forward log entries to the client
            result = log_rx.recv() => {
                match result {
                    Ok(entry) => {
                        let json = match serde_json::to_string(&entry) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client fell behind — send a warning and continue
                        let msg = serde_json::json!({
                            "type": "lagged",
                            "skipped": n,
                        });
                        let _ = socket.send(Message::Text(msg.to_string().into())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Server shutting down
                    }
                }
            }
            // Handle incoming messages from the client (e.g. close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // Ignore other client messages for now
                }
            }
        }
    }
}
