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
//! 3. the authenticated lab-mcp tools, the only writers (CH-05). A
//!    source-scan test in `cimmeria-server` (`lab_chaos_callers`) holds the
//!    callers of [`install`] and [`current`] to an allowlist.
//!
//! Chaos is scoped to one session, keyed by its client address and tagged
//! with the player entity it was set for. [`LabChaos::set`] refuses a TTL of
//! 0 or over [`MAX_TTL_S`], so every entry expires within 30 minutes and a
//! lab run that dies never leaves a session degraded; the default
//! ([`DEFAULT_TTL_S`]) is the caller's to apply. Session teardown clears the
//! address's entry, so a later session on that address (a relaunched client)
//! starts clean. The net shim (CH-06) and the first-login flush (CH-07) read
//! it per send through [`LabChaos::net_for`] and [`LabChaos::flush_for`].

/// True only in a binary built with the `lab-chaos` feature.
pub const COMPILED: bool = cfg!(feature = "lab-chaos");

#[cfg(feature = "lab-chaos")]
use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::BuildHasher,
    net::SocketAddr,
    sync::{Mutex, MutexGuard, OnceLock, PoisonError},
    time::{Duration, Instant},
};

/// The `net` profile's knobs, resolved to integers. Loss and duplication are
/// per thousand so the profile's one-decimal percentages stay exact. Each
/// bound is checked by [`LabChaos::set`].
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub struct NetChaos {
    /// Added delay on each server send, in milliseconds. At most 5000.
    pub latency_ms: u32,
    /// Random spread on top of `latency_ms`, in milliseconds. At most 5000.
    pub jitter_ms: u32,
    /// Packets dropped, per thousand. At most 1000 (every packet).
    pub loss_per_thousand: u32,
    /// Packets sent twice, per thousand. At most 1000.
    pub dup_per_thousand: u32,
    /// Reorder buffer size, in packets; 0 keeps order. At most 1000.
    pub reorder: u32,
    /// Seeds the per-address RNG, so a run replays.
    pub seed: u64,
}

/// The `flush` profile: how the first-login AoI flush is disturbed. Each
/// bound is checked by [`LabChaos::set`].
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub enum FlushChaos {
    /// Hold the whole flush back by `ms` milliseconds. At most 60 000.
    Delay { ms: u32 },
    /// Shuffle the introductions inside each phase with a `seed`ed RNG,
    /// keeping each entity's phase-1 record whole.
    Reorder { seed: u64 },
    /// The pre-#1341-mitigation shape: one single-packet phase-1 bundle of up
    /// to `records`. From 1 to [`PACK_RECORDS_MAX`].
    Pack { records: u16 },
    /// Prepend `len` bytes of filler to a single-packet phase-1 bundle.
    /// 0 (no filler) or 2..=[`SHIFT_LEN_MAX`]: `updateFrequencyNotification`
    /// (0x02, exactly 2 bytes) for 2, `voiceData` (0x33, 3 to 36) for 3 to
    /// 36, and both for 37 and 38. No message is 1 byte long.
    Shift { len: u16 },
    /// Shift by a seeded length, a new one per flush.
    ShiftSweep { seed: u64 },
}

/// One session's chaos. Both profiles may be on at once.
#[cfg(feature = "lab-chaos")]
#[derive(Debug, Clone, PartialEq)]
pub struct SessionChaos {
    /// The player entity the chaos was set for. Status reports this, never
    /// the address.
    pub entity_id: u32,
    /// The CH-01 spec header text, such as `flush:pack seed=7`. The set WARN
    /// logs it as `level`.
    pub label: String,
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
    #[error("ttl {0}s is over the {cap} s cap", cap = MAX_TTL_S)]
    TtlTooLong(u32),
    /// A TTL of 0 would never let the entry live; one too large for the
    /// clock would never let it expire.
    #[error("ttl {0}s is not a usable expiry")]
    BadTtl(u32),
    #[error("bad chaos knob: {0}")]
    BadKnob(&'static str),
}

/// The TTL a caller applies when the request names none (D-CH3). The
/// registry does not apply it: [`LabChaos::set`] takes an explicit TTL.
#[cfg(feature = "lab-chaos")]
pub const DEFAULT_TTL_S: u32 = 300;
/// The longest TTL [`LabChaos::set`] accepts (D-CH3). Longer requests are
/// refused, never clamped.
#[cfg(feature = "lab-chaos")]
pub const MAX_TTL_S: u32 = 1800;

/// The registry. Keyed by the session's client address.
///
/// One mutex guards the whole map and every read takes it, so a reader on a
/// per-packet path (CH-06's shim) pays one uncontended lock per send even
/// when no chaos is set. CH-06 should consider a cheap "map is empty" check
/// before calling [`LabChaos::net_for`] on its hot path.
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
    /// An empty registry with its own `addr_hash` key. Tests build their own;
    /// the server's one comes from [`install`].
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            addr_hasher: RandomState::new(),
        }
    }

    /// Sets (or replaces) one session's chaos and returns when it expires.
    /// Refuses a TTL of 0 or over [`MAX_TTL_S`], a knob out of its bound, or
    /// a request with neither profile. Expired entries of every session are
    /// swept here, so the map never outgrows the sessions under live chaos.
    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &self,
        addr: SocketAddr,
        entity_id: u32,
        label: String,
        net: Option<NetChaos>,
        flush: Option<FlushChaos>,
        ttl_s: u32,
        now: Instant,
    ) -> Result<Instant, ChaosRefused> {
        if ttl_s == 0 {
            return Err(ChaosRefused::BadTtl(ttl_s));
        }
        if ttl_s > MAX_TTL_S {
            return Err(ChaosRefused::TtlTooLong(ttl_s));
        }
        if net.is_none() && flush.is_none() {
            return Err(ChaosRefused::BadKnob("set needs a net or a flush profile"));
        }
        if let Some(net) = &net {
            check_net(net)?;
        }
        if let Some(flush) = &flush {
            check_flush(flush)?;
        }
        let expires_at = now
            .checked_add(Duration::from_secs(u64::from(ttl_s)))
            .ok_or(ChaosRefused::BadTtl(ttl_s))?;
        let chaos = SessionChaos {
            entity_id,
            label,
            net,
            flush,
            expires_at,
        };
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_set",
            profile = profile_label(&chaos),
            level = %chaos.label,
            addr_hash = %self.addr_hash(addr),
            seed = chaos_seed(&chaos),
            expires_in_s = ttl_s,
            "lab chaos set on a session"
        );
        let mut sessions = self.sessions();
        self.sweep_expired(&mut sessions, now);
        sessions.insert(addr, chaos);
        Ok(expires_at)
    }

    /// Clears one session, or every session with `None`. Returns how many
    /// entries were removed, which the clear tool reports. Logs INFO when it
    /// removed something or cleared everything; a per-session clear that
    /// found nothing (every session teardown on a lab server) logs DEBUG.
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
        let scope = if addr.is_some() { "session" } else { "all" };
        if cleared > 0 || addr.is_none() {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope,
                addr_hash = addr_hash.as_deref(),
                cleared,
                "lab chaos cleared"
            );
        } else {
            tracing::debug!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope,
                addr_hash = addr_hash.as_deref(),
                cleared,
                "lab chaos cleared (nothing set)"
            );
        }
        cleared
    }

    /// Removes the entry at `addr` when it was set for another entity than
    /// `entity_id`, and says whether it did. A caller that knows which player
    /// entity now owns `addr` uses it to drop chaos a previous owner left
    /// behind (see the CH-04 worknote for CH-05 and CH-07).
    pub fn clear_if_not(&self, addr: SocketAddr, entity_id: u32) -> bool {
        let removed = {
            let mut sessions = self.sessions();
            let stale = sessions
                .get(&addr)
                .is_some_and(|chaos| chaos.entity_id != entity_id);
            if stale {
                sessions.remove(&addr);
            }
            stale
        };
        if removed {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "stale_entity",
                addr_hash = %self.addr_hash(addr),
                cleared = 1,
                "lab chaos cleared: set for another entity on this address"
            );
        }
        removed
    }

    /// Expired entries are removed here and read as None.
    pub fn net_for(&self, addr: SocketAddr, now: Instant) -> Option<NetChaos> {
        self.live(addr, now, |chaos| chaos.net.clone())
    }

    /// Expired entries are removed here and read as None.
    pub fn flush_for(&self, addr: SocketAddr, now: Instant) -> Option<FlushChaos> {
        self.live(addr, now, |chaos| chaos.flush.clone())
    }

    /// Every live session's chaos, sorted by entity id. Expired entries are
    /// removed. Carries no address: callers report the entity id.
    pub fn status(&self, now: Instant) -> Vec<SessionChaos> {
        let mut sessions = self.sessions();
        self.sweep_expired(&mut sessions, now);
        let mut live: Vec<SessionChaos> = sessions.values().cloned().collect();
        live.sort_by_key(|chaos| chaos.entity_id);
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

    fn sweep_expired(&self, sessions: &mut HashMap<SocketAddr, SessionChaos>, now: Instant) {
        let expired: Vec<SocketAddr> = sessions
            .iter()
            .filter(|(_, chaos)| is_expired(chaos, now))
            .map(|(addr, _)| *addr)
            .collect();
        for addr in expired {
            sessions.remove(&addr);
            self.log_expired(addr);
        }
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
fn check_net(net: &NetChaos) -> Result<(), ChaosRefused> {
    if net.loss_per_thousand > 1000 {
        return Err(ChaosRefused::BadKnob("net loss is over 1000 per thousand"));
    }
    if net.dup_per_thousand > 1000 {
        return Err(ChaosRefused::BadKnob("net dup is over 1000 per thousand"));
    }
    if net.latency_ms > 5000 {
        return Err(ChaosRefused::BadKnob("net latency is over 5000 ms"));
    }
    if net.jitter_ms > 5000 {
        return Err(ChaosRefused::BadKnob("net jitter is over 5000 ms"));
    }
    if net.reorder > 1000 {
        return Err(ChaosRefused::BadKnob("net reorder is over 1000 packets"));
    }
    Ok(())
}

/// The longest shift filler: `updateFrequencyNotification` (2 bytes) plus a
/// 36-byte `voiceData`. 38 is the 1-byte residue class plus one record
/// (37), which reaches every cursor a 1-byte filler would (CH-07 filler
/// design, `docs/analysis/lab-chaos/worknotes/CH-07-filler.md`).
#[cfg(feature = "lab-chaos")]
pub const SHIFT_LEN_MAX: u16 = 38;

/// One phase-1 introduction record: CREATE_ENTITY (11 bytes) then
/// UPDATE_AVATAR (26 bytes).
#[cfg(feature = "lab-chaos")]
pub const PHASE1_RECORD_LEN: u16 = 37;

/// The most records a single-packet phase-1 bundle holds with the longest
/// filler ahead of them (34). `Pack` is refused above it rather than
/// clamped, so a run never silently tests a smaller bundle than it asked for.
#[cfg(feature = "lab-chaos")]
pub const PACK_RECORDS_MAX: u16 = ((cimmeria_mercury::packet::FRAGMENT_BODY_SIZE
    - SHIFT_LEN_MAX as usize)
    / PHASE1_RECORD_LEN as usize) as u16;

#[cfg(feature = "lab-chaos")]
fn check_flush(flush: &FlushChaos) -> Result<(), ChaosRefused> {
    match *flush {
        FlushChaos::Delay { ms } if ms > 60_000 => {
            Err(ChaosRefused::BadKnob("flush delay is over 60000 ms"))
        }
        FlushChaos::Pack { records } if !(1..=PACK_RECORDS_MAX).contains(&records) => Err(
            ChaosRefused::BadKnob("flush pack records is not 1 to 34 (one packet)"),
        ),
        FlushChaos::Shift { len } if len != 0 && !(2..=SHIFT_LEN_MAX).contains(&len) => {
            Err(ChaosRefused::BadKnob("flush shift len is not 0 or 2 to 38"))
        }
        _ => Ok(()),
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
///
/// A startup-only gate: the registry is never uninstalled, so `install(false)`
/// after `install(true)` returns `Err(NotDeveloperMode)` while [`current`]
/// stays `Some`. Only `crates/server/src/main.rs` calls it.
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
mod tests;
