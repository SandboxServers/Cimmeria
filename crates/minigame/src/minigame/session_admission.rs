//! Per-ticket connection admission (#1119).
//!
//! The listener used to cap connections globally and per IP. The owner's
//! rule is one connection per valid ticket instead: a peer is admitted only
//! while the registry holds a session for its address that no live
//! connection has already used up.
//!
//! This is a child module of `session` so it can read the registry's private
//! fields; the counting has to happen under the same lock that guards
//! `sessions`, or two simultaneous accepts could both see a free slot.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use super::{SessionRegistry, SessionRegistryInner, PENDING_SESSION_TTL};

/// What a peer address is matched on. Every non-public peer shares one key:
/// behind a LAN or a Docker bridge, the TCP peer can be a different private
/// address from the one the base recorded at `start_minigame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum AdmissionKey {
    Public(IpAddr),
    NonPublic,
}

/// Canonicalize first, so an IPv4-mapped IPv6 peer keys like its IPv4 form.
pub(super) fn admission_key(ip: IpAddr) -> AdmissionKey {
    let ip = ip.to_canonical();
    let non_public = match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                // CGNAT 100.64.0.0/10
                || (o[0] == 100 && (o[1] & 0xc0) == 0x40)
        }
        IpAddr::V6(v6) => {
            let s0 = v6.segments()[0];
            v6.is_loopback()
                // ULA fc00::/7
                || (s0 & 0xfe00) == 0xfc00
                // link-local fe80::/10
                || (s0 & 0xffc0) == 0xfe80
        }
    };
    if non_public {
        AdmissionKey::NonPublic
    } else {
        AdmissionKey::Public(ip)
    }
}

/// Why `try_admit` refused a peer. The list is closed; each variant is one
/// `reason` value on the listener's refusal row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionRefusalReason {
    /// No session is registered at all: most likely a scanner.
    UnexpectedPeer,
    /// Sessions exist, but none (still live) expects this address.
    UnexpectedPeerWithSessions,
    /// Every ticket for this address already has a connection holding it.
    NoFreeTicket,
}

impl AdmissionRefusalReason {
    /// The `reason` field value on the refusal row.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedPeer => "unexpected_peer",
            Self::UnexpectedPeerWithSessions => "unexpected_peer_with_sessions",
            Self::NoFreeTicket => "no_free_ticket",
        }
    }
}

/// Why `try_admit` refused a peer, and how many sessions were pending then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    /// What kind of refusal this was.
    pub reason: AdmissionRefusalReason,
    /// How many registered sessions had no connection yet, for the INFO row.
    pub pending_sessions: usize,
}

/// The shared state behind one admission slot. The slot frees once, from
/// whichever of the guard's drop, a cancel, or the end of the game loop gets
/// there first; later calls do nothing.
#[derive(Debug)]
pub(super) struct Slot {
    key: AdmissionKey,
    admitted: Arc<StdMutex<HashMap<AdmissionKey, usize>>>,
    released: AtomicBool,
}

impl Slot {
    /// Give the slot back. Idempotent.
    fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        // A std mutex because Drop cannot await. Never panic here: a
        // poisoned map is still a valid counter map.
        let mut map = self
            .admitted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(n) = map.get_mut(&self.key) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                map.remove(&self.key);
            }
        }
    }
}

/// Holds one admission slot; the slot frees when the guard drops, and sooner
/// when the ticket is cancelled or the connection stops playing.
///
/// Owner ruling (2026-10-21): release on cancel. The connection binds its
/// guard to its ticket after the login claims the session
/// ([`SessionRegistry::bind_slot`]). From then on the slot frees at the first
/// of: the registry cancelling that ticket (the player closed the window), the
/// game loop ending for any reason including a closed socket
/// ([`SessionRegistry::release_slot`]), or the connection task ending (the
/// guard dropping, which also covers a refused login). A player who closes and
/// at once reopens a minigame is not refused while the old socket drains. A
/// second concurrent connection on a live ticket is still refused.
#[derive(Debug)]
pub struct AdmissionGuard {
    slot: Arc<Slot>,
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        self.slot.release();
    }
}

impl SessionRegistryInner {
    /// Free and forget the slot bound to `entity_id`'s `ticket`, if any.
    /// Called under the `inner` lock, which is the lock order `try_admit` uses.
    pub(super) fn release_bound_slot(&mut self, entity_id: u32, ticket: &str) {
        if self
            .bound_slots
            .get(&entity_id)
            .is_some_and(|(bound, _)| bound == ticket)
        {
            if let Some((_, slot)) = self.bound_slots.remove(&entity_id) {
                if let Some(slot) = slot.upgrade() {
                    slot.release();
                }
            }
        }
    }
}

impl SessionRegistry {
    /// Admit one connection from `ip` if a ticket is free for it.
    ///
    /// Lock order is always `inner` (tokio) then `admitted` (std), and both
    /// are held across the count and the increment. Counting under `inner`
    /// alone would let two accepts both read "one free slot" before either
    /// incremented. Connected sessions count as slots because their
    /// connection still holds a guard, so N tickets admit exactly N.
    pub async fn try_admit(&self, ip: IpAddr) -> Result<AdmissionGuard, AdmissionRefusal> {
        let key = admission_key(ip);
        let inner = self.inner.lock().await;
        let mut admitted = self
            .admitted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let slots = inner
            .sessions
            .values()
            .filter(|s| {
                s.client_ip.map(admission_key) == Some(key)
                    && (s.connected || s.created_at.elapsed() < PENDING_SESSION_TTL)
            })
            .count();
        let pending_sessions = inner.sessions.values().filter(|s| !s.connected).count();

        if slots == 0 {
            let reason = if inner.sessions.is_empty() {
                AdmissionRefusalReason::UnexpectedPeer
            } else {
                AdmissionRefusalReason::UnexpectedPeerWithSessions
            };
            return Err(AdmissionRefusal {
                reason,
                pending_sessions,
            });
        }
        let used = admitted.entry(key).or_insert(0);
        if *used >= slots {
            return Err(AdmissionRefusal {
                reason: AdmissionRefusalReason::NoFreeTicket,
                pending_sessions,
            });
        }
        *used += 1;
        drop(admitted);
        Ok(AdmissionGuard {
            slot: Arc::new(Slot {
                key,
                admitted: Arc::clone(&self.admitted),
                released: AtomicBool::new(false),
            }),
        })
    }

    /// Tie `guard` to the session `ticket` of `entity_id`, once the login has
    /// claimed it, so a cancel or [`Self::release_slot`] can free the slot
    /// without waiting for the connection task to end.
    ///
    /// If the ticket is already gone (cancelled between the claim and now),
    /// the slot frees at once.
    pub async fn bind_slot(&self, entity_id: u32, ticket: &str, guard: &AdmissionGuard) {
        let mut inner = self.inner.lock().await;
        if inner
            .sessions
            .get(&entity_id)
            .is_some_and(|s| s.ticket == ticket)
        {
            inner
                .bound_slots
                .insert(entity_id, (ticket.to_owned(), Arc::downgrade(&guard.slot)));
        } else {
            guard.slot.release();
        }
    }

    /// Free the slot bound to this ticket now. The connection calls it when
    /// its game loop ends (closed socket, idle timeout, a finished game),
    /// before the close frames, so a stalled teardown cannot hold the slot.
    /// Does nothing if no slot is bound or it already freed.
    pub async fn release_slot(&self, entity_id: u32, ticket: &str) {
        self.inner
            .lock()
            .await
            .release_bound_slot(entity_id, ticket);
    }
}

#[cfg(test)]
#[path = "session_admission_tests.rs"]
mod tests;
