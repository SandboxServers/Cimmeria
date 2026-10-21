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
use std::sync::{Arc, Mutex as StdMutex};

use super::{SessionRegistry, PENDING_SESSION_TTL};

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

/// Why `try_admit` refused a peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    /// `"unexpected_peer"`, `"unexpected_peer_with_sessions"` or
    /// `"no_free_ticket"`.
    pub reason: &'static str,
    /// Same count as `SessionRegistry::pending_count`, for the INFO row.
    pub pending_sessions: usize,
}

/// Holds one admission slot; the slot frees on drop.
#[derive(Debug)]
pub struct AdmissionGuard {
    key: AdmissionKey,
    admitted: Arc<StdMutex<HashMap<AdmissionKey, usize>>>,
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        // A std mutex because Drop cannot await. Never panic in drop: a
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
                "unexpected_peer"
            } else {
                "unexpected_peer_with_sessions"
            };
            return Err(AdmissionRefusal {
                reason,
                pending_sessions,
            });
        }
        let used = admitted.entry(key).or_insert(0);
        if *used >= slots {
            return Err(AdmissionRefusal {
                reason: "no_free_ticket",
                pending_sessions,
            });
        }
        *used += 1;
        drop(admitted);
        Ok(AdmissionGuard {
            key,
            admitted: Arc::clone(&self.admitted),
        })
    }
}

#[cfg(test)]
#[path = "session_admission_tests.rs"]
mod tests;
