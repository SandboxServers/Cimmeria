//! The two pre-game phases: SmartFox version check and ticket login.
//!
//! Both run before the session belongs to a connection task, so a failure
//! here just drops the socket — there is nothing registered to clean up
//! yet. Everything after login lives in [`super::run_session`].

use tokio::net::TcpStream;

use super::framing::{read_null_terminated, send_null_terminated};
use crate::minigame::game::{create_game, MinigameInstance};
use crate::minigame::protocol::{self, SfsMessage};
use crate::minigame::session::{MinigameSession, SessionRegistry};

/// SmartFoxServer API version the original SWFs were built against.
const API_VERSION: u32 = 154;

/// Phase 1 — answer `verChk` with the Flash cross-domain policy and
/// `apiOK`, returning the version the client claimed.
pub(super) async fn read_and_handle_version(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    external_port: u16,
) -> Option<u32> {
    let msg = read_null_terminated(stream, buf, buf_len).await?;
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
            tracing::warn!("Expected verChk, got something else");
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
) -> Option<(MinigameSession, Box<dyn MinigameInstance>)> {
    let msg = read_null_terminated(stream, buf, buf_len).await?;
    let parsed = protocol::parse_message(&msg)?;

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
                tracing::warn!(api_version, expected = API_VERSION, "Bad API version");
                return None;
            }

            let entity_id: u32 = nick.parse().ok()?;
            // Validate and claim atomically — see `authenticate_and_claim` for
            // the interleaving that a separate `mark_connected` would allow.
            let session = registry
                .authenticate_and_claim(entity_id, &password, &zone)
                .await?;

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
                tracing::warn!(entity_id, game = %zone, "Failed to create minigame");
                return None;
            }

            tracing::info!(entity_id, game = %zone, "Minigame login successful");
            Some((session, game.unwrap()))
        }
        _ => {
            tracing::warn!("Expected login, got something else");
            None
        }
    }
}
