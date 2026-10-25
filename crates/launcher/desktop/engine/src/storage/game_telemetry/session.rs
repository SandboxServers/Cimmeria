//! One server session per launch. The launcher mints a token on the telemetry
//! route of the login server the game already talks to, then leaves the marker
//! file the injected DLL reads at boot (`current-session.json`).
use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::telemetry_endpoint::{https_upgrade_origin, EndpointPolicy};
use crate::telemetry_session::{CurrentSession, TelemetryBlock, CURRENT_SESSION_SCHEMA};
use std::time::{Duration, Instant};

/// A mint reply is a few hundred bytes; anything near this is not one.
const MAX_REPLY: usize = 16 * 1024;
/// Play waits this long for a session at most, then starts without telemetry.
/// The transport's refresh uses the same bound.
pub(super) const CONNECT: Duration = Duration::from_secs(4);
pub(super) const TOTAL: Duration = Duration::from_secs(8);

/// The session's `launcher_version`: the D-LX8 release version About shows,
/// not the package's, so a session and the launcher summary name one build.
/// The transport's re-mint sends the same value.
pub(super) fn mint_launcher_version() -> String {
    format!(
        "desktop-{}",
        crate::launcher_compatibility::compiled_version()
    )
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    session_kind: Option<&'a str>,
    /// `"logs"` for the log-ingest session (server C-LF4); absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<&'a str>,
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

/// The origin game telemetry talks to for `login_server` (D-LI11, D-LI15): an http
/// login server on a public DNS name becomes `https://<name>` on 443, the rule
/// summaries use. Anything else is kept as written, and the policy refuses it
/// unless it is https or loopback. There is no http fallback.
fn telemetry_origin(login_server: &str) -> String {
    https_upgrade_origin(login_server)
        .unwrap_or_else(|| login_server.trim_end_matches('/').to_owned())
}

/// The session routes sit under `/api` on the login server's own host and
/// port; core's refresh appends `/auth/dev-session/refresh` to this.
pub(super) fn auth_base_url(login_server: &str) -> String {
    format!("{}/api", telemetry_origin(login_server))
}

pub(super) fn mint_url(login_server: &str) -> String {
    format!("{}/auth/dev-session", auth_base_url(login_server))
}

/// One mint attempt: when it started, and whether it is a player (game) session.
/// Only a game session logs `game_session_mint_failed`: the launcher's own
/// adoption session has its own reporting (LT-07).
#[derive(Clone, Copy)]
struct Attempt {
    started: Instant,
    game: bool,
}

/// A failed mint: the coarse outcome, and the C8 reason the session module
/// recorded for it (launcher-side callers report the reason).
pub(super) struct MintFailure {
    pub(super) outcome: Outcome,
    pub(super) reason: &'static str,
}

/// Logs one `game_session_mint_failed` (C8) for a game session and returns the
/// failure to give back.
fn mint_refused(
    reason: &'static str,
    http_status: Option<u16>,
    attempt: Attempt,
    outcome: Outcome,
) -> MintFailure {
    if attempt.game {
        let duration_ms = u64::try_from(attempt.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        session_events::mint_failed(reason, http_status, duration_ms);
    }
    MintFailure { outcome, reason }
}

/// A request that never got a reply: a timeout, or any other transport failure.
fn transport_refused(error: &reqwest::Error, attempt: Attempt) -> MintFailure {
    let reason = if error.is_timeout() {
        "timeout"
    } else {
        "transport"
    };
    mint_refused(reason, None, attempt, Outcome::SessionUnavailable)
}

/// C10 (D-LT7): the upload address the session is given. A loopback address from
/// a remote login server is replaced and the replacement is logged once, with
/// the session `kind`. `Err` carries the C8 reason when no usable address is left.
pub(super) fn upload_for(
    policy: &EndpointPolicy,
    login_server: &str,
    advertised: &str,
    kind: &'static str,
) -> Result<String, &'static str> {
    let effective =
        endpoint_sanity::effective_upload_endpoint(&telemetry_origin(login_server), advertised)
            .ok_or("no_https_endpoint")?;
    policy
        .check(&effective.url)
        .map_err(|_| "endpoint_refused")?;
    if effective.overridden {
        session_events::endpoint_overridden(kind);
    }
    Ok(effective.url)
}

/// Ask the installation's first login server for a session. Every failure is an
/// [`Outcome`], never an error that could stop Play.
pub async fn start(
    identity: &Identity,
    login_servers: &[LoginServer],
    tags: &[&str],
) -> Result<Session, Outcome> {
    start_kind(identity, login_servers, tags, None, None)
        .await
        .map_err(|failure| failure.outcome)
}

/// `start` for a named session kind; `None` is a player session, sent without
/// a `session_kind` key. A failure keeps its C8 reason.
pub(super) async fn start_kind(
    identity: &Identity,
    login_servers: &[LoginServer],
    tags: &[&str],
    session_kind: Option<&str>,
    purpose: Option<&str>,
) -> Result<Session, MintFailure> {
    // https anywhere, plain http only to this machine; an http login server on a
    // DNS name is asked over https (`telemetry_origin`). The rule the Windows
    // launcher applies, to the mint address and to whatever upload address the
    // server hands back.
    let attempt = Attempt {
        started: Instant::now(),
        game: session_kind.is_none(),
    };
    let policy = EndpointPolicy::from_login_servers(login_servers.iter().map(|s| s.url.as_str()));
    let server = login_servers
        .first()
        .ok_or_else(|| mint_refused("transport", None, attempt, Outcome::SessionUnavailable))?;
    let url = mint_url(&server.url);
    policy
        .check(&url)
        .map_err(|_| mint_refused("endpoint_refused", None, attempt, Outcome::EndpointRefused))?;
    let branch = option_env!("CIMMERIA_BUILD_BRANCH").unwrap_or("desktop");
    let git_sha = option_env!("CIMMERIA_BUILD_GIT_SHA").unwrap_or("unknown");
    let launcher_version = mint_launcher_version();
    let body = serde_json::to_vec(&MintRequest {
        install_id: identity.install_id.to_string(),
        machine_id: if session_kind.is_some() {
            ""
        } else {
            &identity.machine_id
        },
        branch,
        git_sha,
        launcher_version: &launcher_version,
        tags,
        session_kind,
        purpose,
    })
    .map_err(|_| mint_refused("transport", None, attempt, Outcome::SessionUnavailable))?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT)
        .timeout(TOTAL)
        .build()
        .map_err(|_| mint_refused("transport", None, attempt, Outcome::SessionUnavailable))?;
    let mut response = client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| transport_refused(&e, attempt))?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(mint_refused(
            "http_status",
            Some(status),
            attempt,
            Outcome::SessionUnavailable,
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| transport_refused(&e, attempt))?
    {
        if bytes.len() + chunk.len() > MAX_REPLY {
            return Err(mint_refused(
                "decode",
                None,
                attempt,
                Outcome::SessionUnavailable,
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let reply: MintReply = serde_json::from_slice(&bytes)
        .map_err(|_| mint_refused("decode", None, attempt, Outcome::SessionUnavailable))?;
    if reply.token.is_empty() || reply.session_id.is_empty() {
        return Err(mint_refused(
            "decode",
            None,
            attempt,
            Outcome::SessionUnavailable,
        ));
    }
    let upload_endpoint = if attempt.game {
        upload_for(&policy, &server.url, &reply.upload_endpoint, "game")
            .map_err(|reason| mint_refused(reason, None, attempt, Outcome::EndpointRefused))?
    } else {
        policy.check(&reply.upload_endpoint).map_err(|_| {
            mint_refused("endpoint_refused", None, attempt, Outcome::EndpointRefused)
        })?;
        reply.upload_endpoint.clone()
    };
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
            upload_endpoint,
            chunk_max_bytes: reply.chunk_max_bytes,
            flush_interval_ms: reply.flush_interval_ms,
        },
        tags: tags.iter().map(|tag| (*tag).into()).collect(),
    }))
}

#[cfg(test)]
pub(super) fn mint_body_for_test(kind: Option<&str>) -> serde_json::Value {
    serde_json::to_value(MintRequest {
        install_id: Uuid::from_u128(7).to_string(),
        machine_id: if kind.is_some() {
            ""
        } else {
            "0123456789abcdef"
        },
        branch: "desktop",
        git_sha: "unknown",
        launcher_version: "desktop-test",
        tags: &["desktop-launcher", "linux", "launcher"],
        session_kind: kind,
        purpose: None,
    })
    .unwrap()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture_identity() -> Identity {
        Identity {
            install_id: Uuid::from_u128(7),
            machine_id: "0123456789abcdef".into(),
        }
    }

    #[test]
    fn a_loopback_upload_from_a_remote_https_login_is_replaced_and_logged_once() {
        let policy = EndpointPolicy::from_login_servers(["https://play.example"]);
        let (result, events) = capture(|| {
            upload_for(
                &policy,
                "https://play.example",
                "http://localhost:8443/api/telemetry",
                "game",
            )
        });
        assert_eq!(result, Ok("https://play.example/api/telemetry".to_string()));
        let overridden: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("upload_endpoint_overridden"))
            .collect();
        assert_eq!(overridden.len(), 1);
        assert_eq!(overridden[0].target, "launcher.telemetry");
        assert_eq!(overridden[0].level, tracing::Level::INFO);
        assert_eq!(overridden[0].field("kind"), Some("game"));
    }

    #[test]
    fn a_loopback_upload_from_a_plain_http_login_is_replaced_with_its_https_origin() {
        let policy = EndpointPolicy::from_login_servers(["http://play.example:8081"]);
        let (result, events) = capture(|| {
            upload_for(
                &policy,
                "http://play.example:8081",
                "http://localhost:8443/api/telemetry",
                "game",
            )
        });
        assert_eq!(result, Ok("https://play.example/api/telemetry".to_string()));
        let overridden: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("upload_endpoint_overridden"))
            .collect();
        assert_eq!(overridden.len(), 1);
        assert_eq!(overridden[0].field("kind"), Some("game"));
    }

    #[test]
    fn a_launcher_kind_replacement_is_logged_with_its_kind() {
        let policy = EndpointPolicy::from_login_servers(["https://play.example"]);
        let (result, events) = capture(|| {
            upload_for(
                &policy,
                "https://play.example",
                "http://127.0.0.1:8443/api/telemetry",
                "launcher",
            )
        });
        assert_eq!(result, Ok("https://play.example/api/telemetry".to_string()));
        let overridden: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("upload_endpoint_overridden"))
            .collect();
        assert_eq!(overridden.len(), 1);
        assert_eq!(overridden[0].field("kind"), Some("launcher"));
    }

    #[test]
    fn a_launcher_kind_503_keeps_the_http_status_reason() {
        let failure = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .and(path("/api/auth/dev-session"))
                    .respond_with(ResponseTemplate::new(503))
                    .mount(&server)
                    .await;
                let servers = vec![LoginServer {
                    name: "fixture".into(),
                    url: server.uri(),
                }];
                start_kind(&fixture_identity(), &servers, &[], Some("launcher"), None)
                    .await
                    .err()
            })
            .expect("a 503 is a failed mint");
        assert_eq!(failure.reason, "http_status");
        assert_eq!(failure.outcome, Outcome::SessionUnavailable);
    }

    #[test]
    fn a_server_error_on_the_mint_logs_one_warning_and_gives_no_session() {
        let (result, events) = capture(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let server = MockServer::start().await;
                    Mock::given(method("POST"))
                        .and(path("/api/auth/dev-session"))
                        .respond_with(ResponseTemplate::new(500))
                        .mount(&server)
                        .await;
                    let servers = vec![LoginServer {
                        name: "fixture".into(),
                        url: server.uri(),
                    }];
                    start_kind(&fixture_identity(), &servers, &[], None, None)
                        .await
                        .map(|_| ())
                        .map_err(|failure| failure.outcome)
                })
        });
        assert_eq!(result, Err(Outcome::SessionUnavailable));
        let failed: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("game_session_mint_failed"))
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].target, "launcher.telemetry");
        assert_eq!(failed[0].level, tracing::Level::WARN);
        assert_eq!(failed[0].field("reason"), Some("http_status"));
        assert_eq!(failed[0].field("http_status"), Some("500"));
    }

    #[test]
    fn a_launcher_kind_mint_failure_logs_no_game_event() {
        let (result, events) = capture(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let server = MockServer::start().await;
                    Mock::given(method("POST"))
                        .and(path("/api/auth/dev-session"))
                        .respond_with(ResponseTemplate::new(500))
                        .mount(&server)
                        .await;
                    let servers = vec![LoginServer {
                        name: "fixture".into(),
                        url: server.uri(),
                    }];
                    start_kind(&fixture_identity(), &servers, &[], Some("launcher"), None)
                        .await
                        .map(|_| ())
                        .map_err(|failure| failure.outcome)
                })
        });
        assert_eq!(result, Err(Outcome::SessionUnavailable));
        assert!(events
            .iter()
            .all(|e| e.field("event") != Some("game_session_mint_failed")));
    }
}
