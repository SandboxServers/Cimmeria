//! The two pre-game phases: SmartFox version check and ticket login.
//!
//! Both run before the session belongs to a connection task, so a failure
//! here just drops the socket — there is nothing registered to clean up
//! yet. Everything after login lives in [`super::run_session`].
//!
//! Every read here is bounded by one deadline shared across both phases.
//! The port is public and the peer is anonymous until the ticket checks
//! out, so a client that connects and then stalls — or trickles a byte at a
//! time — must lose its socket rather than pin a task and a buffer. The
//! deadline covers the whole handshake, not each read, so a trickle cannot
//! keep resetting it.
//!
//! For the same reason every rejection here logs at DEBUG: an anonymous
//! scanner is expected noise on a public port, and WARN events are
//! forwarded to Discord.

use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio::time::Instant;

use super::framing::{read_null_terminated, send_null_terminated};
use crate::minigame::game::{create_game, MinigameInstance};
use crate::minigame::protocol::{self, SfsMessage};
use crate::minigame::session::{MinigameSession, SessionRegistry};

/// SmartFoxServer API version the original SWFs were built against.
const API_VERSION: u32 = 154;

/// Read the next handshake frame, giving up at `deadline`.
async fn read_handshake_frame(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    deadline: Instant,
    peer: SocketAddr,
) -> Option<String> {
    match tokio::time::timeout_at(deadline, read_null_terminated(stream, buf, buf_len)).await {
        Ok(frame) => frame,
        Err(_) => {
            // Debug, not warn: an idle probe on a public port is expected
            // noise, and warn would forward every one to Discord.
            tracing::debug!(
                %peer,
                reason = "handshake_timeout",
                "Minigame handshake timed out; dropping connection"
            );
            None
        }
    }
}

/// Tell the SWF its login was rejected. Best effort: the connection is
/// dropped straight after either way.
async fn send_login_failed(stream: &mut TcpStream) {
    let fail = protocol::encode_extension_raw(
        "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>",
    );
    let _ = send_null_terminated(stream, &fail).await;
}

/// Phase 1 — answer `verChk` with the Flash cross-domain policy and
/// `apiOK`, returning the version the client claimed.
pub(super) async fn read_and_handle_version(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    external_port: u16,
    deadline: Instant,
    peer: SocketAddr,
) -> Option<u32> {
    let msg = read_handshake_frame(stream, buf, buf_len, deadline, peer).await?;
    let parsed = protocol::parse_message(&msg)?;

    match parsed {
        SfsMessage::VersionCheck { version } => {
            // Cross-domain policy (required by Flash XMLSocket)
            let policy = format!(
                "<cross-domain-policy><allow-access-from domain='*' to-ports='{external_port}' /></cross-domain-policy>"
            );
            send_null_terminated(stream, &policy).await.ok()?;

            // API OK (always send OK even if version mismatches — C++ comment explains why)
            let api_ok = "<msg t='sys'><body action='apiOK' r='0'></body></msg>";
            send_null_terminated(stream, api_ok).await.ok()?;

            Some(version)
        }
        _ => {
            tracing::debug!(%peer, reason = "expected_verchk", "Minigame handshake: expected verChk");
            None
        }
    }
}

/// Phase 2 — validate the ticket against the session registry and build
/// the game instance. The `nick` field carries the entity id and `pword`
/// the ticket minted by `SessionRegistry::register`.
pub(super) async fn read_and_handle_login(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    api_version: u32,
    registry: &SessionRegistry,
    deadline: Instant,
    peer: SocketAddr,
) -> Option<(MinigameSession, Box<dyn MinigameInstance>)> {
    let msg = read_handshake_frame(stream, buf, buf_len, deadline, peer).await?;
    let parsed = protocol::parse_message(&msg)?;

    match parsed {
        SfsMessage::Login {
            zone,
            nick,
            password,
        } => {
            // Check API version
            if api_version != API_VERSION {
                send_login_failed(stream).await;
                tracing::debug!(
                    %peer,
                    api_version,
                    expected = API_VERSION,
                    reason = "bad_api_version",
                    "Minigame handshake: bad API version"
                );
                return None;
            }

            let entity_id: u32 = nick.parse().ok()?;
            // Validate and claim atomically — see `authenticate_and_claim` for
            // the interleaving that a separate `mark_connected` would allow.
            let Some(session) = registry
                .authenticate_and_claim(entity_id, &password, &zone)
                .await
            else {
                // The registry has already logged why. Tell the SWF, as the
                // original did for any rejected login.
                send_login_failed(stream).await;
                return None;
            };

            // Create game instance. Unreachable today: `games::create` has a
            // `_` arm that falls back to `PlaceholderGame`, so it always
            // returns `Some`. Kept as a guard for the first game type that
            // rejects a session (a bad seed, an unsupported difficulty), which
            // is why there is no test driving this branch.
            let game = create_game(&session);
            if game.is_none() {
                // The session is already claimed, so nothing else will sweep
                // it and no connection task is going to tear it down. Release
                // it here or the entity is stuck until relog — the very shape
                // of defect B4.
                registry.remove_if_ticket(entity_id, &session.ticket).await;
                send_login_failed(stream).await;
                tracing::warn!(entity_id, game = %zone, "Failed to create minigame");
                return None;
            }

            tracing::info!(entity_id, game = %zone, "Minigame login successful");
            Some((session, game.unwrap()))
        }
        _ => {
            tracing::debug!(%peer, reason = "expected_login", "Minigame handshake: expected login");
            None
        }
    }
}
