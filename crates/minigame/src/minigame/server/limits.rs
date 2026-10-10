//! Connection limits for the public minigame port.
//!
//! The SFS port is published on every host interface, so anything on the
//! internet can open a socket to it. The original C++ host had none of
//! these limits (no connection cap, no read timeout); they exist because
//! the port is reachable, not because the SWFs need them.
//!
//! The numbers come from how the client behaves. One character runs at most
//! one minigame at a time (the registry is keyed by entity id), and the SWF
//! sends `verChk` and `login` itself as soon as it loads, before the player
//! touches anything. So a player holds one socket, briefly two while a
//! closed SWF's socket drains, and the handshake takes well under a second.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Limits applied by the minigame listener. `Default` is what the server
/// runs with; tests shrink them.
#[derive(Debug, Clone)]
pub struct ListenerLimits {
    /// Open connections across all peers. Further accepts are closed at
    /// once. Each live session holds exactly one.
    pub max_connections: usize,
    /// Open connections from one source address. Sized for several players
    /// behind one NAT, each with a session and a draining one.
    pub max_connections_per_ip: usize,
    /// Time from accept until `login` succeeds. A peer that has not logged
    /// in by then is closed.
    pub handshake_timeout: Duration,
    /// Time an authenticated session may go without sending a frame. On
    /// expiry the session ends as a cancel (result 0), the same as the
    /// player closing the window.
    pub idle_timeout: Duration,
}

impl Default for ListenerLimits {
    fn default() -> Self {
        Self {
            max_connections: 256,
            max_connections_per_ip: 8,
            handshake_timeout: Duration::from_secs(30),
            // Livewire's board can sit unstarted (timer stopped) while the
            // player reads it, so this is long: it reclaims abandoned
            // sockets, it is not a play-time limit.
            idle_timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// Why an accepted socket was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    TotalCap,
    PerIpCap,
}

impl Refusal {
    /// Stable `reason` field value for the log row.
    pub(super) fn reason(self) -> &'static str {
        match self {
            Refusal::TotalCap => "total_connection_cap",
            Refusal::PerIpCap => "per_ip_connection_cap",
        }
    }
}

#[derive(Debug, Default)]
struct Counts {
    total: usize,
    per_ip: HashMap<IpAddr, usize>,
}

/// Counts open connections; [`ConnectionPermit`] gives a slot back on drop.
#[derive(Debug, Clone)]
pub(super) struct ConnectionTracker {
    max_total: usize,
    max_per_ip: usize,
    counts: Arc<Mutex<Counts>>,
}

impl ConnectionTracker {
    /// A cap of 0 would refuse every connection, so a zero is raised to 1
    /// (with a WARN, since it is a configuration mistake).
    pub(super) fn new(limits: &ListenerLimits) -> Self {
        let at_least_one = |value: usize, field: &'static str| {
            if value == 0 {
                tracing::warn!(
                    field,
                    reason = "zero_connection_cap",
                    "Minigame connection cap of 0 would refuse everyone; using 1",
                );
            }
            value.max(1)
        };
        Self {
            max_total: at_least_one(limits.max_connections, "max_connections"),
            max_per_ip: at_least_one(limits.max_connections_per_ip, "max_connections_per_ip"),
            counts: Arc::default(),
        }
    }

    /// The effective total cap.
    pub(super) fn max_total(&self) -> usize {
        self.max_total
    }

    /// The effective per-address cap.
    pub(super) fn max_per_ip(&self) -> usize {
        self.max_per_ip
    }

    /// Take a slot for `ip`, or say which cap is full.
    pub(super) fn try_acquire(&self, ip: IpAddr) -> Result<ConnectionPermit, Refusal> {
        let mut counts = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        if counts.total >= self.max_total {
            return Err(Refusal::TotalCap);
        }
        let from_ip = counts.per_ip.entry(ip).or_insert(0);
        if *from_ip >= self.max_per_ip {
            return Err(Refusal::PerIpCap);
        }
        *from_ip += 1;
        counts.total += 1;
        Ok(ConnectionPermit {
            ip,
            counts: Arc::clone(&self.counts),
        })
    }

    /// Open connections, for the refusal log row.
    pub(super) fn open(&self) -> usize {
        self.counts.lock().unwrap_or_else(|p| p.into_inner()).total
    }
}

/// One open connection's slot. Held by the connection task for its whole
/// life, so every exit path gives the slot back.
#[derive(Debug)]
pub(super) struct ConnectionPermit {
    ip: IpAddr,
    counts: Arc<Mutex<Counts>>,
}

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        let mut counts = self.counts.lock().unwrap_or_else(|p| p.into_inner());
        counts.total = counts.total.saturating_sub(1);
        if let Some(n) = counts.per_ip.get_mut(&self.ip) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                // Keeps the map bounded by the addresses connected now.
                counts.per_ip.remove(&self.ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(total: usize, per_ip: usize) -> ListenerLimits {
        ListenerLimits {
            max_connections: total,
            max_connections_per_ip: per_ip,
            ..ListenerLimits::default()
        }
    }

    #[test]
    fn per_ip_cap_refuses_and_drop_frees_the_slot() {
        let tracker = ConnectionTracker::new(&limits(10, 2));
        let a: IpAddr = "203.0.113.1".parse().unwrap();
        let b: IpAddr = "203.0.113.2".parse().unwrap();
        let first = tracker.try_acquire(a).unwrap();
        let _second = tracker.try_acquire(a).unwrap();
        assert_eq!(tracker.try_acquire(a).unwrap_err(), Refusal::PerIpCap);
        // Another address is unaffected.
        let _other = tracker.try_acquire(b).unwrap();
        drop(first);
        assert!(tracker.try_acquire(a).is_ok());
    }

    #[test]
    fn total_cap_refuses_any_address() {
        let tracker = ConnectionTracker::new(&limits(2, 8));
        let _a = tracker.try_acquire("203.0.113.1".parse().unwrap()).unwrap();
        let _b = tracker.try_acquire("203.0.113.2".parse().unwrap()).unwrap();
        assert_eq!(
            tracker
                .try_acquire("203.0.113.3".parse().unwrap())
                .unwrap_err(),
            Refusal::TotalCap
        );
        assert_eq!(tracker.open(), 2);
    }

    /// A zero cap is a misconfiguration, not a request to refuse everyone.
    #[test]
    fn zero_caps_still_admit_one_connection() {
        let tracker = ConnectionTracker::new(&limits(0, 0));
        let ip: IpAddr = "203.0.113.4".parse().unwrap();
        let _only = tracker
            .try_acquire(ip)
            .expect("a zero cap is raised to 1, so the first connection is admitted");
        assert!(tracker.try_acquire(ip).is_err());
    }

    #[test]
    fn released_addresses_leave_the_map() {
        let tracker = ConnectionTracker::new(&limits(4, 4));
        drop(tracker.try_acquire("203.0.113.9".parse().unwrap()).unwrap());
        assert!(tracker.counts.lock().unwrap().per_ip.is_empty());
        assert_eq!(tracker.open(), 0);
    }
}
