//! Ring transporter state machine.
//!
//! Direct port of `python/cell/RingTransporter.py`. Each [`RingTransporter`]
//! owns one ring pad's runtime state: who's currently standing on it, what
//! state the FSM is in, and any pending timer deadlines.
//!
//! This module holds the struct, the timer block and the shared
//! reset/abort machinery. The per-side transitions live in siblings so no
//! one file carries the whole graph:
//!
//! - [`source`] — `enter_send_wait` → `start_sending` → hide → warmup.
//! - [`destination`] — `remote_wait` → … → `cooldown_timer_expired`.
//! - [`effects`] — the [`Effect`] value type the FSM emits.
//! - [`manager`] — the cross-region registry, the injectable clock, and
//!   the pair-abort / player-forget entry points.
//!
//! Timing is tick-driven (100ms cadence) — see
//! [`super::runtime::run_tick_with_engine`]. The Python original used
//! `Atrea.addTimer` (game-time deadlines), which maps cleanly onto
//! `Instant`-based deadlines we poll each tick.
//!
//! State graph (from the Python original):
//!
//! ```text
//!   IDLE ──interact()──▶ (no state change; sends destination list)
//!   IDLE ──selectDestination()──▶ SEND_WAIT      ── t+60  stall abort
//!   SEND_WAIT ──player on pad──▶ SEND_WARMUP   ── t+0   teleport_out
//!                                                ── t+3.5 hide
//!                                                ── t+4.0 teleport
//!                                              ──▶ REMOTE_LOAD_WAIT
//!   REMOTE_LOAD_WAIT ──all loaded──▶ REMOTE_WARMUP
//!                                       ── t+0 teleport_in
//!                                       ── t+3.0 unhide
//!                                     ──▶ COOLDOWN
//!                                       ── t+2.5 unlock + fire teleport_in
//!                                     ──▶ IDLE
//! ```
//!
//! Every state that waits on something *outside* this FSM (a player walking
//! onto the pad, the peer ring, a client finishing a world load) also carries
//! a bounded stall deadline; see [`STALL_TIMEOUTS`]. Without it a single
//! stalled trip parks the ring in a non-`Idle` state forever, and
//! [`super::runtime::handle_select_destination`] refuses any destination that
//! is not `Idle` — so one stall removes that pad from every peer in an
//! all-to-all mesh (Harset regions 4-8). That is audit defect H-B3.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::regions::RingRegion;

mod destination;
mod effects;
mod manager;
mod source;

#[cfg(test)]
mod tests;

pub use effects::{Effect, RegionEvent};
pub use manager::{RawDeadline, RingTransporterManager};

/// Hide-everyone delay after warmup begins (Python: 3.5s).
pub const HIDE_DELAY: Duration = Duration::from_millis(3_500);
/// Source-side warmup duration before teleport (Python: 4.0s).
pub const WARMUP_DELAY: Duration = Duration::from_millis(4_000);
/// Destination-side delay between teleport_in sequence and unhide (Python: 3.0s).
pub const REMOTE_WARMUP_DELAY: Duration = Duration::from_millis(3_000);
/// Destination-side cooldown before unlock (Python: 2.5s).
pub const COOLDOWN_DELAY: Duration = Duration::from_millis(2_500);

/// Source: how long a ring waits for the player to walk onto the pad after
/// they pick a destination. Generous because the player may be several
/// seconds of walking away from the pad they just used the console on.
pub const SEND_WAIT_TIMEOUT: Duration = Duration::from_secs(60);
/// Destination: how long the far ring holds itself reserved for a source
/// that has not begun sending.
///
/// Deliberately **longer** than [`SEND_WAIT_TIMEOUT`] so the source always
/// aborts first. If the destination expired first it would return to `Idle`,
/// a third ring could reserve it, and the original source's teleport would
/// then land in a trip it does not belong to. The back-pointer check in
/// `kick_off_warmup` is the structural guard; this margin is the cheap one.
pub const RECV_WAIT_TIMEOUT: Duration = Duration::from_secs(65);
/// Destination: bound on `RecvWarmup`, which normally lasts exactly
/// [`WARMUP_DELAY`] (4s) — the source's own warmup. 15s is headroom for tick
/// lag, not a real wait.
pub const RECV_WARMUP_TIMEOUT: Duration = Duration::from_secs(15);
/// Destination: bound on `RemoteLoadWait`, the cross-world client world-load
/// round trip (`GateTravel` → `onClientReady` → `AdvanceRingDestination`).
///
/// Long on purpose. A *false* abort here is worse than a late one: once the
/// ring leaves `RemoteLoadWait`, a late `AdvanceRingDestination` is silently
/// dropped by `try_advance_after_load`'s readiness gate, which eats the
/// `Effect::FireTeleportIn` chain event and with it any arrival mission
/// credit. `handle_remote_player_loaded` has a late-arrival recovery path
/// for exactly that case, but the timeout should not be the thing that
/// exercises it.
pub const REMOTE_LOAD_WAIT_TIMEOUT: Duration = Duration::from_secs(90);

/// Stall timeout per state, as a table so the arm sites and the tests read
/// the same source. States with their own bounded deadline (`SendWarmup`,
/// `RemoteWarmup`, `Cooldown`) and `Idle` are absent — see
/// [`stall_timeout_for`].
pub const STALL_TIMEOUTS: [(State, Duration); 4] = [
    (State::SendWait, SEND_WAIT_TIMEOUT),
    (State::RecvWait, RECV_WAIT_TIMEOUT),
    (State::RecvWarmup, RECV_WARMUP_TIMEOUT),
    (State::RemoteLoadWait, REMOTE_LOAD_WAIT_TIMEOUT),
];

/// Bounded stall timeout for `state`, or `None` when the state already has a
/// deadline of its own (or is `Idle`).
pub fn stall_timeout_for(state: State) -> Option<Duration> {
    STALL_TIMEOUTS
        .iter()
        .find(|(s, _)| *s == state)
        .map(|(_, d)| *d)
}

/// FSM state. Numeric values match the Python class constants so logs line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Idle = 0,
    SendWait = 1,
    SendWarmup = 2,
    RemoteLoadWait = 3,
    RemoteWarmup = 4,
    Cooldown = 5,
    RecvWait = 6,
    RecvWarmup = 7,
}

/// Why a trip was torn down. Carried into the warn log so ops can tell a
/// stalled player apart from a dropped one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortReason {
    /// The state's bounded deadline elapsed.
    Timeout,
    /// A participating player's cell entity went away (disconnect, GM
    /// despawn, respawn, non-ring teleport) and the trip has nobody left.
    PlayerGone,
    /// The peer ring aborted and dragged this end with it.
    PeerAborted,
    /// `run_one_deadline` could not resolve the destination region at warmup
    /// time — `ring_regions` and `ring_transporters` disagree.
    DestinationRegionMissing,
}

impl AbortReason {
    /// Stable short string for the `reason` field of the negative log, per
    /// `docs/architecture/negative-logging-convention.md`.
    pub fn as_str(self) -> &'static str {
        match self {
            AbortReason::Timeout => "stall_timeout",
            AbortReason::PlayerGone => "player_gone",
            AbortReason::PeerAborted => "peer_aborted",
            AbortReason::DestinationRegionMissing => "destination_region_missing",
        }
    }
}

/// Per-region pending timer deadlines. Each is `Some(_)` only while waiting
/// for the corresponding transition.
#[derive(Debug, Default, Clone)]
struct Timers {
    /// Source: when to call `setVisible(false)` on every player on the pad.
    hide_at: Option<Instant>,
    /// Source: when to actually teleport players (Python `__warmupTimerExpired`).
    warmup_at: Option<Instant>,
    /// Destination: when to call `setVisible(true)` (Python `__remoteWarmupTimerExpired`).
    remote_warmup_at: Option<Instant>,
    /// Destination: when to unlock movement + fire `teleport_in` (Python `__cooldownTimerExpired`).
    cooldown_at: Option<Instant>,
    /// Bounded abort deadline for the four states that otherwise wait
    /// forever: `SendWait`, `RecvWait`, `RecvWarmup`, `RemoteLoadWait`.
    ///
    /// One field rather than four because those states are mutually
    /// exclusive on a single transporter **and** never overlap the four
    /// above on that same transporter: `SendWarmup` owns hide+warmup,
    /// `RemoteWarmup` owns remote_warmup, `Cooldown` owns cooldown, and a
    /// new trip cannot start on a non-`Idle` ring (`validate_destination`).
    stall_at: Option<Instant>,
}

/// One ring pad's runtime state.
#[derive(Debug, Clone)]
pub struct RingTransporter {
    pub region_id: i32,
    pub world_name: String,
    pub position: [f32; 3],
    pub destination_ids: Vec<i32>,
    pub event_set_id: i32,
    pub point_set_id: i32,
    /// Mirrors `RingRegion.required_mission_id` — when set, runtime gates
    /// reject `interact()` and `selectDestination()` unless the player has
    /// completed this mission. Loaded once from `ring_transport_regions`
    /// and immutable thereafter.
    pub required_mission_id: Option<i32>,

    pub state: State,
    /// Players currently standing on this ring pad (entity_id → unit).
    ///
    /// These are *not* trip participants: nobody in here has been sent
    /// `LockMovement` or `HidePlayer` yet, so an abort must not emit
    /// `ShowPlayer`/`UnlockMovement` for them.
    pub players: HashMap<u32, ()>,
    /// Players who picked a destination on this pad and have not yet walked
    /// onto it — the `SendWait` occupants.
    ///
    /// The Python original tracked nobody here: `selectDestination` set the
    /// state and the trip's only link to its owner was an instance attribute
    /// on the player. That is why a disconnect during `SendWait` used to
    /// leave the pad reserved forever with no participant to blame — the
    /// player is in `players` only once they physically arrive, and in
    /// `send_players` only once warmup starts.
    ///
    /// Not a trip participant for release purposes: a reserver has been sent
    /// neither `LockMovement` nor `HidePlayer`.
    pub reserved_by: Vec<u32>,
    /// Players being sent (snapshot taken when warmup begins, so late arrivals
    /// don't get teleported mid-cycle).
    pub send_players: Vec<u32>,
    /// Destination side: players that have completed loading on this ring.
    pub players_loaded: Vec<u32>,
    /// Destination side: the players the source said it teleported, **by id**.
    ///
    /// The Python original passed a bare count (`remoteCountUpdate`) and so
    /// did the first Rust port. Keeping the ids means a participant that
    /// disconnects mid-load can be removed from the expectation instead of
    /// decrementing an opaque counter that can underflow, and it means an
    /// abort in `RemoteLoadWait` can un-hide the passengers who are in
    /// flight but not yet loaded. [`Self::num_remote_players`] is the
    /// derived count the readiness check still uses.
    pub expected_players: Vec<u32>,
    /// Cross-link: destination region id while the FSM is busy.
    pub remote_region_id: Option<i32>,

    timers: Timers,
}

impl RingTransporter {
    pub fn from_region(region: &RingRegion) -> Self {
        Self {
            region_id: region.region_id,
            world_name: region.world_name.clone(),
            position: [region.x, region.y, region.z],
            destination_ids: region.destination_ids.clone(),
            event_set_id: region.event_set_id,
            point_set_id: region.point_set_id,
            required_mission_id: region.required_mission_id,
            state: State::Idle,
            players: HashMap::new(),
            reserved_by: Vec::new(),
            send_players: Vec::new(),
            players_loaded: Vec::new(),
            expected_players: Vec::new(),
            remote_region_id: None,
            timers: Timers::default(),
        }
    }

    /// Number of players the source said it teleported. Derived from
    /// [`Self::expected_players`] so the two can never disagree.
    pub fn num_remote_players(&self) -> u32 {
        self.expected_players.len() as u32
    }

    /// Arm the bounded stall deadline for the current state, or clear it if
    /// the current state does not have one.
    ///
    /// Call this on **every** transition. A state change that forgets to
    /// re-arm silently converts a healthy ring into an unbounded one; a
    /// state change that forgets to disarm leaves a stale deadline that
    /// aborts a perfectly good trip later.
    pub(super) fn arm_stall(&mut self, now: Instant) {
        self.timers.stall_at = stall_timeout_for(self.state).map(|d| now + d);
    }

    /// Return every player that is an actual participant in the current
    /// trip — the set that has been sent `LockMovement` (in
    /// `start_sending`) and, past [`HIDE_DELAY`], `HidePlayer`.
    ///
    /// Deliberately excludes [`Self::players`]: pad occupants were never
    /// locked or hidden, and broadcasting a stray `onVisible(1)` for them
    /// would churn visibility for all their witnesses.
    fn trip_participants(&self) -> Vec<u32> {
        let mut out = self.send_players.clone();
        for &eid in self
            .expected_players
            .iter()
            .chain(self.players_loaded.iter())
        {
            if !out.contains(&eid) {
                out.push(eid);
            }
        }
        out
    }

    /// Return this ring to `Idle`, clearing every timer and all transient
    /// trip state. Pure bookkeeping — emits nothing.
    ///
    /// Use this instead of writing `state = State::Idle` by hand: an ad-hoc
    /// assignment leaves the stall deadline armed on an `Idle` transporter,
    /// which then aborts the *next* trip.
    pub(crate) fn reset_to_idle(&mut self) {
        self.state = State::Idle;
        self.timers = Timers::default();
        self.reserved_by.clear();
        self.send_players.clear();
        self.players_loaded.clear();
        self.expected_players.clear();
        self.remote_region_id = None;
    }

    /// Tear the current trip down and return to `Idle`, releasing every
    /// participant.
    ///
    /// Effect order is **show then unlock**, matching the happy path
    /// (`remote_warmup_timer_expired` shows, `cooldown_timer_expired`
    /// unlocks afterwards). Unlocking first would open a window in which the
    /// player can move while witnesses still hold them hidden — and because
    /// `send_visible` computes its fan-out from `get_witnesses_of` at call
    /// time, a witness who enters AoI inside that window never receives the
    /// `onVisible(1)` and renders a permanently invisible avatar.
    ///
    /// `skip` is the entity whose teardown triggered the abort, if any; it
    /// is already gone from the space so effects addressed to it would only
    /// produce a witness-lookup miss.
    pub(super) fn abort_to_idle(&mut self, skip: Option<u32>) -> Vec<Effect> {
        let mut effects = Vec::new();
        for eid in self.trip_participants() {
            if Some(eid) == skip {
                continue;
            }
            effects.push(Effect::ShowPlayer { entity_id: eid });
            effects.push(Effect::UnlockMovement { entity_id: eid });
        }
        self.reset_to_idle();
        effects
    }

    /// Player just entered the source pad's point-set region (or left).
    /// Mirrors `RingTransporter.regionTriggered`.
    pub fn region_triggered(&mut self, entering: bool, entity_id: u32) {
        if entering {
            self.players.insert(entity_id, ());
        } else {
            self.players.remove(&entity_id);
        }
        // When in SendWait, an arrival kicks off the actual transport. The
        // Python TODO note about other states is preserved — we do nothing
        // there to match its behavior exactly.
        // Caller is responsible for noticing the SendWait transition; see
        // `start_sending_if_ready`.
    }

    /// Returns true if [`Self::start_sending`] should be called immediately.
    pub fn should_auto_start(&self) -> bool {
        self.state == State::SendWait && !self.players.is_empty()
    }

    /// `interact()` from Python — generates the destination-list effect.
    /// Called when a player triggers the ring (chain 1043's
    /// `trigger_transporter` action).
    pub fn interact(&self, entity_id: u32) -> Effect {
        Effect::SendDestinationList {
            entity_id,
            source_region_id: self.region_id,
            destinations: self.destination_ids.clone(),
        }
    }

    /// Validate that `destination_id` is permissible. Mirrors the three guards
    /// at the top of `selectDestination`.
    pub fn validate_destination(&self, destination_id: i32) -> Result<(), &'static str> {
        if !self.destination_ids.contains(&destination_id) {
            return Err("destination not in region's destination list");
        }
        if destination_id == self.region_id {
            return Err("source and destination cannot be the same");
        }
        if self.state != State::Idle {
            return Err("source ring is busy");
        }
        Ok(())
    }

    /// Returns the next deadline that has elapsed at `now`, if any. Probed
    /// each tick. Returns the *kind* of deadline so the caller knows which
    /// state-transition method to call.
    ///
    /// `Stall` is checked **last** on purpose: if the exclusivity invariant
    /// on [`Timers::stall_at`] is ever violated by a future change, the real
    /// transition should win over the abort, not the other way round.
    fn elapsed_deadline(&self, now: Instant) -> Option<DeadlineKind> {
        if let Some(t) = self.timers.hide_at {
            if now >= t {
                return Some(DeadlineKind::Hide);
            }
        }
        if let Some(t) = self.timers.warmup_at {
            if now >= t {
                return Some(DeadlineKind::Warmup);
            }
        }
        if let Some(t) = self.timers.remote_warmup_at {
            if now >= t {
                return Some(DeadlineKind::RemoteWarmup);
            }
        }
        if let Some(t) = self.timers.cooldown_at {
            if now >= t {
                return Some(DeadlineKind::Cooldown);
            }
        }
        if let Some(t) = self.timers.stall_at {
            if now >= t {
                return Some(DeadlineKind::Stall);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeadlineKind {
    Hide,
    Warmup,
    RemoteWarmup,
    Cooldown,
    Stall,
}
