//! The two pre-game phases: SmartFox version check and ticket login.
//!
//! Both run before the session belongs to a connection task, so a failure
//! here just drops the socket — there is nothing registered to clean up
//! yet. Everything after login lives in [`super::run_session`].
//!
//! # Logging
//!
//! The peer has not authenticated yet, and the port is public: most of what
//! fails here is a port scanner sending HTTP, TLS or random bytes. Those
//! rows log at DEBUG with a stable `reason` and the `peer` address, so they
//! stay queryable in SigNoz without reaching the Discord warn harvest. The
//! one WARN left in this phase is a ticket mismatch for a registered entity,
//! raised by the registry itself.

use std::net::SocketAddr;

use tokio::net::TcpStream;

use super::framing::{frame_sample, read_null_terminated, send_null_terminated, ReadError};
use crate::minigame::game::{create_game, MinigameInstance};
use crate::minigame::protocol::{self, ParseError, SfsMessage};
use crate::minigame::session::{MinigameSession, SessionRegistry};

/// SmartFoxServer API version the original SWFs were built against.
const API_VERSION: u32 = 154;

/// Read one pre-login frame and parse it, logging a rejection at DEBUG.
async fn read_preauth_message(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    peer: SocketAddr,
    phase: &'static str,
) -> Option<SfsMessage> {
    let msg = match read_null_terminated(stream, buf, buf_len).await {
        Ok(msg) => msg,
        Err(ReadError::Closed) => {
            tracing::debug!(%peer, phase, reason = "preauth_closed", "Minigame peer closed before login");
            return None;
        }
        Err(ReadError::Io(e)) => {
            tracing::debug!(%peer, phase, reason = "preauth_read_error", error = %e, "Minigame read failed before login");
            return None;
        }
        Err(ReadError::TooLong) => {
            tracing::debug!(
                %peer,
                phase,
                reason = "preauth_message_too_long",
                limit = buf.len(),
                sample = %frame_sample(&String::from_utf8_lossy(&buf[..*buf_len])),
                "Minigame frame over the size limit before login; closing",
            );
            return None;
        }
    };
    match protocol::parse_message(&msg) {
        Ok(parsed) => Some(parsed),
        Err(e) => {
            log_non_sfs_preauth(peer, phase, &e, &msg);
            None
        }
    }
}

/// The scanner case: an unauthenticated peer sent something that is not a
/// message this phase can use. DEBUG, never WARN.
fn log_non_sfs_preauth(peer: SocketAddr, phase: &'static str, error: &ParseError, msg: &str) {
    let (msg_type, body_action) = error.kind();
    tracing::debug!(
        %peer,
        phase,
        reason = "non_sfs_preauth",
        parse_error = error.reason(),
        msg_type,
        body_action,
        len = msg.len(),
        sample = %frame_sample(msg),
        "Minigame rejected a pre-login frame; closing",
    );
}

/// Phase 1 — answer `verChk` with the Flash cross-domain policy and
/// `apiOK`, returning the version the client claimed.
pub(super) async fn read_and_handle_version(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    external_port: u16,
    peer: SocketAddr,
) -> Option<u32> {
    let parsed = read_preauth_message(stream, buf, buf_len, peer, "verChk").await?;

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
        other => {
            let (msg_type, body_action) = other.kind();
            tracing::debug!(
                %peer,
                phase = "verChk",
                reason = "unexpected_preauth_message",
                msg_type,
                body_action,
                "Minigame expected verChk; closing",
            );
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
    peer: SocketAddr,
) -> Option<(MinigameSession, Box<dyn MinigameInstance>)> {
    let parsed = read_preauth_message(stream, buf, buf_len, peer, "login").await?;

    match parsed {
        SfsMessage::Login {
            zone,
            nick,
            password,
        } => {
            // Check API version
            if api_version != API_VERSION {
                let fail = protocol::encode_extension_raw(
                    "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>",
                );
                let _ = send_null_terminated(stream, &fail).await;
                // INFO, not WARN: anyone can send this pair to the public
                // port, and only a WARN reaches Discord.
                tracing::info!(
                    %peer,
                    api_version,
                    expected = API_VERSION,
                    reason = "bad_api_version",
                    "Bad API version",
                );
                return None;
            }

            let Ok(entity_id) = nick.parse::<u32>() else {
                tracing::debug!(%peer, reason = "bad_login_nick", "Minigame login nick is not an entity id; closing");
                return None;
            };
            // Validate and claim atomically — see `authenticate_and_claim` for
            // the interleaving that a separate `mark_connected` would allow.
            let Some(session) = registry
                .authenticate_and_claim(entity_id, &password, &zone)
                .await
            else {
                tracing::debug!(
                    %peer,
                    entity_id, // nt:id-only claimed by an unauthenticated peer, no session to name it
                    game = %zone,
                    reason = "login_rejected",
                    "Minigame login rejected; closing",
                );
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
                let fail = protocol::encode_extension_raw(
                    "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>",
                );
                let _ = send_null_terminated(stream, &fail).await;
                tracing::warn!(
                    entity_id,
                    entity_name = session.player_name.as_deref(),
                    game = %zone,
                    "Failed to create minigame",
                );
                return None;
            }

            tracing::info!(
                entity_id,
                entity_name = session.player_name.as_deref(),
                game = %zone,
                "Minigame login successful",
            );
            Some((session, game.unwrap()))
        }
        other => {
            let (msg_type, body_action) = other.kind();
            tracing::debug!(
                %peer,
                phase = "login",
                reason = "unexpected_preauth_message",
                msg_type,
                body_action,
                "Minigame expected login; closing",
            );
            None
        }
    }
}
