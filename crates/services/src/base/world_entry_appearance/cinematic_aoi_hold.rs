//! First-login cinematic AoI hold — keep entity introductions off the wire
//! while the client plays the fullscreen intro movie.
//!
//! # The bug shape this guards against
//!
//! 2026-09-19, Castle_CellBlock: a first-login player never saw the
//! `class_id 0` static-mesh guard corpse two metres away until they
//! relogged. The server had sent its CREATE_ENTITY + cascade reliably at
//! `onClientReady`, in the same instant as `onPlayMovie`, and the client
//! ACKed both first try — one retransmit in the whole 75 s window, 17 s
//! after the create burst. So the drop is inside the client, after
//! delivery. The cinematic-exit `CollectGarbage` is already known to reclaim
//! the player's own appearance (#288, healed by the appearance spam in
//! [`super::cinematic`]); a static mesh whose entity was created mid-movie
//! has no such heal path.
//!
//! # The hold
//!
//! [`begin`] runs inside `handle_on_client_ready`'s `pending_client_ready`
//! take, so no `EnteredAoI` can slip through between the pre-ready gate
//! opening and the hold closing. While the hold is set,
//! [`crate::base::deferred_aoi::should_hold_entity_traffic`] buffers entity
//! introductions and everything that depends on them. The hold ends at
//! whichever comes first:
//!
//! - the client's `cancelMovie` (Esc / Lua stop) — [`release_on_cancel`];
//! - [`HOLD_DURATION`] elapsing — the task armed by [`arm_timeout`]. The
//!   client sends nothing when the movie ends on its own.
//!
//! Either way the buffer flushes through the ordinary deferred-AoI path, so
//! the client creates the entities after the movie and its exit GC.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::transport::Transport;

use super::super::world_entry::cell_dispatch::flush_deferred_aoi;
use super::super::ConnectedClientState;

/// How long entity introductions wait when the movie runs to its natural
/// end.
///
/// `Cine-SGWLogo` is 13.10 s (314 frames @ 23.976 fps — see the BIK-header
/// note in [`super::cinematic`]). The remainder covers the cinematic-exit GC.
/// The player is reading the intro dialog by then: in the 2026-09-19 repro
/// the first input came 16.4 s after `onClientReady`, so NPCs arriving at
/// 16 s land behind that dialog rather than popping into an empty room.
pub(crate) const HOLD_DURATION: Duration = Duration::from_secs(16);

/// An active hold, stored on [`ConnectedClientState::cinematic_aoi_hold`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct CinematicAoiHold {
    /// Distinguishes this hold from a later one on the same session, so a
    /// stale timeout task cannot release a hold it did not start.
    pub token: u64,
    pub started: Instant,
}

/// Why a hold ended — the `reason` field on the release log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReleaseReason {
    CancelMovie,
    Timeout,
}

impl ReleaseReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::CancelMovie => "cancel_movie",
            Self::Timeout => "timeout",
        }
    }
}

/// Start a hold on `state`, returning its token. Caller holds the
/// `connected` lock and is taking `pending_client_ready` in the same
/// critical section.
pub(crate) fn begin(state: &mut ConnectedClientState) -> u64 {
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    state.cinematic_aoi_hold = Some(CinematicAoiHold {
        token,
        started: Instant::now(),
    });
    token
}

/// Spawn the task that releases hold `token` after [`HOLD_DURATION`] if
/// `cancelMovie` hasn't released it first.
pub(crate) fn arm_timeout(
    token: u64,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let transport = Arc::clone(transport);
    let connected = Arc::clone(connected);
    let entity_to_addr = Arc::clone(entity_to_addr);
    tracing::info!(
        target: "aoi.cinematic_hold",
        event = "hold_started",
        %addr,
        witness_id,
        token,
        hold_ms = HOLD_DURATION.as_millis() as u64,
        "Cinematic AoI hold: entity introductions buffered until the movie ends"
    );
    tokio::spawn(async move {
        tokio::time::sleep(HOLD_DURATION).await;
        release(
            Some(token),
            ReleaseReason::Timeout,
            witness_id,
            addr,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
    });
}

/// Release whatever hold the session has, because the client dismissed the
/// movie. No-op when no hold is active (mid-session cinematics, or a
/// `cancelMovie` that lost the race with the timeout).
pub(crate) async fn release_on_cancel(
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    release(
        None,
        ReleaseReason::CancelMovie,
        witness_id,
        addr,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Flush the held traffic, then lift the hold.
///
/// `expected` is the timeout task's own token: a task that outlived its hold
/// (released by `cancelMovie`, or the session moved on to a new world entry)
/// must not release a later one. `None` releases any active hold.
///
/// The hold lifts only once a flush leaves the buffer empty **under the same
/// lock**. Messages that arrive while a flush is awaiting its sends buffer
/// behind it and go out on the next pass, so nothing held is overtaken by
/// live traffic — a live `LeftAoI(X)` beating the buffered `EnteredAoI(X)`
/// would leave a ghost on the client.
async fn release(
    expected: Option<u64>,
    reason: ReleaseReason,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let mut flushed = 0usize;
    let held_for;
    loop {
        {
            let Ok(mut clients) = connected.lock() else {
                return;
            };
            let Some(state) = clients.get_mut(&addr) else {
                return;
            };
            let Some(active) = state.cinematic_aoi_hold else {
                return;
            };
            if expected.is_some_and(|token| token != active.token) {
                return;
            }
            if state.deferred_aoi_msgs.is_empty() {
                state.cinematic_aoi_hold = None;
                held_for = active.started.elapsed();
                break;
            }
            flushed += state.deferred_aoi_msgs.len();
        }
        flush_deferred_aoi(
            witness_id,
            addr,
            "cinematic_hold_release",
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
    tracing::info!(
        target: "aoi.cinematic_hold",
        event = "hold_released",
        %addr,
        witness_id,
        reason = reason.as_str(),
        flushed,
        held_ms = held_for.as_millis() as u64,
        "Cinematic AoI hold: released, held entity introductions flushed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::deferred_aoi::DeferredAoiMsg;
    use crate::base::world_entry_appearance::{handle_cancel_movie, handle_on_client_ready};
    use crate::base::PendingClientReadyInfo;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    const PLAYER: u32 = 2;
    /// The Castle_CellBlock guard corpse from the repro: `class_id 0`.
    const CORPSE: u32 = 100_150;

    type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
    type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

    fn corpse_entered() -> DeferredAoiMsg {
        DeferredAoiMsg::EnteredAoI {
            entity_id: CORPSE,
            class_id: 0,
            position: [-322.5, 73.47, -209.83],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: None,
        }
    }

    fn session(addr: SocketAddr, state: ConnectedClientState) -> (Connected, EntityToAddr) {
        (
            Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            Arc::new(Mutex::new(HashMap::from([(PLAYER, addr)]))),
        )
    }

    /// A session mid world-entry, with `first_login` set as given and one
    /// entity introduction plus one player-self call already buffered.
    fn pre_ready_session(addr: SocketAddr, first_login: i32) -> (Connected, EntityToAddr) {
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(PLAYER);
        state.player_name = Some("Tester".to_string());
        state.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id: PLAYER,
            player_id: 42,
            world_name: "Castle_CellBlock".to_string(),
            appearance_args: vec![0xAB],
            tint_args: vec![0xCD],
            first_login,
        });
        state.deferred_aoi_msgs.push(corpse_entered());
        state
            .deferred_aoi_msgs
            .push(DeferredAoiMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index: 0x42,
                args: vec![0x01],
            });
        session(addr, state)
    }

    fn buffered(connected: &Connected, addr: SocketAddr) -> usize {
        connected.lock().unwrap()[&addr].deferred_aoi_msgs.len()
    }

    fn hold_active(connected: &Connected, addr: SocketAddr) -> bool {
        connected.lock().unwrap()[&addr]
            .cinematic_aoi_hold
            .is_some()
    }

    /// The repro, end to end: first login, a `class_id 0` entity already
    /// waiting in the pre-ready buffer. `onClientReady` must NOT flush it
    /// alongside `onPlayMovie`; it goes out only once the movie has had time
    /// to finish. Reverting `begin` in `handle_on_client_ready` flushes it at
    /// ready and fails the first buffer assertion.
    #[tokio::test(start_paused = true)]
    async fn first_login_holds_entity_intro_until_the_movie_has_run_out() {
        let capture = LogCapture::install();
        let addr: SocketAddr = "127.0.0.1:55810".parse().unwrap();
        let (connected, entity_to_addr) = pre_ready_session(addr, /*first_login=*/ 1);
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();

        handle_on_client_ready(
            addr,
            [0u8; 32],
            &connected,
            &None,
            &transport,
            &entity_to_addr,
            &None,
        )
        .await
        .unwrap();

        assert!(hold_active(&connected, addr), "first login starts a hold");
        assert!(
            matches!(
                connected.lock().unwrap()[&addr]
                    .deferred_aoi_msgs
                    .as_slice(),
                [DeferredAoiMsg::EnteredAoI {
                    entity_id: CORPSE,
                    ..
                }]
            ),
            "the entity introduction stays buffered; the player-self call has gone"
        );
        let sent_at_ready = typed_transport.len();

        // Just short of the hold: still nothing for the corpse.
        tokio::time::sleep(HOLD_DURATION - Duration::from_millis(50)).await;
        assert_eq!(buffered(&connected, addr), 1, "held for the whole movie");
        assert_eq!(typed_transport.len(), sent_at_ready);

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!hold_active(&connected, addr), "timeout lifts the hold");
        assert_eq!(buffered(&connected, addr), 0);
        assert_eq!(
            typed_transport.len() - sent_at_ready,
            2,
            "the held introduction flushes as its phase-1 + phase-2 bundles"
        );
        let released = capture
            .find_message(Level::INFO, "Cinematic AoI hold: released")
            .expect("release must be logged for the next repro");
        assert!(released.has_field("reason", "timeout"), "{released:#?}");
        assert!(released.has_field("flushed", "1"), "{released:#?}");
    }

    /// No cinematic, no hold: everything buffered pre-ready flushes at
    /// `onClientReady` as before.
    #[tokio::test(start_paused = true)]
    async fn returning_player_gets_no_hold_and_a_full_flush_at_ready() {
        let addr: SocketAddr = "127.0.0.1:55811".parse().unwrap();
        let (connected, entity_to_addr) = pre_ready_session(addr, /*first_login=*/ 0);
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());

        handle_on_client_ready(
            addr,
            [0u8; 32],
            &connected,
            &None,
            &transport,
            &entity_to_addr,
            &None,
        )
        .await
        .unwrap();

        assert!(!hold_active(&connected, addr));
        assert_eq!(buffered(&connected, addr), 0);
    }

    /// Esc ends the movie early, so the hold ends with it — and traffic that
    /// depends on the held create follows it out rather than being lost.
    #[tokio::test(start_paused = true)]
    async fn cancel_movie_releases_the_hold_and_flushes_in_order() {
        let capture = LogCapture::install();
        let addr: SocketAddr = "127.0.0.1:55812".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(PLAYER);
        begin(&mut state);
        state.deferred_aoi_msgs.push(corpse_entered());
        state
            .deferred_aoi_msgs
            .push(DeferredAoiMsg::WitnessEntityMethod {
                entity_id: CORPSE,
                method_index: 3,
                args: vec![0x00, 0x00, 0x40, 0x00],
                entity_is_player: false,
            });
        let (connected, entity_to_addr) = session(addr, state);
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();

        handle_cancel_movie(&transport, addr, PLAYER, &connected, &entity_to_addr).await;

        assert!(!hold_active(&connected, addr));
        assert_eq!(buffered(&connected, addr), 0);
        assert_eq!(
            typed_transport.len(),
            3,
            "phase-1 bundle, phase-2 bundle, then the held witness method \
             (no cached appearance in this fixture, so no resend packet)"
        );
        let released = capture
            .find_message(Level::INFO, "Cinematic AoI hold: released")
            .expect("release must be logged");
        assert!(
            released.has_field("reason", "cancel_movie"),
            "{released:#?}"
        );
        assert!(released.has_field("flushed", "2"), "{released:#?}");
    }

    /// A timeout task that outlives its hold must not end a later one: Esc
    /// the movie, back out to character select, make another new character
    /// and enter the world again, all inside the first hold's 16 seconds.
    #[tokio::test(start_paused = true)]
    async fn stale_timeout_does_not_release_a_later_hold() {
        let addr: SocketAddr = "127.0.0.1:55813".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(PLAYER);
        let stale = begin(&mut state);
        let current = begin(&mut state);
        assert_ne!(stale, current);
        state.deferred_aoi_msgs.push(corpse_entered());
        let (connected, entity_to_addr) = session(addr, state);
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();

        release(
            Some(stale),
            ReleaseReason::Timeout,
            PLAYER,
            addr,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        assert!(hold_active(&connected, addr), "the later hold survives");
        assert_eq!(buffered(&connected, addr), 1);
        assert!(typed_transport.is_empty());
    }
}
