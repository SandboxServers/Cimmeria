//! Upload slots: a server-wide pool per route, and a per-address share of
//! it.
//!
//! A request holds a slot from the moment the gate admits it until its
//! body is read, expanded and replayed. Without the per-address share one
//! caller sending slow bodies could hold every slot of a route; with it a
//! peer address holds at most [`super::upload_gate::UploadLimits`]'
//! `*_slots_per_peer` of them, and the body read has a deadline besides.
//!
//! The per-address table only holds addresses with an upload in flight, so
//! it is bounded by the pool size and needs no eviction.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::upload_gate::Route;

/// In-flight uploads per `(route, address key)`.
#[derive(Debug, Default)]
pub(super) struct PeerSlots {
    in_flight: Mutex<HashMap<(Route, u64), usize>>,
}

/// One address's share of a route's slots, released on drop.
#[derive(Debug)]
pub(super) struct PeerSlot {
    owner: Arc<PeerSlots>,
    route: Route,
    key: u64,
}

impl PeerSlots {
    /// Take one of `per_peer` slots for `key` on `route`, or `None` when
    /// the address already holds all of them.
    pub(super) fn try_take(
        self: &Arc<Self>,
        route: Route,
        key: u64,
        per_peer: usize,
    ) -> Option<PeerSlot> {
        let mut map = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let n = map.entry((route, key)).or_insert(0);
        if *n >= per_peer {
            if *n == 0 {
                map.remove(&(route, key));
            }
            return None;
        }
        *n += 1;
        Some(PeerSlot {
            owner: Arc::clone(self),
            route,
            key,
        })
    }

    /// Addresses with an upload in flight, for tests.
    #[cfg(test)]
    pub(super) fn tracked(&self) -> usize {
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl Drop for PeerSlot {
    fn drop(&mut self) {
        let mut map = self
            .owner
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(n) = map.get_mut(&(self.route, self.key)) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                map.remove(&(self.route, self.key));
            }
        }
    }
}

/// What an admitted upload holds: its address's share and a server-wide
/// slot. Owned and `'static`, so it can move into a blocking worker and
/// stay held while the worker runs even if the request goes away.
#[derive(Debug)]
pub(super) struct UploadSlot {
    _peer: PeerSlot,
    _global: OwnedSemaphorePermit,
}

/// Take a slot for `key` on `route`: the address's share first, then one
/// of the route's pool.
pub(super) fn take_slot(
    peers: &Arc<PeerSlots>,
    pool: &Arc<Semaphore>,
    route: Route,
    key: u64,
    per_peer: usize,
) -> Option<UploadSlot> {
    let peer = peers.try_take(route, key, per_peer)?;
    let global = Arc::clone(pool).try_acquire_owned().ok()?;
    Some(UploadSlot {
        _peer: peer,
        _global: global,
    })
}
