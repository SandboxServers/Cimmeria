//! SmartFoxServer-compatible TCP server for minigame connections.
//!
//! Each client connection runs in its own tokio task with an independent
//! game instance and tick timer. This module owns the connection lifecycle;
//! the siblings hold the pieces it leans on:
//!
//! - [`accept`] — the listener and its accept loop.
//! - [`limits`] — connection caps and timeouts for the public port.
//! - [`framing`] — null-terminated message framing over the socket.
//! - [`handshake`] — the pre-game version check and ticket login.
//! - [`result_dispatch`] — the single seam every outcome leaves through.
//! - [`session_loop`] — one authenticated session: room join, game loop,
//!   teardown.
//!
//! Every row a connection's task logs, including the framing and session
//! rows, sits under that connection's `minigame.connection` span.
//!
//! Reference: `deprecated/cpp/src/baseapp/minigame_connection.cpp`.

mod accept;
mod framing;
mod handshake;
mod limits;
mod result_dispatch;
mod session_loop;

#[cfg(test)]
mod connection_span_tests;
#[cfg(test)]
mod converse_loop_tests;
#[cfg(test)]
mod crystal_game_loop_tests;
#[cfg(test)]
mod goauld_loop_tests;
#[cfg(test)]
mod listener_tests;
#[cfg(test)]
mod livewire_abort_tests;
#[cfg(test)]
mod session_snapshot_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timeout_tests;

use std::net::SocketAddr;
#[cfg(test)]
use std::time::Duration;

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::field::Empty;
use tracing::Instrument;

pub use self::accept::{run, run_with_limits};
pub use self::limits::ListenerLimits;

use self::framing::MAX_MESSAGE_LEN;
use self::handshake::{read_and_handle_login, read_and_handle_version};
use self::limits::ConnectionPermit;
use self::session_loop::run_session;
// The test modules reach these through `use super::*`; they lived here
// before the session loop moved to `session_loop`.
#[cfg(test)]
use self::framing::send_null_terminated;
#[cfg(test)]
use self::result_dispatch::{RESULT_CANCELED, RESULT_VICTORY};
use super::session::SessionRegistry;

/// Handle a single minigame connection through the full lifecycle.
///
/// The whole connection runs inside one `minigame.connection` span, opened
/// with the peer address. The login fills in who it is (`entity_id`,
/// `player_id`, `game` and their names), so every row the task logs after
/// that, down to a framing send error, can be traced back to one player's
/// session. The span is attached with `.instrument`, never an entered
/// guard, because the body awaits throughout.
///
/// `_permit` is this connection's slot in the listener's caps; holding it
/// for the whole function gives the slot back on every exit path.
async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    permit: ConnectionPermit,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    external_port: u16,
    limits: ListenerLimits,
) {
    let span = tracing::info_span!(
        "minigame.connection",
        %peer,
        entity_id = Empty,
        entity_name = Empty,
        player_id = Empty,
        player_name = Empty,
        game = Empty,
    );
    run_connection(
        stream,
        peer,
        permit,
        registry,
        result_tx,
        external_port,
        limits,
    )
    .instrument(span)
    .await;
}

/// The body of [`handle_connection`], run inside its span.
async fn run_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    _permit: ConnectionPermit,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    external_port: u16,
    limits: ListenerLimits,
) {
    let mut buf = vec![0u8; MAX_MESSAGE_LEN];
    let mut buf_len = 0usize;

    // Phases 1 and 2 share one deadline, measured from accept. The SWF
    // sends verChk and login on its own as soon as it loads, so a real
    // client finishes in well under a second; a peer that connects and
    // says nothing would otherwise hold its slot forever. Cancelling here
    // is safe: the only await after the registry claim is on the
    // unreachable `create_game` failure branch.
    let handshake = async {
        let api_version =
            read_and_handle_version(&mut stream, &mut buf, &mut buf_len, external_port, peer)
                .await?;
        read_and_handle_login(
            &mut stream,
            &mut buf,
            &mut buf_len,
            api_version,
            &registry,
            peer,
        )
        .await
    };
    let (session, game) = match tokio::time::timeout(limits.handshake_timeout, handshake).await {
        Ok(Some(pair)) => pair,
        Ok(None) => return,
        Err(_) => {
            tracing::debug!(
                %peer,
                reason = "handshake_timeout",
                timeout_s = limits.handshake_timeout.as_secs(),
                "Minigame peer did not log in before the deadline; closing",
            );
            return;
        }
    };

    let entity_id = session.entity_id;

    // The login phase already claimed this session under the registry lock,
    // so the sweep will leave it alone. What is left is the other half:
    // *every* exit path below has to unregister it, or the player can never
    // start another minigame without relogging (defect B4).
    let game_name = session.game_name.clone();
    // Captured before `run_session` takes the session: the teardown deletes
    // by ticket so a stale task can never unregister a *newer* session that
    // was registered for the same entity after this one was swept.
    let ticket = session.ticket.clone();
    // For the closing row, which outlives the session `run_session` takes.
    let player = session.discord_player();

    // The login named the peer: fill in the connection span so every row
    // from here to the teardown carries the player. `None` ids and names
    // stay unrecorded rather than becoming "" or 0.
    let span = tracing::Span::current();
    span.record("entity_id", entity_id);
    span.record("entity_name", player.name());
    span.record("player_id", player.id);
    span.record("player_name", player.name());
    span.record("game", game_name.as_str());

    let started = std::time::Instant::now();
    let end = run_session(
        stream,
        &registry,
        &result_tx,
        session,
        game,
        ReadState { buf, len: buf_len },
        limits.idle_timeout,
    )
    .await;

    registry.remove_if_ticket(entity_id, &ticket).await;
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        event = "minigame.session_end",
        entity_id,
        entity_name = player.name(),
        player_id = player.id,
        player_name = player.name(),
        game = %game_name,
        room_id = end.room_id, // nt:id-only a SmartFox room number allocated per session
        end_reason = end.reason.as_str(),
        outcome = end.reason.outcome(),
        duration_ms,
        "Minigame session ended"
    );
}

/// Why a minigame session stopped. Each variant is one `end_reason` value on
/// the `minigame.session_end` row; the list is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EndReason {
    /// The game reported a win, sent upstream as `RESULT_VICTORY`.
    Victory,
    /// The game reported a loss, sent upstream as `RESULT_DEFEAT`.
    Defeat,
    /// The client closed the socket (FIN) between frames.
    ClientClosed,
    /// The authenticated game client closed the minigame window.
    PlayerCanceled,
    /// The socket read failed with an I/O error.
    ReadFailed,
    /// A client frame went over the size limit.
    FrameTooLong,
    /// Nothing arrived from the client for the idle timeout.
    IdleTimeout,
    /// A send to the client failed or timed out, or the session was torn
    /// down before the game loop started.
    SendFailed,
}

impl EndReason {
    /// The `end_reason` field value.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
            Self::ClientClosed => "client_closed",
            Self::PlayerCanceled => "player_canceled",
            Self::ReadFailed => "read_error",
            Self::FrameTooLong => "frame_too_long",
            Self::IdleTimeout => "idle_timeout",
            Self::SendFailed => "send_failed",
        }
    }

    /// The `outcome` field value: a win or a loss when one was reported,
    /// otherwise a cancel.
    pub(super) fn outcome(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
            _ => "canceled",
        }
    }
}

/// What [`run_session`] hands back to [`handle_connection`] for the closing
/// row: why it stopped, and the room it ran in.
pub(super) struct SessionEnd {
    pub(super) reason: EndReason,
    pub(super) room_id: u32,
}

/// The read buffer and how much of it holds unconsumed bytes, carried from
/// the handshake into the session (a login frame can arrive coalesced with
/// the first game frame).
struct ReadState {
    buf: Vec<u8>,
    len: usize,
}

impl ReadState {
    /// A fresh, empty buffer of the framing size cap.
    #[cfg(test)]
    fn empty() -> Self {
        Self {
            buf: vec![0u8; MAX_MESSAGE_LEN],
            len: 0,
        }
    }
}
