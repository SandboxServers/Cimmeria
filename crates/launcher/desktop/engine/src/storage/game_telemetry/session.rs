//! One server session per launch. The launcher mints a token on the telemetry
//! route of the login server the game already talks to, then leaves the marker
//! file the injected DLL reads at boot (`current-session.json`).
use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::telemetry_endpoint::EndpointPolicy;
use crate::telemetry_session::{CurrentSession, TelemetryBlock, CURRENT_SESSION_SCHEMA};
use std::time::Duration;

/// A mint reply is a few hundred bytes; anything near this is not one.
const MAX_REPLY: usize = 16 * 1024;
/// Play waits this long for a session at most, then starts without telemetry.
const CONNECT: Duration = Duration::from_secs(4);
const TOTAL: Duration = Duration::from_secs(8);

/// Capture switches a developer may set in the launcher's own environment.
/// The game's environment is built from scratch, so they are copied on purpose,
/// and only when telemetry is attached. Same variables the Windows launcher's
/// game inherits.
const PASSTHROUGH: [&str; 3] = [
    "CIMMERIA_CLIENT_CAPTURE",
    "CIMMERIA_CLIENT_HOOKS_ENABLE",
    "CIMMERIA_CLIENT_HOOKS_DISABLE",
];

#[derive(Serialize)]
struct MintRequest<'a> {
    install_id: String,
    machine_id: &'a str,
    branch: &'a str,
    git_sha: &'a str,
    launcher_version: &'a str,
    tags: &'a [&'a str],
}

#[derive(Deserialize)]
struct MintReply {
    session_id: String,
    token: String,
    expires_at_ms: i64,
    upload_endpoint: String,
    chunk_max_bytes: u64,
    flush_interval_ms: u64,
}

/// A minted session, not yet on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session(CurrentSession);
impl Session {
    pub fn marker(&self) -> &CurrentSession {
        &self.0
    }
}

/// The session route sits under `/api` on the login server's own host and port.
fn mint_url(login_server: &str) -> String {
    format!(
        "{}/api/auth/dev-session",
        login_server.trim_end_matches('/')
    )
}

/// Ask the installation's first login server for a session. Every failure is an
/// [`Outcome`], never an error that could stop Play.
pub async fn start(
    identity: &Identity,
    login_servers: &[LoginServer],
    tags: &[&str],
) -> Result<Session, Outcome> {
    // https anywhere, plain http only to this machine or a login server: the
    // rule the Windows launcher applies, to the mint address and to whatever
    // upload address the server hands back.
    let policy = EndpointPolicy::from_login_servers(login_servers.iter().map(|s| s.url.as_str()));
    let server = login_servers.first().ok_or(Outcome::SessionUnavailable)?;
    let url = mint_url(&server.url);
    policy.check(&url).map_err(|_| Outcome::EndpointRefused)?;
    let branch = option_env!("CIMMERIA_BUILD_BRANCH").unwrap_or("desktop");
    let git_sha = option_env!("CIMMERIA_BUILD_GIT_SHA").unwrap_or("unknown");
    let launcher_version = concat!("desktop-", env!("CARGO_PKG_VERSION"));
    let body = serde_json::to_vec(&MintRequest {
        install_id: identity.install_id.to_string(),
        machine_id: &identity.machine_id,
        branch,
        git_sha,
        launcher_version,
        tags,
    })
    .map_err(|_| Outcome::SessionUnavailable)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT)
        .timeout(TOTAL)
        .build()
        .map_err(|_| Outcome::SessionUnavailable)?;
    let mut response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| Outcome::SessionUnavailable)?;
    if !response.status().is_success() {
        return Err(Outcome::SessionUnavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Outcome::SessionUnavailable)?
    {
        if bytes.len() + chunk.len() > MAX_REPLY {
            return Err(Outcome::SessionUnavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    let reply: MintReply =
        serde_json::from_slice(&bytes).map_err(|_| Outcome::SessionUnavailable)?;
    if reply.token.is_empty() || reply.session_id.is_empty() {
        return Err(Outcome::SessionUnavailable);
    }
    policy
        .check(&reply.upload_endpoint)
        .map_err(|_| Outcome::EndpointRefused)?;
    Ok(Session(CurrentSession {
        schema_version: CURRENT_SESSION_SCHEMA,
        install_id: identity.install_id.to_string(),
        machine_id: identity.machine_id.clone(),
        session_id: reply.session_id,
        session_started_at_ms: chrono::Utc::now().timestamp_millis(),
        branch: branch.into(),
        git_sha: git_sha.into(),
        telemetry: TelemetryBlock {
            enabled: true,
            token: reply.token,
            expires_at_ms: reply.expires_at_ms,
            upload_endpoint: reply.upload_endpoint,
            chunk_max_bytes: reply.chunk_max_bytes,
            flush_interval_ms: reply.flush_interval_ms,
        },
        tags: tags.iter().map(|tag| (*tag).into()).collect(),
    }))
}

/// The developer capture switches to hand to a game that has telemetry attached.
pub fn passthrough_environment() -> Vec<(&'static str, String)> {
    passthrough_from(|name| std::env::var(name).ok())
}

fn passthrough_from(lookup: impl Fn(&str) -> Option<String>) -> Vec<(&'static str, String)> {
    PASSTHROUGH
        .into_iter()
        .filter_map(|name| Some((name, lookup(name)?)))
        // Switch lists are short names; anything else is not passed to the guest.
        .filter(|(_, value)| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b',' | b'_' | b'*'))
        })
        .collect()
}

#[cfg(test)]
impl Session {
    /// A session as the server would mint it, for tests that never reach one.
    pub(crate) fn fixture() -> Self {
        Self(CurrentSession {
            schema_version: CURRENT_SESSION_SCHEMA,
            install_id: Uuid::from_u128(7).to_string(),
            machine_id: "0123456789abcdef".into(),
            session_id: "session-fixture".into(),
            session_started_at_ms: 1_700_000_000_000,
            branch: "desktop".into(),
            git_sha: "unknown".into(),
            telemetry: TelemetryBlock {
                enabled: true,
                token: "payload.sig".into(),
                expires_at_ms: 1_700_028_800_000,
                upload_endpoint: "http://localhost:8081/api/telemetry".into(),
                chunk_max_bytes: 1_048_576,
                flush_interval_ms: 2_000,
            },
            tags: vec!["desktop-launcher".into()],
        })
    }
}

#[cfg(test)]
pub(super) fn passthrough_for_test(
    lookup: impl Fn(&str) -> Option<String>,
) -> Vec<(&'static str, String)> {
    passthrough_from(lookup)
}
