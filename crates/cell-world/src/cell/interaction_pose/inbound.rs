//! The Mercury sequence of the last interaction call per actor (PC-26, D-PC19).
//!
//! The cell router notes the `packet_seq` of each client `interact` and
//! `useItem` call; the pose row takes it when the row is written. Telemetry
//! only: nothing here changes game state.

use super::geometry::InteractionKind;
use cimmeria_wire::cell::cell_methods::{inventory, player};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How old a noted call may be and still be taken by a pose row.
pub const INBOUND_MAX_AGE: Duration = Duration::from_secs(5);

/// Map size at which entries older than [`INBOUND_MAX_AGE`] are dropped.
const PRUNE_AT: usize = 2048;

/// Map slot of the `interact` call.
const SLOT_INTERACT: u8 = 0;

/// Map slot of the `useItem` call.
const SLOT_USE_ITEM: u8 = 1;

/// The newest noted call per `(actor entity id, slot)`: its seq and arrival time.
static LAST_INBOUND: LazyLock<Mutex<HashMap<(u32, u8), (u32, Instant)>>> =
    LazyLock::new(Mutex::default);

/// The slot a cell method notes under, or `None` for a method PC-26 ignores.
fn slot_for_method(method_index: u16) -> Option<u8> {
    match method_index {
        player::INTERACT => Some(SLOT_INTERACT),
        inventory::USE_ITEM => Some(SLOT_USE_ITEM),
        _ => None,
    }
}

/// The slot a pose kind takes from.
fn slot_for_kind(kind: InteractionKind) -> u8 {
    match kind {
        InteractionKind::Interact => SLOT_INTERACT,
        InteractionKind::UseItem => SLOT_USE_ITEM,
    }
}

/// Note a client cell method call. Only `interact` and `useItem` are kept; a `None` seq is ignored.
pub fn note_inbound(entity_id: u32, method_index: u16, packet_seq: Option<u32>) {
    note_inbound_at(entity_id, method_index, packet_seq, Instant::now());
}

/// Note a client cell method call as of `now`. See [`note_inbound`].
pub fn note_inbound_at(entity_id: u32, method_index: u16, packet_seq: Option<u32>, now: Instant) {
    let (Some(seq), Some(slot)) = (packet_seq, slot_for_method(method_index)) else {
        return;
    };
    let mut map = LAST_INBOUND.lock().unwrap_or_else(PoisonError::into_inner);
    if map.len() >= PRUNE_AT {
        map.retain(|_, (_, at)| now.saturating_duration_since(*at) < INBOUND_MAX_AGE);
    }
    map.insert((entity_id, slot), (seq, now));
}

/// Take the noted seq for this actor and kind if it is younger than [`INBOUND_MAX_AGE`].
/// A seq is taken once: a second take, or a take of an old seq, returns `None`.
pub fn take_inbound(entity_id: u32, kind: InteractionKind, now: Instant) -> Option<u32> {
    let mut map = LAST_INBOUND.lock().unwrap_or_else(PoisonError::into_inner);
    let (seq, at) = map.remove(&(entity_id, slot_for_kind(kind)))?;
    (now.saturating_duration_since(at) < INBOUND_MAX_AGE).then_some(seq)
}
