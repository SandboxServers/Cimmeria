//! Ring transporter state machine + manager.
//!
//! Direct port of `python/cell/RingTransporter.py`. Each [`RingTransporter`]
//! owns one ring pad's runtime state: who's currently standing on it, what
//! state the FSM is in, and any pending timer deadlines.
//!
//! Timing is tick-driven (100ms cadence) — see [`ring_transport_tick`]. The
//! Python original used `Atrea.addTimer` (game-time deadlines), which maps
//! cleanly onto `Instant`-based deadlines we poll each tick.
//!
//! State graph (from the Python original):
//!
//! ```text
//!   IDLE ──interact()──▶ (no state change; sends destination list)
//!   IDLE ──selectDestination()──▶ SEND_WAIT
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

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::regions::RingRegion;

/// Hide-everyone delay after warmup begins (Python: 3.5s).
pub const HIDE_DELAY: Duration = Duration::from_millis(3_500);
/// Source-side warmup duration before teleport (Python: 4.0s).
pub const WARMUP_DELAY: Duration = Duration::from_millis(4_000);
/// Destination-side delay between teleport_in sequence and unhide (Python: 3.0s).
pub const REMOTE_WARMUP_DELAY: Duration = Duration::from_millis(3_000);
/// Destination-side cooldown before unlock (Python: 2.5s).
pub const COOLDOWN_DELAY: Duration = Duration::from_millis(2_500);

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
}


/// Effects produced by FSM transitions. The state machine never touches the
/// world directly; the high-level executor consumes these and dispatches them
/// (sends wire methods, teleports entities, fires chain events).
///
/// Mirrors the Python side-effects in order — each variant maps 1:1 to a
/// specific call site (e.g. `Effect::PlaySequence` ⇆ `player.playSequence(...)`,
/// `Effect::TeleportPlayer` ⇆ `player.teleportTo(...)`).
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Play a Kismet sequence on the given player (used for both Teleport_Out
    /// and Teleport_In — Python only fires it on the first player to keep the
    /// matinee from desyncing).
    PlaySequence { entity_id: u32, event_set_id: i32, region_event: RegionEvent },
    /// Fire `teleport::out` script event on the given player.
    /// Currently no chain triggers register on this in our content engine, but
    /// we emit it for parity in case content scripts add one later.
    OnTeleportOut { entity_id: u32, region_id: i32, destination_id: i32 },
    /// Set `BSF_MovementLock` on the player and broadcast onStateFieldUpdate.
    LockMovement { entity_id: u32 },
    /// Clear `BSF_MovementLock` on the player and broadcast onStateFieldUpdate.
    UnlockMovement { entity_id: u32 },
    /// Send `onVisible(false)` to make the player invisible.
    HidePlayer { entity_id: u32 },
    /// Send `onVisible(true)` to restore the player.
    ShowPlayer { entity_id: u32 },
    /// Move player to the destination ring's position. Same world (cross-world
    /// is not yet supported — see [`Effect::TeleportCrossWorld`]).
    TeleportPlayer { entity_id: u32, position: [f32; 3], world_name: String, destination_region_id: i32 },
    /// Cross-world teleport — currently unimplemented; emitted as a warn so
    /// future work can wire it up without changing the FSM.
    TeleportCrossWorld { entity_id: u32, position: [f32; 3], world_name: String, destination_region_id: i32 },
    /// Fire the `teleport_in` content engine event with `region_id` as the key.
    /// Chain 1044 (Castle_CellBlock) hooks this to complete mission 640.
    FireTeleportIn { entity_id: u32, region_id: i32 },
    /// Send `onRingTransporterList` to the player's client.
    SendDestinationList { entity_id: u32, source_region_id: i32, destinations: Vec<i32> },
}

/// Which Kismet sequence to look up in the region's event set.
///
/// Matches `Atrea.enums.Region_Teleport_Out` (8000) and `Region_Teleport_In`
/// (8001) — see `docs/gameplay/ring-transport-system.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionEvent {
    TeleportOut,
    TeleportIn,
}

impl RegionEvent {
    /// Numeric event ID used in the `event_sets_sequences` lookup.
    pub fn event_id(self) -> i32 {
        match self {
            RegionEvent::TeleportOut => 8000,
            RegionEvent::TeleportIn => 8001,
        }
    }
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

    pub state: State,
    /// Players currently standing on this ring pad (entity_id → unit).
    pub players: HashMap<u32, ()>,
    /// Players being sent (snapshot taken when warmup begins, so late arrivals
    /// don't get teleported mid-cycle).
    pub send_players: Vec<u32>,
    /// Destination side: players that have completed loading on this ring.
    pub players_loaded: Vec<u32>,
    /// Destination side: number of players the source said it teleported.
    pub num_remote_players: u32,
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
            state: State::Idle,
            players: HashMap::new(),
            send_players: Vec::new(),
            players_loaded: Vec::new(),
            num_remote_players: 0,
            remote_region_id: None,
            timers: Timers::default(),
        }
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

    /// Move to `SendWait` and remember the remote region ID. Caller must
    /// drive the remote transporter into `RecvWait` separately.
    pub fn enter_send_wait(&mut self, destination_id: i32) {
        self.state = State::SendWait;
        self.remote_region_id = Some(destination_id);
    }

    /// `remoteWait()` from Python — driven by the source transporter when a
    /// destination is selected. Source ring → SendWait, destination → RecvWait.
    pub fn remote_wait(&mut self, source_region_id: i32) {
        debug_assert_eq!(self.state, State::Idle);
        self.state = State::RecvWait;
        self.remote_region_id = Some(source_region_id);
    }

    /// `__startSending()` from Python: transition `SendWait → SendWarmup`,
    /// snapshot the current players, schedule the hide and warmup timers, and
    /// return the side-effect list. The caller must also poke the remote
    /// transporter into `RecvWarmup` via [`RingTransporter::remote_send`]
    /// after dispatching these effects.
    pub fn start_sending(&mut self, now: Instant) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWait);
        self.state = State::SendWarmup;

        // Snapshot players before clearing — we need to keep teleporting them
        // even if more arrive during warmup, and we must not double-teleport
        // anyone who steps off the pad.
        self.send_players = self.players.keys().copied().collect();
        self.send_players.sort_unstable(); // deterministic order for "first player" sequence pick
        self.players.clear();

        let mut effects = Vec::new();

        // Python fires the kismet on only the first player — playing it for
        // every player desyncs the matinee. Pick the lowest entity_id so the
        // choice is deterministic across runs.
        if let Some(&first) = self.send_players.first() {
            effects.push(Effect::PlaySequence {
                entity_id: first,
                event_set_id: self.event_set_id,
                region_event: RegionEvent::TeleportOut,
            });
        }

        let dst_id = self.remote_region_id.unwrap_or(0);
        for &eid in &self.send_players {
            effects.push(Effect::OnTeleportOut {
                entity_id: eid,
                region_id: self.region_id,
                destination_id: dst_id,
            });
            effects.push(Effect::LockMovement { entity_id: eid });
        }

        self.timers.hide_at = Some(now + HIDE_DELAY);
        self.timers.warmup_at = Some(now + WARMUP_DELAY);
        effects
    }

    /// `remoteSend()` from Python: destination's `RecvWait → RecvWarmup`.
    /// Currently a state-only transition because we don't fire any wire
    /// effects on this transition (Python's `__beginTransport` runs on the
    /// source; the destination just notes it's now warming up).
    pub fn remote_send(&mut self) {
        debug_assert_eq!(self.state, State::RecvWait);
        self.state = State::RecvWarmup;
    }

    /// `remoteCountUpdate()` — destination receives the number of players the
    /// source is teleporting. Used for the all-players-loaded check.
    pub fn remote_count_update(&mut self, num: u32) {
        self.num_remote_players = num;
    }

    /// `remoteTransport()` — destination's `RecvWarmup → RemoteLoadWait`.
    /// Pure state transition; effects flow from the source's
    /// `warmup_timer_expired`.
    pub fn remote_transport(&mut self) {
        debug_assert_eq!(self.state, State::RecvWarmup);
        self.state = State::RemoteLoadWait;
        self.players_loaded.clear();
    }

    /// Hide-timer: source-side `setVisible(false)` for every snapshotted player.
    /// Mirrors `__hideTimerExpired`.
    pub fn hide_timer_expired(&mut self) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWarmup);
        self.timers.hide_at = None;
        self.send_players.iter()
            .map(|&eid| Effect::HidePlayer { entity_id: eid })
            .collect()
    }

    /// Warmup-timer: source-side teleport. Mirrors `__warmupTimerExpired` +
    /// `__doTransport`.
    ///
    /// Caller must use [`Self::remote_count_update`] + [`Self::remote_transport`]
    /// on the destination ring after this returns.
    pub fn warmup_timer_expired(&mut self, dst_position: [f32; 3], dst_world: &str) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWarmup);
        self.state = State::RemoteLoadWait;
        self.timers.warmup_at = None;
        let dst_region = self.remote_region_id.unwrap_or(0);
        self.send_players.iter()
            .map(|&eid| {
                if dst_world == self.world_name {
                    Effect::TeleportPlayer {
                        entity_id: eid,
                        position: dst_position,
                        world_name: dst_world.to_string(),
                        destination_region_id: dst_region,
                    }
                } else {
                    Effect::TeleportCrossWorld {
                        entity_id: eid,
                        position: dst_position,
                        world_name: dst_world.to_string(),
                        destination_region_id: dst_region,
                    }
                }
            })
            .collect()
    }

    /// `playerLoaded()` — destination side. Returns true when the count of
    /// loaded players matches the source's `num_remote_players`.
    pub fn player_loaded(&mut self, entity_id: u32) -> bool {
        if !self.players_loaded.contains(&entity_id) {
            self.players_loaded.push(entity_id);
        }
        self.players_loaded.len() as u32 == self.num_remote_players
    }

    /// `__allPlayersLoaded()` — destination's `RemoteLoadWait → RemoteWarmup`.
    /// Returns the Teleport_In sequence effect (only the first player triggers
    /// the matinee, see Python comment), and arms the remote-warmup timer.
    /// The caller is responsible for short-circuiting to
    /// [`Self::remote_warmup_timer_expired`] when no players loaded — Python
    /// does the same.
    pub fn all_players_loaded(&mut self, now: Instant) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::RemoteLoadWait);
        self.state = State::RemoteWarmup;

        if self.players_loaded.is_empty() {
            // Match Python's branch: no sequence, no timer arm; caller will
            // call `remote_warmup_timer_expired` immediately.
            return Vec::new();
        }

        let mut effects = Vec::new();
        // Python: only the first player triggers the kismet matinee.
        if let Some(&first) = self.players_loaded.first() {
            effects.push(Effect::PlaySequence {
                entity_id: first,
                event_set_id: self.event_set_id,
                region_event: RegionEvent::TeleportIn,
            });
        }
        self.timers.remote_warmup_at = Some(now + REMOTE_WARMUP_DELAY);
        effects
    }

    /// Remote-warmup timer: destination's `setVisible(true)` for every loaded
    /// player. Transitions to Cooldown and arms the cooldown timer.
    pub fn remote_warmup_timer_expired(&mut self, now: Instant) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::RemoteWarmup);
        self.state = State::Cooldown;
        self.timers.remote_warmup_at = None;

        let effects: Vec<Effect> = self.players_loaded.iter()
            .map(|&eid| Effect::ShowPlayer { entity_id: eid })
            .collect();

        if !self.players_loaded.is_empty() {
            self.timers.cooldown_at = Some(now + COOLDOWN_DELAY);
        } else {
            // Python: no players loaded → skip straight to cooldown timer expiry.
            // Surface this as a "fire now" by leaving cooldown_at unset and
            // letting the caller invoke `cooldown_timer_expired` directly.
            // We can't recursively return effects from two timer methods here
            // without tangling state, so the manager handles the fast path.
            // Push an empty list — manager checks `players_loaded.is_empty()`.
        }
        // Emit unlock + teleport_in chain events from cooldown_timer_expired —
        // not here. Keeps the side-effect grouping aligned with Python.
        effects
    }

    /// Cooldown timer: unlock movement and fire the `teleport_in` chain event
    /// for each loaded player. Returns to Idle.
    pub fn cooldown_timer_expired(&mut self) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::Cooldown);
        self.state = State::Idle;
        self.timers.cooldown_at = None;
        let region_id = self.region_id;

        let mut effects = Vec::with_capacity(self.players_loaded.len() * 2);
        for &eid in &self.players_loaded {
            effects.push(Effect::UnlockMovement { entity_id: eid });
            effects.push(Effect::FireTeleportIn { entity_id: eid, region_id });
        }
        // Reset destination-side state in preparation for the next run.
        self.players_loaded.clear();
        self.send_players.clear();
        self.num_remote_players = 0;
        self.remote_region_id = None;
        effects
    }

    /// Returns the next deadline that has elapsed at `now`, if any. Probed
    /// each tick. Returns the *kind* of deadline so the caller knows which
    /// state-transition method to call.
    fn elapsed_deadline(&self, now: Instant) -> Option<DeadlineKind> {
        if let Some(t) = self.timers.hide_at      { if now >= t { return Some(DeadlineKind::Hide); } }
        if let Some(t) = self.timers.warmup_at    { if now >= t { return Some(DeadlineKind::Warmup); } }
        if let Some(t) = self.timers.remote_warmup_at { if now >= t { return Some(DeadlineKind::RemoteWarmup); } }
        if let Some(t) = self.timers.cooldown_at  { if now >= t { return Some(DeadlineKind::Cooldown); } }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeadlineKind {
    Hide,
    Warmup,
    RemoteWarmup,
    Cooldown,
}

/// Process-wide registry of ring transporters (one entry per region_id).
///
/// All worlds' transporters live in the same map because cross-world rings
/// need to find each other by region_id.
#[derive(Debug, Default)]
pub struct RingTransporterManager {
    pub regions: HashMap<i32, RingTransporter>,
}

impl RingTransporterManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build one transporter per region in the supplied table. Idempotent —
    /// re-init clears any in-flight FSM state (use only at startup).
    pub fn load(&mut self, ring_regions: &HashMap<i32, RingRegion>) {
        self.regions.clear();
        for (id, region) in ring_regions {
            self.regions.insert(*id, RingTransporter::from_region(region));
        }
    }

    pub fn get(&self, region_id: i32) -> Option<&RingTransporter> {
        self.regions.get(&region_id)
    }

    pub fn get_mut(&mut self, region_id: i32) -> Option<&mut RingTransporter> {
        self.regions.get_mut(&region_id)
    }

    /// Return all region_ids that have an elapsed deadline at `now`. Used by
    /// the tick to drive timers forward.
    pub fn ready_regions(&self, now: Instant) -> Vec<(i32, RawDeadline)> {
        self.regions.iter()
            .filter_map(|(id, r)| r.elapsed_deadline(now).map(|dk| (*id, RawDeadline(dk))))
            .collect()
    }
}

/// Opaque deadline-kind handle returned by `ready_regions`. Wrapping it
/// keeps `DeadlineKind` private while still letting the tick code dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawDeadline(DeadlineKind);

impl RawDeadline {
    pub(crate) fn is_hide(self) -> bool { self.0 == DeadlineKind::Hide }
    pub(crate) fn is_warmup(self) -> bool { self.0 == DeadlineKind::Warmup }
    pub(crate) fn is_remote_warmup(self) -> bool { self.0 == DeadlineKind::RemoteWarmup }
    pub(crate) fn is_cooldown(self) -> bool { self.0 == DeadlineKind::Cooldown }
}

/// Tick entry point — re-exports the runtime tick so callers can stick to
/// `super::ring_transport::ring_transport_tick`. The actual effect dispatch
/// lives in [`crate::cell::ring_transport_runtime`] (effect → wire) so this
/// module stays test-friendly.
pub async fn ring_transport_tick(
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    crate::cell::ring_transport_runtime::run_tick_with_engine(tx, space_mgr, engine).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(id: i32, world: &str, dests: Vec<i32>) -> RingRegion {
        RingRegion {
            region_id: id,
            world_id: 12,
            world_name: world.to_string(),
            x: 1.0, y: 2.0, z: 3.0,
            tag: format!("Ring{id}"),
            height: 1.7, radius: 3.5,
            event_set_id: 100, display_name_id: 7508,
            destination_ids: dests, point_set_id: 200,
        }
    }

    #[test]
    fn fsm_starts_idle() {
        let r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
        assert_eq!(r.state, State::Idle);
        assert!(r.players.is_empty());
    }

    #[test]
    fn validate_destination_rejects_self_busy_unknown() {
        let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2, 3]));
        assert_eq!(r.validate_destination(99), Err("destination not in region's destination list"));
        assert_eq!(r.validate_destination(1), Err("destination not in region's destination list"));
        // Self-as-dest is filtered at load time, so it'd hit the "not in list" branch first.
        // Force a self-list entry to test the source==dest guard.
        r.destination_ids.push(1);
        assert_eq!(r.validate_destination(1), Err("source and destination cannot be the same"));

        r.state = State::SendWait;
        assert_eq!(r.validate_destination(2), Err("source ring is busy"));
    }

    #[test]
    fn idle_to_send_wait_via_enter_send_wait() {
        let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
        r.enter_send_wait(2);
        assert_eq!(r.state, State::SendWait);
        assert_eq!(r.remote_region_id, Some(2));
    }

    #[test]
    fn remote_wait_idle_to_recv_wait() {
        let mut r = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
        r.remote_wait(1);
        assert_eq!(r.state, State::RecvWait);
        assert_eq!(r.remote_region_id, Some(1));
    }

    #[test]
    fn full_cycle_source_side() {
        let mut src = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
        src.enter_send_wait(2);

        // Player walks onto the pad
        src.region_triggered(true, 100);
        assert!(src.should_auto_start());

        let now = Instant::now();
        let effs = src.start_sending(now);
        assert_eq!(src.state, State::SendWarmup);
        // Should produce: PlaySequence (1) + OnTeleportOut + LockMovement = 3 for one player.
        assert!(matches!(effs[0], Effect::PlaySequence { .. }));
        assert!(matches!(effs[1], Effect::OnTeleportOut { region_id: 1, destination_id: 2, .. }));
        assert!(matches!(effs[2], Effect::LockMovement { entity_id: 100 }));
        assert_eq!(effs.len(), 3);

        // Hide timer
        let hide_now = now + HIDE_DELAY;
        let effs = src.hide_timer_expired();
        assert_eq!(effs.len(), 1);
        assert!(matches!(effs[0], Effect::HidePlayer { entity_id: 100 }));
        assert_eq!(src.state, State::SendWarmup);
        let _ = hide_now;

        // Warmup timer → teleport
        let effs = src.warmup_timer_expired([10.0, 20.0, 30.0], "Castle");
        assert_eq!(src.state, State::RemoteLoadWait);
        assert_eq!(effs.len(), 1);
        match &effs[0] {
            Effect::TeleportPlayer { entity_id, position, world_name, destination_region_id } => {
                assert_eq!(*entity_id, 100);
                assert_eq!(*position, [10.0, 20.0, 30.0]);
                assert_eq!(world_name, "Castle");
                assert_eq!(*destination_region_id, 2);
            }
            _ => panic!("expected TeleportPlayer"),
        }
    }

    #[test]
    fn full_cycle_destination_side() {
        let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
        dst.remote_wait(1);
        dst.remote_send();
        assert_eq!(dst.state, State::RecvWarmup);
        dst.remote_count_update(2);
        dst.remote_transport();
        assert_eq!(dst.state, State::RemoteLoadWait);
        // 1 of 2 loaded → not yet ready.
        assert!(!dst.player_loaded(100));
        // 2 of 2 → ready.
        assert!(dst.player_loaded(101));
    }

    #[test]
    fn destination_full_cycle_to_idle() {
        let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
        dst.remote_wait(1);
        dst.remote_send();
        dst.remote_count_update(1);
        dst.remote_transport();

        assert!(dst.player_loaded(100));

        let now = Instant::now();
        let effs = dst.all_players_loaded(now);
        assert_eq!(dst.state, State::RemoteWarmup);
        assert_eq!(effs.len(), 1); // PlaySequence(TeleportIn) for first player

        let effs = dst.remote_warmup_timer_expired(now + REMOTE_WARMUP_DELAY);
        assert_eq!(dst.state, State::Cooldown);
        assert_eq!(effs.len(), 1);
        assert!(matches!(effs[0], Effect::ShowPlayer { entity_id: 100 }));

        let effs = dst.cooldown_timer_expired();
        assert_eq!(dst.state, State::Idle);
        assert_eq!(effs.len(), 2); // unlock + fire teleport_in
        assert!(matches!(effs[0], Effect::UnlockMovement { entity_id: 100 }));
        assert!(matches!(effs[1], Effect::FireTeleportIn { entity_id: 100, region_id: 2 }));
        assert!(dst.players_loaded.is_empty());
        assert!(dst.remote_region_id.is_none());
    }

    #[test]
    fn destination_with_no_loaded_players_skips_sequence() {
        let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
        dst.remote_wait(1);
        dst.remote_send();
        dst.remote_count_update(0);
        dst.remote_transport();

        let effs = dst.all_players_loaded(Instant::now());
        assert!(effs.is_empty());
        assert_eq!(dst.state, State::RemoteWarmup);
    }
}
