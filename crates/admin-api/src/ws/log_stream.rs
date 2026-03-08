//! Server log WebSocket stream.
//!
//! Streams real-time server log output to connected admin clients via a
//! `broadcast` channel fed by the [`BroadcastLayer`](super::broadcast_layer::BroadcastLayer).
//!
//! On connect, the handler replays recent history from the [`LogBuffer`] ring
//! buffer so clients see startup logs even if they connected late.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use tokio::sync::broadcast;

use super::broadcast_layer::{LogBuffer, LogEntry};

/// WebSocket handler for log streams.
///
/// Subscribes to the shared broadcast channel and forwards every log entry
/// to the connected WebSocket client as a JSON text frame. On initial connect,
/// replays the ring buffer so the client receives recent history.
pub async fn log_ws_handler(
    ws: WebSocketUpgrade,
    Extension(log_tx): Extension<broadcast::Sender<LogEntry>>,
    Extension(log_buffer): Extension<LogBuffer>,
) -> Response {
    let log_rx = log_tx.subscribe();
    ws.on_upgrade(move |socket| handle_log_socket(socket, log_rx, log_buffer))
}

async fn handle_log_socket(
    mut socket: WebSocket,
    mut log_rx: broadcast::Receiver<LogEntry>,
    log_buffer: LogBuffer,
) {
    // Do not use tracing macros here — the BroadcastLayer would re-enter.

    // Replay buffered history so the client sees startup logs.
    for entry in log_buffer.snapshot() {
        let json = match serde_json::to_string(&entry) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            return; // Client disconnected during replay
        }
    }

    // Stream live entries.
    loop {
        tokio::select! {
            result = log_rx.recv() => {
                match result {
                    Ok(entry) => {
                        let json = match serde_json::to_string(&entry) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let msg = serde_json::json!({
                            "type": "lagged",
                            "skipped": n,
                        });
                        let _ = socket.send(Message::Text(msg.to_string().into())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
