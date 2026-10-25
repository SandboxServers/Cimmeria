//! How Play reaches the login server (C8, C9 and D-CT21). A player who is signed
//! in gets a one-time launch ticket through the loopback relay; everyone else
//! keeps the relay password. The relay is never the telemetry endpoint: the
//! worker hands `prepare` the list from `plan` and telemetry keeps the real one.
use super::Plan;
use crate::sign_in::discovery::{self, Discovery, DiscoveryCache};
use crate::sign_in::session::SignIn;
use crate::sign_in::SignInError;
use cimmeria_launcher_core::client_setup::login_servers::{self, LoginServer};
use cimmeria_launcher_core::client_setup::login_session::{self, TicketLogin};
use cimmeria_launcher_relay::{RelayConfig, RelayHandle};
use std::future::pending;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Every heartbeat is this far apart. The server keeps a ticket alive while it hears them.
const HEARTBEAT: Duration = Duration::from_secs(10 * 60);
/// This many heartbeats in a row failing stops them. The game may already be in the world.
const HEARTBEAT_FAILURES: u32 = 3;

/// The discovery results shared by every Play in this process.
static DISCOVERY: OnceLock<tokio::sync::Mutex<DiscoveryCache>> = OnceLock::new();

/// What the client is given for this Play, and what must be undone when it ends.
pub(super) enum ClientLogin {
    /// No relay: the list is what the launcher already had.
    Plain,
    /// The relay stands in for the first server; the password is the player's.
    RelayPassword { relay: RelayHandle },
    /// A one-time ticket for the first server, through the relay.
    Ticket {
        relay: RelayHandle,
        login: Box<TicketLogin>,
        sign_in: Arc<SignIn>,
        launch_id: String,
    },
}

/// Why Play did not start with a launch ticket. Never a password fallback (D-CT11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TicketRefusal {
    /// The sign-in server could not give a ticket (offline, 401, 403 or 5xx).
    Unavailable,
    /// The stored sign-in was refused, so the player must sign in again.
    SignInEnded,
}

impl TicketRefusal {
    /// The reason code the panel shows (CT-40 and CT-42).
    pub(super) fn reason(&self) -> &'static str {
        match self {
            Self::Unavailable => "ticket_unavailable",
            Self::SignInEnded => "sign_in_ended",
        }
    }
}

/// The discovery for the first login server, through the process-wide cache.
/// `None` when the list is empty, the server does not answer, or it does not
/// describe its sign-in.
pub(super) async fn discover(servers: &[LoginServer]) -> Option<Discovery> {
    // Unit and shell tests never probe a real server; plan() tests pass a Discovery directly.
    if cfg!(any(test, feature = "test-support")) {
        return None;
    }
    let first = servers.first()?;
    let cache = DISCOVERY.get_or_init(|| tokio::sync::Mutex::new(DiscoveryCache::default()));
    let mut cache = cache.lock().await;
    discovery::probe_cached(&mut cache, &discovery::client(), &first.url, Instant::now()).await
}

/// The list the client gets and how it logs in. `Err` ends the launch before `prepare`; a signed-in player gets `Err` whenever no ticket can be had (C15 d).
pub(super) async fn plan(
    sign_in: Option<Arc<SignIn>>,
    plan: &Plan,
    servers: &[LoginServer],
    discovery: Option<Discovery>,
) -> Result<(Vec<LoginServer>, ClientLogin), TicketRefusal> {
    let plain = || (servers.to_vec(), ClientLogin::Plain);
    let Some(first) = servers.first() else {
        return Ok(plain());
    };
    // The sign-in this Play is for (D-CT21): the session for the first server's
    // auth base. Without discovery that base comes from the server URL itself.
    let base = match &discovery {
        Some(found) => Some(found.auth_base.clone()),
        None => discovery::auth_base(&first.url),
    };
    let sign_in = sign_in.filter(|s| base.as_ref() == Some(s.base()));
    let signed_in = sign_in.is_some();
    let discovery = match discovery {
        Some(found) if found.soap_relay => found,
        Some(_) => return without_relay(signed_in, "soap_relay_disabled").map(|()| plain()),
        None => return without_relay(signed_in, "no_discovery").map(|()| plain()),
    };
    let config = RelayConfig {
        upstream: discovery.auth_base.clone(),
        extra_root_pem: discovery::extra_root_pem(),
        session: None,
    };
    let relay = match cimmeria_launcher_relay::start(config).await {
        Ok(relay) => relay,
        Err(_) => {
            tracing::info!(target: "launcher.relay", event = "relay_start_failed", reason = "relay_start_failed");
            return without_relay(signed_in, "relay_start_failed").map(|()| plain());
        }
    };
    let relay_url = relay.soap_url();
    let first_only = |url: String| {
        let mut list = servers.to_vec();
        list[0].url = url;
        list
    };
    let Some(sign_in) = sign_in else {
        let list = first_only(relay_url);
        tracing::info!(target: "launcher.relay", event = "started", mode = "relay_password");
        return Ok((list, ClientLogin::RelayPassword { relay }));
    };
    let launch_id = launch_id(plan.id);
    match sign_in.launch_ticket(&launch_id).await {
        Ok(view) => {
            let login = TicketLogin {
                server_name: first.name.clone(),
                relay_url: relay_url.clone(),
                account: view.account_name.clone(),
                ticket: view.ticket,
                display: view.account_name,
                links: discovery.links.clone(),
                errors: login_session::default_errors(),
            };
            tracing::info!(target: "launcher.relay", event = "started", mode = "ticket");
            let list = vec![LoginServer {
                name: first.name.clone(),
                url: relay_url,
            }];
            Ok((
                list,
                ClientLogin::Ticket {
                    relay,
                    login: Box::new(login),
                    sign_in,
                    launch_id,
                },
            ))
        }
        Err(error) => {
            relay.shutdown().await;
            let refusal = if error == SignInError::Server("signed_out".into()) {
                TicketRefusal::SignInEnded
            } else {
                TicketRefusal::Unavailable
            };
            tracing::info!(target: "launcher.relay", event = "ticket_failed", reason = refusal.reason());
            Err(refusal)
        }
    }
}

/// No relay for this Play. C15 (d): a signed-in player is refused, never handed
/// the password screen; anyone else keeps the plain list.
fn without_relay(signed_in: bool, cause: &'static str) -> Result<(), TicketRefusal> {
    if !signed_in {
        return Ok(());
    }
    tracing::info!(target: "launcher.relay", event = "ticket_failed", reason = cause);
    Err(TicketRefusal::Unavailable)
}

/// The telemetry list and the client list for one Play (C9). Telemetry always
/// gets the real servers, so the relay is never its endpoint.
pub(super) fn lists_for(
    servers: &[LoginServer],
    client: Vec<LoginServer>,
) -> (&[LoginServer], Vec<LoginServer>) {
    (servers, client)
}

/// Write the ticket file after `prepare` wrote the plain one. Other modes write nothing.
pub(super) fn write(root: &Path, login: &ClientLogin) -> std::io::Result<()> {
    match login {
        ClientLogin::Ticket { login, .. } => login_session::write_ticket(root, login),
        ClientLogin::Plain | ClientLogin::RelayPassword { .. } => Ok(()),
    }
}

/// Heartbeats a ticket every [`HEARTBEAT`]. It never returns: after
/// [`HEARTBEAT_FAILURES`] failures in a row it stops quietly and pends, so the
/// caller's `select!` ends it when the game does.
pub(super) async fn heartbeat_loop(login: &ClientLogin) {
    let ClientLogin::Ticket {
        sign_in, launch_id, ..
    } = login
    else {
        return pending().await;
    };
    let start = tokio::time::Instant::now() + HEARTBEAT;
    let mut ticker = tokio::time::interval_at(start, HEARTBEAT);
    let mut failures = 0;
    loop {
        ticker.tick().await;
        match sign_in.heartbeat(launch_id).await {
            Ok(true) => failures = 0,
            _ => failures += 1,
        }
        if failures >= HEARTBEAT_FAILURES {
            tracing::info!(target: "launcher.relay", event = "heartbeat_stopped", reason = "heartbeat_failed");
            return pending().await;
        }
    }
}

/// End the ticket, stop the relay, and put the plain list back on disk so no
/// ticket stays in the install. Runs on every exit after the game started.
pub(super) async fn finish(root: &Path, plain: &[LoginServer], login: ClientLogin) {
    // Scrub before the ticket is ended on the network, so a slow or dead server
    // never keeps the live ticket in the install.
    if let Err(error) = login_servers::write(root, plain) {
        tracing::warn!(target: "launcher.relay", event = "scrub_failed", reason = %error.kind());
    }
    let mode = match login {
        ClientLogin::Plain => "plain",
        ClientLogin::RelayPassword { relay } => {
            relay.shutdown().await;
            "relay_password"
        }
        ClientLogin::Ticket {
            relay,
            sign_in,
            launch_id,
            ..
        } => {
            if let Err(error) = sign_in.end_ticket(&launch_id).await {
                tracing::info!(target: "launcher.relay", event = "end_failed", reason = ?error);
            }
            relay.shutdown().await;
            "ticket"
        }
    };
    tracing::info!(target: "launcher.relay", event = "stopped", mode);
}

/// The launch id the server knows the ticket by: the operation id as text, with
/// only `[A-Za-z0-9-]` and at least 16 characters.
fn launch_id(id: Uuid) -> String {
    let mut text: String = id
        .to_string()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    while text.len() < 16 {
        text.push('0');
    }
    text
}
