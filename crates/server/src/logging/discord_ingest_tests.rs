//! The telemetry ingest's credential refusals never post to Discord, end
//! to end: a real request to the login-port upload route, refused for its
//! missing token, writes its `RefusalLog` row through the real
//! `DiscordLayer`, and nothing is posted. This ties the `event` the
//! admin-api writes to the `SIGNOZ_ONLY_EVENTS` row the discord crate
//! matches (2026-10-12: an expired-token retrier posted 1,363 rows in 6 h).
//! Changing either string, or the level, trips it.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_discord::{spawn_sender, Config, ConfigWatcher, DiscordLayer, MockSender};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

/// The `event` of every WARN on `launcher.ingest`, so the test knows the
/// refusal row really was written (and is not just throttled away).
#[derive(Clone, Default)]
struct IngestWarns(Arc<Mutex<Vec<String>>>);

struct EventField(Option<String>);

impl Visit for EventField {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "event" {
            self.0 = Some(value.to_string());
        }
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

impl<S: tracing::Subscriber> Layer<S> for IngestWarns {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let meta = event.metadata();
        if meta.target() == "launcher.ingest" && *meta.level() == tracing::Level::WARN {
            let mut field = EventField(None);
            event.record(&mut field);
            self.0.lock().unwrap().push(field.0.unwrap_or_default());
        }
    }
}

const CONFIG: &str = r#"
[discord]
enabled = true

[discord.channels.errors]
url = "https://discord.com/api/webhooks/1/errors"
rate_limit_per_min = 60

[discord.events]
warning = true
error = true
"#;

/// POST an empty body to `path` on `addr` and return the status line.
async fn post(addr: SocketAddr, path: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response.lines().next().unwrap_or_default().to_string()
}

#[tokio::test(flavor = "current_thread")]
async fn ingest_credential_refusals_never_post_to_discord() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = ConfigWatcher::from_static(Config::from_toml_str(CONFIG).unwrap()).handle();
    let (handle, _task) = spawn_sender(mock, cfg.clone());
    // The default subscriber is thread-local; the current-thread runtime
    // runs the server's tasks on this thread, so they report through it.
    let warns = IngestWarns::default();
    let subscriber = tracing_subscriber::registry()
        .with(DiscordLayer::new(handle, cfg))
        .with(warns.clone());
    let _guard = tracing::subscriber::set_default(subscriber);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = cimmeria_admin_api::login_port_telemetry_router()
        .into_make_service_with_connect_info::<SocketAddr>();
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let status = post(addr, "/api/telemetry/upload-chunk").await;
    assert!(
        status.contains(" 401 "),
        "no token is a credential refusal: {status}"
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        *warns.0.lock().unwrap(),
        ["upload_auth_refused"],
        "the refusal wrote its WARN row, with the credential event"
    );
    assert!(
        calls.lock().await.is_empty(),
        "an upload_auth_refused row must not post to Discord"
    );

    // The layer is live: an ordinary WARN on the same target posts.
    tracing::warn!(target: "launcher.ingest", event = "upload_refused", "control row");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(calls.lock().await.len(), 1, "the control row posts");

    server.abort();
}
