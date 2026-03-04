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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use quick_xml::{Reader, events::Event};
use rand::Rng;
use tokio::net::TcpListener;

use cimmeria_common::ServerConfig;

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
}

/// A pending login handoff created by Phase 2 and consumed by Phase 3.
///
/// After the client selects a shard, the auth server generates a session key
/// and ticket and stores a `PendingLogin` here. The BaseApp validates the
/// ticket when the client connects via Mercury UDP (`baseAppLogin` message).
#[derive(Clone)]
pub struct PendingLogin {
    pub account_id: u32,
    /// 20-char uppercase hex ticket ID.
    pub ticket: String,
    /// 64-char uppercase hex session key (32-byte AES-256 key).
    pub session_key: String,
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
    account_name: String,
}

/// State shared between the axum HTTP handlers.
#[derive(Clone)]
struct HandlerState {
    shards: Vec<ShardInfo>,
    sessions: Arc<Mutex<HashMap<String, SessionRecord>>>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    developer_mode: bool,
}

// ── AuthService ──────────────────────────────────────────────────────────────

/// Authentication service managing player login via HTTP/SOAP.
pub struct AuthService {
    /// Internal TCP port for BaseApp registration messages (13001).
    pub listener_addr: SocketAddr,
    /// HTTP port for client SOAP login requests (8081).
    pub logon_addr: SocketAddr,
    /// Whether the HTTP listener is running.
    pub is_running: bool,
    /// Registered BaseApp shards included in Phase 1 responses.
    pub shards: Vec<ShardInfo>,
    /// Pending logins keyed by ticket; shared with the BaseService for Phase 3 validation.
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    developer_mode: bool,
}

impl AuthService {
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = SocketAddr::from(([127, 0, 0, 1], config.auth_port));
        let logon_addr = SocketAddr::new(
            config
                .auth_host
                .parse()
                .unwrap_or_else(|_| [0, 0, 0, 0].into()),
            config.logon_port,
        );

        Self {
            listener_addr,
            logon_addr,
            is_running: false,
            shards: Vec::new(),
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            developer_mode: config.developer_mode,
        }
    }

    /// Register a BaseApp shard.
    ///
    /// Must be called before [`start`] so the shard appears in Phase 1 responses.
    pub fn register_shard(&mut self, info: ShardInfo) {
        tracing::info!(name = %info.name, host = %info.host, port = info.port, "Registering shard");
        self.shards.push(info);
    }

    /// Start the HTTP/SOAP login listener on `logon_addr`.
    ///
    /// Spawns an axum server as a background tokio task. Returns once the
    /// listener is bound; the task runs until the process exits.
    pub async fn start(&mut self) -> Result<(), AuthError> {
        tracing::info!(addr = %self.logon_addr, "Starting auth HTTP listener");
        tracing::trace!(shard_count = self.shards.len(), developer_mode = self.developer_mode, "Auth service config");

        let state = Arc::new(HandlerState {
            shards: self.shards.clone(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pending_logins: Arc::clone(&self.pending_logins),
            developer_mode: self.developer_mode,
        });

        let app = Router::new()
            .route("/SGWLogin/UserAuth", post(handle_user_auth))
            .route("/SGWLogin/ServerSelection", post(handle_server_selection))
            .with_state(state);

        tracing::trace!(addr = %self.logon_addr, "Binding TCP listener for auth HTTP");
        let listener = TcpListener::bind(self.logon_addr).await.map_err(|e| {
            tracing::error!(addr = %self.logon_addr, error = %e, "Failed to bind auth TCP listener");
            e
        })?;
        tracing::info!(addr = %listener.local_addr().unwrap(), "Auth HTTP listener bound");

        tracing::trace!("Spawning auth HTTP server task");
        tokio::spawn(async move {
            tracing::trace!("Auth HTTP server task started");
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Auth HTTP server error: {e}");
            }
            tracing::trace!("Auth HTTP server task exited");
        });

        self.is_running = true;
        Ok(())
    }

    /// Stop the auth service.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping authentication service");
        // TODO: signal the HTTP server task to exit cleanly
        self.is_running = false;
        tracing::trace!("Authentication service stopped");
    }

    /// Consume a pending login by ticket.
    ///
    /// Called by the BaseService when a `baseAppLogin` Mercury message arrives.
    /// Returns `None` if the ticket is unknown or has already been consumed.
    pub fn take_pending_login(&self, ticket: &str) -> Option<PendingLogin> {
        self.pending_logins.lock().ok()?.remove(ticket)
    }

    /// Return a cloned `Arc` pointing to the pending-logins map.
    ///
    /// Used by the orchestrator to wire the shared map into `BaseService`
    /// before starting services, so the BaseService can validate Phase 3 tickets.
    pub fn pending_logins_arc(&self) -> Arc<Mutex<HashMap<String, PendingLogin>>> {
        Arc::clone(&self.pending_logins)
    }
}

// ── Axum handlers ────────────────────────────────────────────────────────────

/// Phase 1: `POST /SGWLogin/UserAuth`
async fn handle_user_auth(
    State(state): State<Arc<HandlerState>>,
    body: String,
) -> Response {
    tracing::debug!("Phase 1: UserAuth");
    tracing::trace!(body = %body, "Phase 1 raw SOAP request");

    let req = match parse_login_request(&body) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Bad login request: {e}");
            return login_error(13, "Internal error.");
        }
    };

    if req.sku != "SGW_BETA" {
        return login_error(3, "The specified service does not exist.");
    }
    if req.password.len() != 40 || !req.password.chars().all(|c| c.is_ascii_hexdigit()) {
        return login_error(2, "The specified password has is invalid.");
    }
    let name_ok = req.account_name.len() >= 3
        && req.account_name.len() <= 20
        && req.account_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    if !name_ok {
        return login_error(1, "The specified account name is invalid.");
    }
    if !state.developer_mode
        && req.protocol_digest.to_uppercase() != PROTOCOL_DIGEST
    {
        tracing::warn!(got = %req.protocol_digest, expected = PROTOCOL_DIGEST, "Protocol digest mismatch");
        return login_error(
            17,
            "Protocol version mismatch; your client version is not supported.",
        );
    }

    // Credential check.
    // In developer mode, any valid-format credentials are accepted (account_id = 1).
    // TODO: query sgw.account for production use.
    let account_id: u32 = if state.developer_mode {
        tracing::debug!(user = %req.account_name, "developer mode: accepting credentials");
        1
    } else {
        return login_error(10, "A request to the database server failed.");
    };

    if state.shards.is_empty() {
        return login_error(7, "No shards are available to the authentication server.");
    }

    let sid = random_hex(20);
    tracing::debug!(sid = %sid, "Phase 1 generated SID");
    {
        state.sessions.lock().unwrap().insert(
            sid.clone(),
            SessionRecord {
                account_id,
                account_name: req.account_name.clone(),
            },
        );
    }

    tracing::info!(user = %req.account_name, account_id, "Phase 1 success");

    let xml = login_success_xml(account_id, &state.shards);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/xml".to_string()),
            (header::SET_COOKIE, format!("SID={sid}")),
        ],
        xml,
    )
        .into_response()
}

/// Phase 2: `POST /SGWLogin/ServerSelection`
async fn handle_server_selection(
    State(state): State<Arc<HandlerState>>,
    headers: HeaderMap,
    body: String,
) -> Response {
    tracing::debug!("Phase 2: ServerSelection");
    tracing::trace!(body = %body, "Phase 2 raw SOAP request");

    let sid = match extract_sid(&headers) {
        Some(s) => s,
        None => return select_error(15, "Your logon session has expired. Please log in again."),
    };

    let session = {
        state.sessions.lock().unwrap().get(&sid).cloned()
    };
    let session = match session {
        Some(s) => s,
        None => return select_error(15, "Your logon session has expired. Please log in again."),
    };

    let selected = match parse_server_selection(&body) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Bad server selection: {e}");
            return select_error(13, "Internal error.");
        }
    };

    let shard = match state.shards.iter().find(|s| s.name == selected) {
        Some(s) => s.clone(),
        None => return select_error(8, "No such shard."),
    };

    // 64-char hex AES-256 session key, 20-char hex ticket.
    let session_key = random_hex(32);
    let ticket = random_hex(10);
    tracing::debug!(session_key = %session_key, ticket = %ticket, "Phase 2 generated session_key + ticket");

    {
        state.pending_logins.lock().unwrap().insert(
            ticket.clone(),
            PendingLogin {
                account_id: session.account_id,
                ticket: ticket.clone(),
                session_key: session_key.clone(),
            },
        );
    }

    tracing::info!(
        user = %session.account_name,
        shard = %shard.name,
        "Phase 2 success"
    );

    let xml = server_location_xml(&shard, &session_key, &ticket);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

// ── XML helpers ──────────────────────────────────────────────────────────────

fn parse_login_request(body: &str) -> Result<LoginReq, String> {
    let mut reader = Reader::from_str(body);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"SGWLoginRequest" {
                    let mut req = LoginReq::default();
                    for attr in e.attributes().flatten() {
                        let val =
                            String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match attr.key.as_ref() {
                            b"SKU" => req.sku = val,
                            b"AccountName" => req.account_name = val,
                            b"Password" => req.password = val,
                            b"ProtocolDigest" => req.protocol_digest = val,
                            _ => {}
                        }
                    }
                    return Ok(req);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Err("SGWLoginRequest element not found".into())
}

fn parse_server_selection(body: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(body);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"SGWSelectServerRequest" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"ServerSelection" {
                            return Ok(
                                String::from_utf8_lossy(attr.value.as_ref()).into_owned()
                            );
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Err("SGWSelectServerRequest/ServerSelection not found".into())
}

fn extract_sid(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie
        .split(';')
        .map(str::trim)
        .find(|s| s.starts_with("SID="))
        .map(|s| s["SID=".len()..].to_string())
}

fn login_error(code: u32, msg: &str) -> Response {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginError ns3:ErrorStr=\"{msg}\" ns3:ErrorNum=\"{code}\" />\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml".to_string())], xml).into_response()
}

fn login_success_xml(account_id: u32, shards: &[ShardInfo]) -> String {
    let entries: String = shards
        .iter()
        .map(|s| {
            format!(
                "<Shard ServerName=\"{}\" Fullness=\"LOW\" Busy=\"LOW\" />",
                s.name
            )
        })
        .collect();

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginSuccess>\
         <AccountInfo ExpireDate=\"0000-00-00T00:00:00.000Z\" AccountId=\"{account_id}\" />\
         <SGWShardListResp>{entries}</SGWShardListResp>\
         </SGWLoginSuccess>\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    )
}

fn select_error(code: u32, msg: &str) -> Response {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerSelectionError ns1:ErrorStr=\"{msg}\" ns1:ErrorNum=\"{code}\" />\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
    );
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml".to_string())], xml).into_response()
}

fn server_location_xml(shard: &ShardInfo, session_key: &str, ticket: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerLocation SessionKey=\"{session_key}\" Port=\"{port}\" IP=\"{ip}\" BWMailBox=\"1\">\
         <TICKET Ticket=\"{ticket}\" />\
         </ServerLocation>\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
        port = shard.port,
        ip = shard.host,
    )
}

/// Generate `byte_count` random bytes as uppercase hex.
fn random_hex(byte_count: usize) -> String {
    let mut rng = rand::rng();
    (0..byte_count)
        .map(|_| format!("{:02X}", rng.random::<u8>()))
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = AuthService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 13001);
        assert_eq!(svc.logon_addr.port(), 8081);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let mut config = ServerConfig::default();
        config.logon_port = 0; // OS-assigned port to avoid conflicts in tests
        let mut svc = AuthService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[test]
    fn random_hex_length() {
        assert_eq!(random_hex(20).len(), 40); // SID
        assert_eq!(random_hex(10).len(), 20); // ticket
        assert_eq!(random_hex(32).len(), 64); // session key
    }

    #[test]
    fn parse_valid_login_request() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="test" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let req = parse_login_request(body).unwrap();
        assert_eq!(req.sku, "SGW_BETA");
        assert_eq!(req.account_name, "test");
        assert_eq!(req.password, "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3");
    }

    #[test]
    fn parse_server_selection_request() {
        let body = r#"<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="Shard" />"#;
        let sel = parse_server_selection(body).unwrap();
        assert_eq!(sel, "Shard");
    }

    #[test]
    fn login_success_xml_contains_shard() {
        let shards = vec![ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
        }];
        let xml = login_success_xml(42, &shards);
        assert!(xml.contains(r#"AccountId="42""#));
        assert!(xml.contains(r#"ServerName="Shard""#));
    }

    #[test]
    fn server_location_xml_contains_key_and_ticket() {
        let shard = ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
        };
        let xml = server_location_xml(&shard, "AAAA", "BBBB");
        assert!(xml.contains(r#"SessionKey="AAAA""#));
        assert!(xml.contains(r#"Ticket="BBBB""#));
        assert!(xml.contains(r#"Port="32832""#));
    }
}
