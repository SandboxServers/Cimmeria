//! Authentication service — HTTP/SOAP login handshake (Phases 1 and 2).
//!
//! Phase 1: client POSTs `SGWLoginRequest` to `/SGWLogin/UserAuth`. Server
//! validates credentials and returns the shard list.
//!
//! Phase 2: client POSTs `SGWSelectServerRequest` to
//! `/SGWLogin/ServerSelection`. Server generates a session key and ticket and
//! returns the BaseApp connection info.
//!
//! See `docs/protocol/login-handshake.md` for the full protocol spec.

mod credentials;
mod handlers;
mod service;
mod tls;

#[cfg(test)]
mod login_smoke;

#[cfg(test)]
mod tls_smoke;

pub use service::AuthService;
pub use tls::{TlsCertStore, TlsError, TlsListener};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::audit::{LoginEvent, LoginEventBuffer};

// ── Expiration constants ─────────────────────────────────────────────────────

/// How long a Phase 1 session cookie (SID) remains valid before Phase 2
/// must consume it.  C++ had no explicit TTL but sessions were effectively
/// short-lived; 5 minutes is generous.
const SESSION_TTL: Duration = Duration::from_secs(300);

/// How long a Phase 2 ticket remains valid before Phase 3 must consume it.
/// Matches the C++ `ShardLogonQueue::TicketExpiration` (30 seconds).
const TICKET_TTL: Duration = Duration::from_secs(30);

/// How often the background reaper sweeps expired sessions and tickets.
const REAPER_INTERVAL: Duration = Duration::from_secs(10);

// ── Protocol constants ───────────────────────────────────────────────────────

/// Expected MD5 digest of the entity definitions sent by the client.
const PROTOCOL_DIGEST: &str = "58AFA196AD3AC4F65CADD99BFF23B799";

const LOGIN_NS: &str = concat!(
    r#"xmlns:ns2="http://www.stargateworlds.com/xml/sgwlogin" "#,
    r#"xmlns:ns3="http://www.cheyenneme.com/xml/cmebase" "#,
    r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" "#,
    r#"xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin""#
);

const SELECT_NS: &str = concat!(
    r#"xmlns:ns3="http://www.stargateworlds.com/xml/sgwlogin" "#,
    r#"xmlns:ns1="http://www.cheyenneme.com/xml/cmebase" "#,
    r#"xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" "#,
    r#"xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin""#
);

/// XML declaration prefix matching the original C++ auth server output.
const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials for user '{0}'")]
    InvalidCredentials(String),

    #[error("Account '{0}' is locked")]
    AccountLocked(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

// ── Public types ─────────────────────────────────────────────────────────────

/// Info about a registered BaseApp shard.
#[derive(Clone, Debug)]
pub struct ShardInfo {
    pub name: String,
    pub host: String,
    pub port: u16,
    /// If true, only accounts with `access_level >= 2` may connect.
    pub protected: bool,
}

/// A pending login handoff created by Phase 2 and consumed by Phase 3.
///
/// After the client selects a shard, the auth server generates a session key
/// and ticket and stores a `PendingLogin` here. The BaseApp validates the
/// ticket when the client connects via Mercury UDP (`baseAppLogin` message).
#[derive(Clone)]
pub struct PendingLogin {
    pub account_id: u32,
    /// Human-readable account name (login username). Threaded from the
    /// Phase-1 `SessionRecord` so the BaseApp can surface it (e.g. Discord
    /// notifications) without a second DB lookup at Mercury login.
    pub account_name: String,
    /// Account privilege level (0 = normal, 2+ = admin/GM).
    pub access_level: u32,
    /// 20-char uppercase hex ticket ID.
    pub ticket: String,
    /// 64-char uppercase hex session key (32-byte AES-256 key).
    pub session_key: String,
    /// When this ticket was created (for expiration).
    pub created: Instant,
}

// ── Internal types ───────────────────────────────────────────────────────────

#[derive(Default)]
struct LoginReq {
    sku: String,
    account_name: String,
    password: String,
    protocol_digest: String,
}

#[derive(Clone)]
struct SessionRecord {
    account_id: u32,
    access_level: u32,
    account_name: String,
    created: Instant,
}

/// Request-extension marker inserted only by the TLS listener's middleware.
///
/// Its presence in a request's extensions proves the request arrived over the
/// TLS-terminated listener. The handler uses it to gate plaintext-password
/// acceptance: a plaintext credential is only honoured when this marker is
/// present, so a plaintext password can never be accepted over plain HTTP.
#[derive(Clone, Copy)]
pub(super) struct TlsConn;

/// State shared between the axum HTTP handlers.
#[derive(Clone)]
struct HandlerState {
    shards: Vec<ShardInfo>,
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    developer_mode: bool,
    db: Option<Arc<PgPool>>,
    login_tx: Option<broadcast::Sender<LoginEvent>>,
    login_buffer: Option<LoginEventBuffer>,
}
