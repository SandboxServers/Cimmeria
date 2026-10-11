//! Lab chaos: per-session fault injection for the live research lab
//! (lab-chaos campaign, `docs/analysis/lab-chaos/`).
//!
//! Three gates keep it away from players (decisions D-CH2 and D-CH3):
//!
//! 1. the `lab-chaos` cargo feature. Only [`COMPILED`] exists without it, and
//!    no crate turns the feature on through a dependency, so an ordinary
//!    build (and the workspace-hack) never compiles the registry in;
//! 2. `DEVELOPER_MODE` at runtime: [`install`] refuses without it, and with
//!    nothing installed [`current`] is `None` and every hook is a no-op;
//! 3. the authenticated lab-mcp tools, the only writers (CH-05).
//!
//! Chaos is scoped to one session, keyed by its client address, and every
//! entry expires ([`DEFAULT_TTL_S`], capped at [`MAX_TTL_S`]) so a lab run
//! that dies never leaves a session degraded. The net shim (CH-06) and the
//! first-login flush (CH-07) read it per send through [`LabChaos::net_for`]
//! and [`LabChaos::flush_for`].

/// True only in a binary built with the `lab-chaos` feature.
pub const COMPILED: bool = cfg!(feature = "lab-chaos");

#[cfg(feature = "lab-chaos")]
use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::BuildHasher,
    net::SocketAddr,
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::Instant,
};

/// The `net` profile's knobs, resolved to integers. Loss and duplication are
/// per thousand so the profile's one-decimal percentages stay exact.
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub struct NetChaos {
    pub latency_ms: u32,
    pub jitter_ms: u32,
    pub loss_per_thousand: u32,
    pub dup_per_thousand: u32,
    pub reorder: u32,
    pub seed: u64,
}

/// The `flush` profile: how the first-login AoI flush is disturbed.
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub enum FlushChaos {
    Delay {
        ms: u32,
    },
    Reorder {
        seed: u64,
    },
    /// The pre-#1341-mitigation shape: one single-packet phase-1 bundle of up to `records`.
    Pack {
        records: u16,
    },
    /// Prepend one filler message of `len` bytes to the phase-1 bundle.
    Shift {
        len: u16,
    },
    /// Shift by a seeded length, a new one per flush.
    ShiftSweep {
        seed: u64,
    },
}

/// One session's chaos. Both profiles may be on at once.
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub struct SessionChaos {
    pub net: Option<NetChaos>,
    pub flush: Option<FlushChaos>,
    pub expires_at: Instant,
}

/// Why a chaos request was refused. The lab-mcp tools return the text.
#[cfg(feature = "lab-chaos")]
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChaosRefused {
    #[error("lab chaos needs DEVELOPER_MODE")]
    NotDeveloperMode,
    #[error("no session for entity {0}")]
    NoSession(u32),
    #[error("ttl {0}s is over the 1800 s cap")]
    TtlTooLong(u32),
}

#[cfg(feature = "lab-chaos")]
pub const DEFAULT_TTL_S: u32 = 300;
#[cfg(feature = "lab-chaos")]
pub const MAX_TTL_S: u32 = 1800;

/// The registry. Keyed by the session's client address.
#[cfg(feature = "lab-chaos")]
#[derive(Debug)]
pub struct LabChaos {
    sessions: Mutex<HashMap<SocketAddr, SessionChaos>>,
    /// Keys `addr_hash` in the logs. Random per process, so the hash
    /// correlates one run's rows without being a lookup table back to the
    /// address (an IPv4 space is small enough to hash exhaustively).
    addr_hasher: RandomState,
}

#[cfg(feature = "lab-chaos")]
impl Default for LabChaos {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "lab-chaos")]
impl LabChaos {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            addr_hasher: RandomState::new(),
        }
    }

    /// Sets (or replaces) one session's chaos.
    pub fn set(&self, addr: SocketAddr, chaos: SessionChaos) {
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_set",
            profile = profile_label(&chaos),
            addr_hash = %self.addr_hash(addr),
            seed = chaos_seed(&chaos),
            expires_in_s = chaos.expires_at.saturating_duration_since(Instant::now()).as_secs(),
            "lab chaos set on a session"
        );
        self.sessions().insert(addr, chaos);
    }

    /// Clears one session, or every session with `None`. Returns how many
    /// entries were removed, which the clear tool reports.
    pub fn clear(&self, addr: Option<SocketAddr>) -> usize {
        let cleared = {
            let mut sessions = self.sessions();
            match addr {
                Some(addr) => usize::from(sessions.remove(&addr).is_some()),
                None => {
                    let n = sessions.len();
                    sessions.clear();
                    n
                }
            }
        };
        let addr_hash = addr.map(|a| self.addr_hash(a));
        tracing::info!(
            target: "lab.chaos",
            event = "lab_chaos_cleared",
            scope = if addr.is_some() { "session" } else { "all" },
            addr_hash = addr_hash.as_deref(),
            cleared,
            "lab chaos cleared"
        );
        cleared
    }

    /// Expired entries are removed here and read as None.
    pub fn net_for(&self, addr: SocketAddr, now: Instant) -> Option<NetChaos> {
        self.live(addr, now, |chaos| chaos.net.clone())
    }

    /// Expired entries are removed here and read as None.
    pub fn flush_for(&self, addr: SocketAddr, now: Instant) -> Option<FlushChaos> {
        self.live(addr, now, |chaos| chaos.flush.clone())
    }

    /// Every live session, sorted by address. Expired entries are removed.
    pub fn status(&self, now: Instant) -> Vec<(SocketAddr, SessionChaos)> {
        let mut sessions = self.sessions();
        let expired: Vec<SocketAddr> = sessions
            .iter()
            .filter(|(_, chaos)| is_expired(chaos, now))
            .map(|(addr, _)| *addr)
            .collect();
        for addr in expired {
            sessions.remove(&addr);
            self.log_expired(addr);
        }
        let mut live: Vec<_> = sessions
            .iter()
            .map(|(addr, chaos)| (*addr, chaos.clone()))
            .collect();
        live.sort_by_key(|(addr, _)| *addr);
        live
    }

    fn live<R>(
        &self,
        addr: SocketAddr,
        now: Instant,
        read: impl FnOnce(&SessionChaos) -> Option<R>,
    ) -> Option<R> {
        let mut sessions = self.sessions();
        let chaos = sessions.get(&addr)?;
        if is_expired(chaos, now) {
            sessions.remove(&addr);
            self.log_expired(addr);
            return None;
        }
        read(chaos)
    }

    /// A panic while the lock was held cannot leave the map half-written
    /// (every change is one insert or remove), so a poisoned lock is still
    /// safe to use; refusing would wedge every send the shim filters.
    fn sessions(&self) -> MutexGuard<'_, HashMap<SocketAddr, SessionChaos>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn addr_hash(&self, addr: SocketAddr) -> String {
        format!("{:016x}", self.addr_hasher.hash_one(addr))
    }

    fn log_expired(&self, addr: SocketAddr) {
        tracing::info!(
            target: "lab.chaos",
            event = "lab_chaos_expired",
            addr_hash = %self.addr_hash(addr),
            "lab chaos expired on a session"
        );
    }
}

#[cfg(feature = "lab-chaos")]
fn is_expired(chaos: &SessionChaos, now: Instant) -> bool {
    now >= chaos.expires_at
}

#[cfg(feature = "lab-chaos")]
fn profile_label(chaos: &SessionChaos) -> &'static str {
    match (chaos.net.is_some(), chaos.flush.is_some()) {
        (true, true) => "net+flush",
        (true, false) => "net",
        (false, true) => "flush",
        (false, false) => "none",
    }
}

/// The seed that replays this chaos: the net seed, else the flush seed, else
/// 0 for the flush modes that are not random (delay, pack, a fixed shift).
#[cfg(feature = "lab-chaos")]
fn chaos_seed(chaos: &SessionChaos) -> u64 {
    if let Some(net) = &chaos.net {
        return net.seed;
    }
    match chaos.flush {
        Some(FlushChaos::Reorder { seed } | FlushChaos::ShiftSweep { seed }) => seed,
        _ => 0,
    }
}

#[cfg(feature = "lab-chaos")]
static REGISTRY: OnceLock<LabChaos> = OnceLock::new();

/// Installed once at startup by the server, only when DEVELOPER_MODE is true.
#[cfg(feature = "lab-chaos")]
pub fn install(developer_mode: bool) -> Result<&'static LabChaos, ChaosRefused> {
    if !developer_mode {
        return Err(ChaosRefused::NotDeveloperMode);
    }
    Ok(REGISTRY.get_or_init(LabChaos::new))
}

/// None when not installed; every hook is then a no-op.
#[cfg(feature = "lab-chaos")]
pub fn current() -> Option<&'static LabChaos> {
    REGISTRY.get()
}

#[cfg(all(test, feature = "lab-chaos"))]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use std::time::Duration;
    use tracing::Level;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    fn net_chaos(seed: u64, expires_at: Instant) -> SessionChaos {
        SessionChaos {
            net: Some(NetChaos {
                latency_ms: 120,
                jitter_ms: 0,
                loss_per_thousand: 10,
                dup_per_thousand: 0,
                reorder: 0,
                seed,
            }),
            flush: None,
            expires_at,
        }
    }

    /// The runtime gate. This is the only test that touches the process-wide
    /// registry, so the "nothing installed" check cannot race another test.
    #[test]
    fn install_refuses_without_developer_mode() {
        assert_eq!(install(false).err(), Some(ChaosRefused::NotDeveloperMode));
        assert!(
            current().is_none(),
            "a refused install leaves nothing installed"
        );
        let installed = install(true).expect("DEVELOPER_MODE installs");
        assert!(current().is_some_and(|c| std::ptr::eq(c, installed)));
    }

    #[test]
    fn expired_entry_reads_as_none() {
        let chaos = LabChaos::new();
        let now = Instant::now();
        let past = now - Duration::from_secs(1);
        chaos.set(addr(40_001), net_chaos(7, past));
        chaos.set(
            addr(40_002),
            SessionChaos {
                net: None,
                flush: Some(FlushChaos::Pack { records: 40 }),
                expires_at: past,
            },
        );
        assert_eq!(chaos.net_for(addr(40_001), now), None);
        assert_eq!(chaos.flush_for(addr(40_002), now), None);
        assert!(
            chaos.status(now).is_empty(),
            "expired entries are removed, not just hidden"
        );
    }

    #[test]
    fn clear_none_clears_every_session() {
        let chaos = LabChaos::new();
        let now = Instant::now();
        let later = now + Duration::from_secs(u64::from(DEFAULT_TTL_S));
        chaos.set(addr(40_011), net_chaos(1, later));
        chaos.set(addr(40_012), net_chaos(2, later));
        chaos.set(addr(40_013), net_chaos(3, later));
        assert_eq!(chaos.clear(Some(addr(40_011))), 1);
        assert_eq!(chaos.status(now).len(), 2);
        assert_eq!(chaos.clear(None), 2);
        assert!(chaos.status(now).is_empty());
        assert_eq!(chaos.net_for(addr(40_012), now), None);
        assert_eq!(chaos.net_for(addr(40_013), now), None);
    }

    #[test]
    fn scope_is_per_address() {
        let chaos = LabChaos::new();
        let now = Instant::now();
        let later = now + Duration::from_secs(u64::from(DEFAULT_TTL_S));
        chaos.set(addr(40_021), net_chaos(9, later));
        assert_eq!(chaos.net_for(addr(40_021), now).map(|n| n.seed), Some(9));
        // Same IP, other port: a second client on the same machine.
        assert_eq!(chaos.net_for(addr(40_022), now), None);
        assert_eq!(chaos.flush_for(addr(40_022), now), None);
    }

    /// The set WARN names the session by a hash only: the logs reach SigNoz
    /// and Discord, and a lab address is still a player-style address.
    #[test]
    fn set_logs_a_hash_never_the_address() {
        let capture = LogCapture::install();
        let chaos = LabChaos::new();
        let later = Instant::now() + Duration::from_secs(u64::from(DEFAULT_TTL_S));
        chaos.set(addr(40_031), net_chaos(7, later));
        let row = capture
            .find_message(Level::WARN, "lab chaos set")
            .expect("the set WARN");
        assert_eq!(row.target, "lab.chaos");
        assert!(row.has_field("profile", "net"));
        assert!(row.has_field("seed", "7"));
        assert!(row.fields.contains_key("expires_in_s"));
        let hash = row.fields.get("addr_hash").expect("addr_hash");
        assert!(
            hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "addr_hash is a 64-bit hex hash: {hash}"
        );
        for value in row.fields.values() {
            assert!(!value.contains("127.0.0.1"), "address leaked: {value}");
        }
    }
}
