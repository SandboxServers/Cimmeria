use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

// ── Log broadcast infrastructure ──

const BUFFER_CAPACITY: usize = 500;

#[derive(Clone, Debug, Serialize)]
struct LogEntry {
    timestamp_ms: u64,
    level: String,
    target: String,
    message: String,
    fields: serde_json::Value,
}

#[derive(Clone)]
struct LogBuffer {
    inner: Arc<StdMutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    fn new() -> Self {
        Self {
            inner: Arc::new(StdMutex::new(VecDeque::with_capacity(BUFFER_CAPACITY))),
        }
    }

    fn push(&self, entry: LogEntry) {
        if let Ok(mut buf) = self.inner.lock() {
            if buf.len() >= BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }

    fn snapshot(&self) -> Vec<LogEntry> {
        self.inner
            .lock()
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}

struct BroadcastLayer {
    tx: broadcast::Sender<LogEntry>,
    buffer: LogBuffer,
}

impl BroadcastLayer {
    fn new(tx: broadcast::Sender<LogEntry>, buffer: LogBuffer) -> Self {
        Self { tx, buffer }
    }
}

impl<S> tracing_subscriber::Layer<S> for BroadcastLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let entry = LogEntry {
            timestamp_ms: now,
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message,
            fields: serde_json::Value::Object(visitor.fields),
        };

        self.buffer.push(entry.clone());
        let _ = self.tx.send(entry);
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(format!("{:?}", value)),
            );
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

// ── WebSocket log stream handler ──

async fn log_ws_handler(
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
    // Replay buffered history.
    for entry in log_buffer.snapshot() {
        let json = match serde_json::to_string(&entry) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            return;
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

// ── Supervisor state and handlers ──

#[derive(Serialize)]
struct StatusResponse {
    process_running: bool,
    building: bool,
    build_output: String,
    last_exit_code: Option<i32>,
    services: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ActionResponse {
    status: String,
    message: String,
}

struct SupervisorState {
    child: Mutex<Option<Child>>,
    building: AtomicBool,
    build_output: Mutex<String>,
    last_exit_code: Mutex<Option<i32>>,
    server_binary: String,
    server_port: u16,
}

impl SupervisorState {
    fn is_building(&self) -> bool {
        self.building.load(Ordering::Relaxed)
    }
}

/// Spawn the game server binary as a child process.
async fn spawn_server(state: &SupervisorState) -> Result<(), String> {
    let mut guard = state.child.lock().await;
    if let Some(ref mut child) = *guard {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => return Err("Server is already running".into()),
            Err(e) => return Err(format!("Failed to check child status: {e}")),
        }
    }

    let child = Command::new(&state.server_binary)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn server: {e}"))?;

    tracing::info!(pid = child.id(), binary = %state.server_binary, "Spawned game server");
    *guard = Some(child);
    *state.last_exit_code.lock().await = None;
    Ok(())
}

/// Kill the running child process.
async fn kill_server(state: &SupervisorState) -> Result<(), String> {
    let mut guard = state.child.lock().await;
    if let Some(ref mut child) = *guard {
        match child.try_wait() {
            Ok(Some(exit)) => {
                *state.last_exit_code.lock().await = exit.code();
                *guard = None;
                return Ok(());
            }
            Ok(None) => {
                child.kill().await.map_err(|e| format!("Failed to kill server: {e}"))?;
                let exit = child
                    .wait()
                    .await
                    .map_err(|e| format!("Failed to wait for exit: {e}"))?;
                tracing::info!(exit_code = ?exit.code(), "Game server stopped");
                *state.last_exit_code.lock().await = exit.code();
                *guard = None;
                Ok(())
            }
            Err(e) => Err(format!("Failed to check child status: {e}")),
        }
    } else {
        Ok(())
    }
}

/// Check if the child process is currently running.
async fn is_running(state: &SupervisorState) -> bool {
    let mut guard = state.child.lock().await;
    if let Some(ref mut child) = *guard {
        match child.try_wait() {
            Ok(Some(exit)) => {
                let code = exit.code();
                *guard = None;
                drop(guard);
                *state.last_exit_code.lock().await = code;
                false
            }
            Ok(None) => true,
            Err(_) => false,
        }
    } else {
        false
    }
}

/// Fetch service health from the game server's admin API via raw HTTP.
async fn fetch_services(port: u16) -> Option<serde_json::Value> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let timeout = std::time::Duration::from_secs(2);
    let stream = match tokio::time::timeout(timeout, TcpStream::connect(format!("127.0.0.1:{port}"))).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::debug!("Health proxy: connect failed: {e}");
            return None;
        }
        Err(_) => {
            tracing::debug!("Health proxy: connect timed out");
            return None;
        }
    };

    let request = format!("GET /api/config/status HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    let mut stream = stream;
    if let Err(e) = stream.write_all(request.as_bytes()).await {
        tracing::debug!("Health proxy: write failed: {e}");
        return None;
    }

    let mut buf = Vec::with_capacity(4096);
    if let Err(e) = stream.read_to_end(&mut buf).await {
        tracing::debug!("Health proxy: read failed: {e}");
        return None;
    }

    let response = String::from_utf8_lossy(&buf);
    tracing::debug!("Health proxy: raw response ({} bytes):\n{response}", buf.len());

    let body = response.split("\r\n\r\n").nth(1)?;
    let json: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Health proxy: JSON parse failed: {e}");
            return None;
        }
    };
    json.get("services").cloned()
}

// ── Handlers ──

async fn get_status(State(state): State<Arc<SupervisorState>>) -> Json<StatusResponse> {
    let running = is_running(&state).await;
    let services = if running {
        fetch_services(state.server_port).await
    } else {
        None
    };

    Json(StatusResponse {
        process_running: running,
        building: state.is_building(),
        build_output: state.build_output.lock().await.clone(),
        last_exit_code: *state.last_exit_code.lock().await,
        services,
    })
}

async fn post_start(State(state): State<Arc<SupervisorState>>) -> Json<ActionResponse> {
    if state.is_building() {
        return Json(ActionResponse {
            status: "error".into(),
            message: "Cannot start while a build is in progress".into(),
        });
    }

    match spawn_server(&state).await {
        Ok(()) => Json(ActionResponse {
            status: "ok".into(),
            message: "Server started".into(),
        }),
        Err(e) => Json(ActionResponse {
            status: "error".into(),
            message: e,
        }),
    }
}

async fn post_stop(State(state): State<Arc<SupervisorState>>) -> Json<ActionResponse> {
    match kill_server(&state).await {
        Ok(()) => Json(ActionResponse {
            status: "ok".into(),
            message: "Server stopped".into(),
        }),
        Err(e) => Json(ActionResponse {
            status: "error".into(),
            message: e,
        }),
    }
}

async fn post_rebuild(State(state): State<Arc<SupervisorState>>) -> Json<ActionResponse> {
    if state.is_building() {
        return Json(ActionResponse {
            status: "error".into(),
            message: "Build already in progress".into(),
        });
    }

    // 1. Stop current server
    if let Err(e) = kill_server(&state).await {
        return Json(ActionResponse {
            status: "error".into(),
            message: format!("Failed to stop server before rebuild: {e}"),
        });
    }

    // 2. Build
    state.building.store(true, Ordering::Relaxed);
    *state.build_output.lock().await = String::new();

    tracing::info!("Starting rebuild: cargo build -p cimmeria-server");

    let build_result = Command::new("cargo")
        .args(["build", "-p", "cimmeria-server"])
        .output()
        .await;

    state.building.store(false, Ordering::Relaxed);

    match build_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            *state.build_output.lock().await = combined.clone();

            if output.status.success() {
                tracing::info!("Build succeeded, starting server");
                match spawn_server(&state).await {
                    Ok(()) => Json(ActionResponse {
                        status: "ok".into(),
                        message: "Rebuild succeeded, server started".into(),
                    }),
                    Err(e) => Json(ActionResponse {
                        status: "error".into(),
                        message: format!("Build succeeded but failed to start: {e}"),
                    }),
                }
            } else {
                tracing::error!("Build failed");
                Json(ActionResponse {
                    status: "error".into(),
                    message: format!("Build failed:\n{combined}"),
                })
            }
        }
        Err(e) => {
            let msg = format!("Failed to run cargo build: {e}");
            *state.build_output.lock().await = msg.clone();
            Json(ActionResponse {
                status: "error".into(),
                message: msg,
            })
        }
    }
}

#[tokio::main]
async fn main() {
    // Set up broadcast channel and ring buffer for log streaming.
    let (log_tx, _) = broadcast::channel::<LogEntry>(2048);
    let log_buffer = LogBuffer::new();

    // Build tracing subscriber with broadcast layer.
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,tower_http=debug".into());

    let fmt_layer = tracing_subscriber::fmt::layer();

    let broadcast = BroadcastLayer::new(log_tx.clone(), log_buffer.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(broadcast)
        .init();

    let port: u16 = std::env::var("SUPERVISOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8444);

    let server_binary = std::env::var("SERVER_BINARY").unwrap_or_else(|_| {
        if cfg!(windows) {
            r".\target\debug\cimmeria-server.exe".into()
        } else {
            "./target/debug/cimmeria-server".into()
        }
    });

    let server_port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8443);

    let state = Arc::new(SupervisorState {
        child: Mutex::new(None),
        building: AtomicBool::new(false),
        build_output: Mutex::new(String::new()),
        last_exit_code: Mutex::new(None),
        server_binary,
        server_port,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/supervisor/status", get(get_status))
        .route("/supervisor/start", post(post_start))
        .route("/supervisor/stop", post(post_stop))
        .route("/supervisor/rebuild", post(post_rebuild))
        .route("/supervisor/ws/logs", get(log_ws_handler))
        .layer(Extension(log_tx))
        .layer(Extension(log_buffer))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!(%addr, "Supervisor listening");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
