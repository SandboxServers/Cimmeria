//! Lab chaos `Delay` (CH-07) as an AoI hold.
//!
//! The base's receive loop awaits `onClientReady` and `cancelMovie` inline,
//! so a delay slept inside the flush stopped packet processing for every
//! session, and near the 60 s cap let the server's inactivity reaper drop
//! them. It also slept after the pre-ready gate had opened and the buffer
//! had been drained, so a live `LeftAoI` could overtake the held creates and
//! leave a ghost. Holding instead fixes both: the hold keeps the witness's
//! AoI traffic buffered behind the introductions, and a spawned timer
//! releases the whole buffer, introductions first, once the delay is over.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::super::super::deferred_aoi::DeferredAoiMsg;
use super::super::super::world_entry::cell_dispatch::{flush_delay, log_flush_delayed};
use super::super::super::ConnectedClientState;
use super::{drain_and_lift, ReleaseReason};

/// A `Delay` is set for this session's player entity.
pub(super) fn wanted(addr: SocketAddr, witness_id: u32) -> bool {
    flush_delay(addr, witness_id).is_some()
}

/// For the hold `claimed`, just claimed by a release: with a `Delay` set,
/// spawn the drain behind it and return true. The hold stays up (and
/// claimed) until the timer's drain lifts it, so the session keeps
/// buffering and no other releaser can take it.
pub(super) fn defer_release(
    claimed: u64,
    reason: ReleaseReason,
    witness_id: u32,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let Some(delay) = flush_delay(addr, witness_id) else {
        return false;
    };
    let buffered_introductions = connected.lock().map_or(0, |clients| {
        clients.get(&addr).map_or(0, |state| {
            state
                .deferred_aoi_msgs
                .iter()
                .filter(|m| matches!(m, DeferredAoiMsg::EnteredAoI { .. }))
                .count()
        })
    });
    let witness_name =
        crate::base::session_identity::identity_for_addr(connected, addr).player_name;
    log_flush_delayed(witness_id, witness_name, delay, buffered_introductions);
    let transport = Arc::clone(transport);
    let connected = Arc::clone(connected);
    let entity_to_addr = Arc::clone(entity_to_addr);
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        drain_and_lift(
            claimed,
            reason,
            witness_id,
            addr,
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;
    });
    true
}
