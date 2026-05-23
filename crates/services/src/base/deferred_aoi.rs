//! Defer cell-driven AoI dispatches until the client signals `onClientReady`.
//!
//! # The bug shape this guards against
//!
//! Between `play_character` (sending RESET_ENTITIES) and the client's
//! `onClientReady`, the cell server starts populating the witness's
//! AoI as soon as the player entity is spawned in the space. For a
//! Castle_CellBlock instance with 28 NPCs that's 28 [`CellToBaseMsg::EnteredAoI`]
//! events fired pre-`mapLoaded`, each translating into a CREATE_ENTITY +
//! UPDATE_AVATAR packet (and a property cascade), totaling ~33+ reliable
//! packets sent in the 3.5s window while the client is loading terrain
//! and cannot ACK. The TX window fills before `mapLoaded` itself runs;
//! its own burst then triggers the deferred-queue / silent-best-effort
//! path (see #354).
//!
//! # The defer gate
//!
//! [`should_defer`] checks whether the witness's session is still pre-
//! `onClientReady`. While `pending_map_loaded` or `pending_client_ready`
//! is set, AoI-class messages targeted at that witness are pushed into
//! [`ConnectedClientState::deferred_aoi_msgs`] via [`push_deferred`]
//! instead of dispatching immediately. When `onClientReady` fires, the
//! caller drains the buffer with [`drain_deferred`] and re-dispatches
//! through the normal AoI handlers — by then the client is ready to
//! ACK at line rate, so the burst clears the TX window quickly.
//!
//! # Why not buffer `EntityMoved`
//!
//! Position updates are unreliable and short-lived; the client doesn't
//! have the AoI'd entity yet (its `EnteredAoI` is queued in the buffer),
//! so a position update would target an unknown entity and be dropped.
//! The next post-`onClientReady` position frame supersedes anything we
//! would have buffered, so dropping `EntityMoved` pre-ready is correct
//! and cheaper than queueing.
//!
//! See issue #354 fix #2.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::cell::messages::NpcAoIData;

use super::ConnectedClientState;

/// AoI-class cell→base messages that may be deferred when the witness
/// hasn't signalled `onClientReady`.
///
/// Mirrors the relevant `CellToBaseMsg` variants but trimmed to the
/// fields the AoI dispatch handlers need on flush, so we don't have to
/// drag unrelated variants through the buffer.
#[derive(Debug)]
pub(crate) enum DeferredAoiMsg {
    /// Buffered [`crate::cell::messages::CellToBaseMsg::EnteredAoI`].
    EnteredAoI {
        entity_id: u32,
        class_id: u8,
        position: [f32; 3],
        direction: [f32; 3],
        level: u32,
        npc_data: Option<NpcAoIData>,
    },
    /// Buffered [`crate::cell::messages::CellToBaseMsg::LeftAoI`].
    LeftAoI { entity_id: u32 },
    /// Buffered [`crate::cell::messages::CellToBaseMsg::EntityMethodCall`].
    EntityMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },
}

/// True iff the witness's session is still in the pre-`onClientReady`
/// world-entry window — i.e. either `pending_map_loaded` or
/// `pending_client_ready` is set, indicating the client has not yet
/// finished loading terrain and signalled it's ready for AoI traffic.
///
/// Returns `false` when the witness has no session entry (already
/// disconnected — the message would be dropped anyway) or when the
/// session is past `onClientReady`.
pub(crate) fn should_defer(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> bool {
    let Ok(clients) = connected.lock() else {
        return false;
    };
    let Some(state) = clients.get(&addr) else {
        return false;
    };
    state.pending_map_loaded.is_some() || state.pending_client_ready.is_some()
}

/// Push `msg` into the witness's deferred-AoI buffer. Caller has already
/// determined via [`should_defer`] that the session is pre-ready.
///
/// Silent no-op if the session has been removed between the check and
/// this call — the client disconnected mid-flight and any buffered work
/// would have been dropped by [`drain_deferred`] anyway.
pub(crate) fn push_deferred(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    msg: DeferredAoiMsg,
) {
    let Ok(mut clients) = connected.lock() else {
        return;
    };
    let Some(state) = clients.get_mut(&addr) else {
        return;
    };
    state.deferred_aoi_msgs.push(msg);
}

/// Drain and return all buffered AoI messages for this session, leaving
/// the buffer empty. Called by `handle_on_client_ready` once the client
/// signals it's ready to receive AoI traffic.
///
/// Returns an empty `Vec` if the session has no buffered messages or
/// has been removed.
pub(crate) fn drain_deferred(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Vec<DeferredAoiMsg> {
    let Ok(mut clients) = connected.lock() else {
        return Vec::new();
    };
    let Some(state) = clients.get_mut(&addr) else {
        return Vec::new();
    };
    std::mem::take(&mut state.deferred_aoi_msgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_default_connected_client_state;

    fn make_state_and_connected() -> (
        SocketAddr,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let addr: SocketAddr = "127.0.0.1:9876".parse().unwrap();
        let state = test_default_connected_client_state();
        let mut map = HashMap::new();
        map.insert(addr, state);
        (addr, Arc::new(Mutex::new(map)))
    }

    /// A freshly-constructed session has neither `pending_map_loaded` nor
    /// `pending_client_ready` set, so AoI dispatch should NOT defer.
    /// The default-state path is the steady-state hot path; if it ever
    /// returns true by mistake we'd silently buffer every AoI message
    /// forever.
    #[test]
    fn should_defer_returns_false_for_default_session() {
        let (addr, connected) = make_state_and_connected();
        assert!(
            !should_defer(&connected, addr),
            "default session has neither pending_map_loaded nor pending_client_ready"
        );
    }

    #[test]
    fn should_defer_returns_true_when_pending_client_ready_set() {
        let (addr, connected) = make_state_and_connected();
        {
            let mut clients = connected.lock().unwrap();
            let state = clients.get_mut(&addr).unwrap();
            state.pending_client_ready = Some(super::super::PendingClientReadyInfo {
                entity_id: 2,
                player_id: 63,
                world_name: "Test".into(),
                appearance_args: Vec::new(),
                tint_args: Vec::new(),
                first_login: 0,
            });
        }
        assert!(
            should_defer(&connected, addr),
            "pending_client_ready set → world entry in progress → defer"
        );
    }

    #[test]
    fn should_defer_returns_false_for_missing_session() {
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        assert!(
            !should_defer(&connected, addr),
            "missing session can't defer — message is just dropped by AoI handler"
        );
    }

    #[test]
    fn push_then_drain_round_trips_messages() {
        let (addr, connected) = make_state_and_connected();
        push_deferred(&connected, addr, DeferredAoiMsg::LeftAoI { entity_id: 100 });
        push_deferred(
            &connected,
            addr,
            DeferredAoiMsg::EntityMethodCall {
                entity_id: 100,
                method_index: 0x10,
                args: vec![0xAA, 0xBB],
            },
        );

        let drained = drain_deferred(&connected, addr);
        assert_eq!(drained.len(), 2, "both pushed messages drain in order");

        // After drain, the buffer is empty — a second drain yields zero.
        let drained_again = drain_deferred(&connected, addr);
        assert!(
            drained_again.is_empty(),
            "drain consumes the buffer; subsequent drain is empty"
        );
    }

    /// Pushing into a missing session is a silent no-op — happens when
    /// the client disconnects between the defer check and the push.
    /// Must not panic.
    #[test]
    fn push_into_missing_session_is_silent_noop() {
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        push_deferred(&connected, addr, DeferredAoiMsg::LeftAoI { entity_id: 1 });
        // No panic; no observable side effect. The drained buffer for
        // the (non-existent) session is empty.
        assert!(drain_deferred(&connected, addr).is_empty());
    }
}
