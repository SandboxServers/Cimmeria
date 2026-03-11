//! Game event WebSocket stream.
//!
//! Streams real-time login audit events to connected admin clients.
//! On connect, replays recent history from the [`LoginEventBuffer`] ring
//! buffer, then streams live events from the broadcast channel.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::Extension;
use tokio::sync::broadcast;

use cimmeria_services::audit::{LoginEvent, LoginEventBuffer};

/// WebSocket handler for game event streams.
///
/// Upgrades the HTTP connection to a WebSocket and begins streaming
/// login audit events as JSON text frames.
pub async fn event_ws_handler(
    ws: WebSocketUpgrade,
    Extension(login_tx): Extension<broadcast::Sender<LoginEvent>>,
    Extension(login_buffer): Extension<LoginEventBuffer>,
) -> Response {
    let login_rx = login_tx.subscribe();
    ws.on_upgrade(move |socket| handle_event_socket(socket, login_rx, login_buffer))
}

async fn handle_event_socket(
    mut socket: WebSocket,
    mut login_rx: broadcast::Receiver<LoginEvent>,
    login_buffer: LoginEventBuffer,
) {
    // Replay buffered history so the client sees recent events.
    for event in login_buffer.snapshot() {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // Stream live events.
    loop {
        tokio::select! {
            result = login_rx.recv() => {
                match result {
                    Ok(event) => {
                        let json = match serde_json::to_string(&event) {
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
