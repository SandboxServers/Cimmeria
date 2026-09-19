//! Process-wide registry of ring transporters (one entry per region_id).
//!
//! All worlds' transporters live in the same map because cross-world rings
//! need to find each other by region_id.
//!
//! The registry also owns the two things that are inherently *cross*-region:
//! the injectable clock every FSM deadline is measured against, and the
//! pair-abort / player-forget bookkeeping that has to touch both ends of a
//! trip at once.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use cimmeria_mercury::clock::{system_clock, Clock};

use super::super::regions::RingRegion;
use super::{AbortReason, DeadlineKind, Effect, RingTransporter, State};

pub struct RingTransporterManager {
    pub regions: HashMap<i32, RingTransporter>,
    /// Time source for every FSM deadline.
    ///
    /// Injectable so the timeout tests can drive 90 seconds of ring stall in
    /// microseconds instead of sleeping. Reuses `cimmeria_mercury::clock`
    /// rather than inventing a second clock trait — `Channel` already reads
    /// time through it for RTO/keepalive. Lives on the manager (not threaded
    /// through every `handle_*` signature) because the cell-side callers of
    /// those entry points are outside this module's ownership.
    clock: Arc<dyn Clock>,
    /// Entities destroyed via the synchronous `SpaceManager::destroy_entity`
    /// path, awaiting source-side ring cleanup on the next tick.
    ///
    /// See [`Self::note_player_gone`] for why this is queued rather than
    /// applied inline.
    pending_player_gone: Vec<u32>,
    /// Destination regions whose load-readiness condition may have become
    /// satisfiable because a passenger dropped out of the expectation.
    /// Drained by the tick, which holds the `ChainEngine` that
    /// `try_advance_after_load` needs.
    pending_load_recheck: Vec<i32>,
}

// `Arc<dyn Clock>` has no `Debug` bound, so the derive can't be used. The
// clock has no interesting state to print anyway.
impl std::fmt::Debug for RingTransporterManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RingTransporterManager")
            .field("regions", &self.regions)
            .field("pending_player_gone", &self.pending_player_gone)
            .field("pending_load_recheck", &self.pending_load_recheck)
            .finish_non_exhaustive()
    }
}

impl Default for RingTransporterManager {
    fn default() -> Self {
        Self {
            regions: HashMap::new(),
            clock: system_clock(),
            pending_player_gone: Vec::new(),
            pending_load_recheck: Vec::new(),
        }
    }
}

impl RingTransporterManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current time as every ring deadline measures it.
    pub fn now(&self) -> Instant {
        self.clock.now()
    }

    /// Swap the time source. Tests only — production never calls this, so
    /// the FSM always reads a real monotonic clock in the server.
    pub fn set_clock(&mut self, clock: Arc<dyn Clock>) {
        self.clock = clock;
    }

    /// Build one transporter per region in the supplied table. Idempotent —
    /// re-init clears any in-flight FSM state (use only at startup).
    pub fn load(&mut self, ring_regions: &HashMap<i32, RingRegion>) {
        // Startup-only by contract, but a hot reload against a live world
        // would strand every locked/hidden passenger with no FSM left to
        // release them. Cheap to surface; free when the contract holds.
        let busy: Vec<i32> = self
            .regions
            .iter()
            .filter(|(_, t)| t.state != State::Idle)
            .map(|(id, _)| *id)
            .collect();
        if !busy.is_empty() {
            tracing::warn!(
                busy_regions = ?busy,
                reason = "reload_while_busy",
                "ring load: reloading transporters while trips are in flight — \
                 those passengers keep BSF_MovementLock and stay hidden with no FSM to release them"
            );
        }
        self.regions.clear();
        self.pending_player_gone.clear();
        self.pending_load_recheck.clear();
        for (id, region) in ring_regions {
            self.regions
                .insert(*id, RingTransporter::from_region(region));
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
    ///
    /// Ordered, not `HashMap`-arbitrary: bounded-stall aborts come last, and
    /// ties break on `region_id`. This is the cross-region extension of the
    /// same rule [`RingTransporter::elapsed_deadline`] applies within one
    /// transporter — a real transition wins over an abort. It matters
    /// because a tick that lags past several deadlines at once can hold both
    /// a source's warmup and its peer's `RecvWarmup` stall, and handling the
    /// warmup is exactly what makes the peer's stall stale.
    ///
    /// The ordering is necessary but not sufficient on its own; the tick
    /// also revalidates each deadline against live state before applying it
    /// (see [`Self::current_deadline`]).
    pub fn ready_regions(&self, now: Instant) -> Vec<(i32, RawDeadline)> {
        let mut out: Vec<(i32, RawDeadline)> = self
            .regions
            .iter()
            .filter_map(|(id, r)| r.elapsed_deadline(now).map(|dk| (*id, RawDeadline(dk))))
            .collect();
        out.sort_by_key(|(id, d)| (d.is_stall(), *id));
        out
    }

    /// Re-read `region_id`'s currently-elapsed deadline at `now`.
    ///
    /// The tick snapshots [`Self::ready_regions`] once and then applies the
    /// entries one at a time, but applying one entry can change another
    /// region: the source's warmup drives its peer `RecvWarmup →
    /// RemoteLoadWait` and re-arms the peer's stall deadline. The peer's
    /// snapshot entry is stale from that moment on, and applying it would
    /// abort a trip that has just become healthy. Comparing against this
    /// makes the snapshot advisory rather than authoritative.
    pub(crate) fn current_deadline(&self, region_id: i32, now: Instant) -> Option<RawDeadline> {
        self.regions
            .get(&region_id)
            .and_then(|r| r.elapsed_deadline(now))
            .map(RawDeadline)
    }

    /// Tear down the trip on `region_id` **and** its peer, returning the
    /// release effects for every participant on both ends.
    ///
    /// The peer is only reset when its `remote_region_id` points back at
    /// `region_id`: a peer pointing elsewhere belongs to a different trip and
    /// resetting it would abort a healthy transport. A mismatch is a genuine
    /// cross-link desync and is logged.
    ///
    /// `skip` is an entity already removed from the space (its teardown is
    /// what triggered the abort); no effects are addressed to it.
    pub fn abort_pair(
        &mut self,
        region_id: i32,
        reason: AbortReason,
        skip: Option<u32>,
    ) -> Vec<Effect> {
        // Read the peer link and the reporting fields out first: `regions` is
        // one map, so source and peer cannot both be borrowed mutably.
        let Some((peer_id, state, participants)) = self
            .regions
            .get(&region_id)
            .map(|t| (t.remote_region_id, t.state, t.trip_participant_count()))
        else {
            return Vec::new();
        };
        if state == State::Idle {
            // Nothing to tear down — but clear the timers anyway. A deadline
            // left armed on an `Idle` ring would re-enter this function every
            // tick forever (`ready_regions` keeps reporting it), burning the
            // per-region transition budget and logging on each pass.
            if let Some(t) = self.regions.get_mut(&region_id) {
                t.reset_to_idle();
            }
            return Vec::new();
        }

        let mut effects = Vec::new();
        if let Some(t) = self.regions.get_mut(&region_id) {
            effects.extend(t.abort_to_idle(skip));
        }

        // Negative log per docs/architecture/negative-logging-convention.md:
        // player-visible (a stuck ring rejects every future destination) but
        // recoverable, so `warn`.
        tracing::warn!(
            region_id,
            peer_region_id = peer_id,
            state = ?state,
            reason = reason.as_str(),
            participants,
            released = effects.len() / 2,
            "ring transport aborted: trip torn down and both ends returned to Idle — \
             players released from movement lock and re-shown; the destination is selectable again"
        );

        if let Some(peer_id) = peer_id {
            let peer_points_back = self
                .regions
                .get(&peer_id)
                .map(|p| (p.remote_region_id, p.state));
            match peer_points_back {
                Some((Some(back), peer_state))
                    if back == region_id && peer_state != State::Idle =>
                {
                    if let Some(p) = self.regions.get_mut(&peer_id) {
                        effects.extend(p.abort_to_idle(skip));
                    }
                    tracing::warn!(
                        region_id = peer_id,
                        peer_region_id = region_id,
                        state = ?peer_state,
                        reason = AbortReason::PeerAborted.as_str(),
                        "ring transport aborted: peer end torn down alongside its partner"
                    );
                }
                Some((back, peer_state)) if peer_state != State::Idle => {
                    tracing::warn!(
                        region_id,
                        peer_region_id = peer_id,
                        peer_back_pointer = back,
                        peer_state = ?peer_state,
                        reason = "peer_backpointer_mismatch",
                        "ring abort: peer is busy but does not point back at this ring — \
                         leaving it alone; it belongs to another trip and its own stall timeout will bound it"
                    );
                }
                _ => {}
            }
        }

        effects
    }

    /// Record that `entity_id`'s cell entity has been destroyed by the
    /// synchronous [`crate::cell::space_manager::SpaceManager::destroy_entity`]
    /// path, to be reconciled on the next ring tick.
    ///
    /// **Queued, not applied.** Two reasons:
    ///
    /// 1. `destroy_entity` is sync with no `CellToBaseMsg` sender and ~30
    ///    call sites, so it cannot dispatch the release effects. Flipping FSM
    ///    state here while the wire effects wait for the tick would let a
    ///    player re-trigger the pad inside the gap and have the *previous*
    ///    trip's `ShowPlayer`/`UnlockMovement` land on top of the new trip.
    ///    Queuing the id instead keeps the state flip and the effects in the
    ///    same tick, atomically.
    /// 2. The real player-disconnect path
    ///    ([`super::super::runtime::forget_player`], called from
    ///    `SpaceManager::disconnect_entity`) is async and does the full
    ///    cleanup synchronously; by the time it reaches `destroy_entity` the
    ///    entity is already out of every set, so this queue entry is a no-op.
    ///    It exists to cover the *other* destroy sites (GM despawn, gate
    ///    travel, respawn, content transport).
    pub fn note_player_gone(&mut self, entity_id: u32) {
        self.pending_player_gone.push(entity_id);
    }

    /// Drain the queue filled by [`Self::note_player_gone`].
    pub(crate) fn take_pending_player_gone(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.pending_player_gone)
    }

    /// Queue a destination region for a load-readiness re-check on the next
    /// tick. Used when a passenger leaves the expectation while co-travellers
    /// remain.
    pub(crate) fn note_load_recheck(&mut self, region_id: i32) {
        if !self.pending_load_recheck.contains(&region_id) {
            self.pending_load_recheck.push(region_id);
        }
    }

    /// Drain the queue filled by [`Self::note_load_recheck`].
    pub(crate) fn take_pending_load_recheck(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.pending_load_recheck)
    }

    /// Remove `entity_id` from the **source-side** sets of every transporter
    /// and report the regions whose in-flight trip that emptied.
    ///
    /// Deliberately source-side only (`players`, `send_players`). The
    /// destination's `expected_players` must NOT be touched from the
    /// `destroy_entity` path, because a legitimate cross-world ring handoff
    /// destroys the cell entity itself (`Effect::TeleportCrossWorld` →
    /// `space_mgr.destroy_entity`) while the destination is correctly parked
    /// in `RemoteLoadWait` counting that very passenger. Dropping them there
    /// would fast-path the destination to `Idle` and strand the traveller on
    /// arrival. A genuine disconnect reaches the destination sets through
    /// [`Self::forget_participant_everywhere`] instead.
    pub(crate) fn forget_source_side(&mut self, entity_id: u32) -> Vec<i32> {
        let mut emptied = Vec::new();
        for (id, t) in self.regions.iter_mut() {
            let was_on_pad = t.players.remove(&entity_id).is_some();
            let before = (t.send_players.len(), t.reserved_by.len());
            t.send_players.retain(|&e| e != entity_id);
            t.reserved_by.retain(|&e| e != entity_id);
            let was_tracked = before != (t.send_players.len(), t.reserved_by.len());
            if (was_tracked || was_on_pad)
                && t.send_players.is_empty()
                && t.reserved_by.is_empty()
                && t.players.is_empty()
                && matches!(t.state, State::SendWait | State::SendWarmup)
            {
                emptied.push(*id);
            }
        }
        emptied
    }

    /// Remove `entity_id` from every set on every transporter, source and
    /// destination, and report which regions need follow-up.
    ///
    /// Returns `(emptied, rechecks)`: `emptied` are regions whose trip has no
    /// participants left and must be aborted; `rechecks` are destination
    /// regions still holding other passengers, whose readiness condition may
    /// now be satisfiable and should be re-run through
    /// `try_advance_after_load`.
    pub(crate) fn forget_participant_everywhere(&mut self, entity_id: u32) -> (Vec<i32>, Vec<i32>) {
        let mut emptied = self.forget_source_side(entity_id);
        let mut rechecks = Vec::new();
        for (id, t) in self.regions.iter_mut() {
            if !t.forget_participant(entity_id) {
                continue;
            }
            if t.expected_players.is_empty() && t.players_loaded.is_empty() {
                if !emptied.contains(id) {
                    emptied.push(*id);
                }
            } else {
                rechecks.push(*id);
            }
        }
        (emptied, rechecks)
    }
}

impl RingTransporter {
    /// Participant count for the abort log.
    fn trip_participant_count(&self) -> usize {
        self.reserved_by.len()
            + self.send_players.len()
            + self.expected_players.len()
            + self.players_loaded.len()
    }
}

/// Opaque deadline-kind handle returned by `ready_regions`. Wrapping it
/// keeps `DeadlineKind` private while still letting the tick code dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawDeadline(DeadlineKind);

impl RawDeadline {
    pub(crate) fn is_hide(self) -> bool {
        self.0 == DeadlineKind::Hide
    }
    pub(crate) fn is_warmup(self) -> bool {
        self.0 == DeadlineKind::Warmup
    }
    pub(crate) fn is_remote_warmup(self) -> bool {
        self.0 == DeadlineKind::RemoteWarmup
    }
    pub(crate) fn is_cooldown(self) -> bool {
        self.0 == DeadlineKind::Cooldown
    }
    pub(crate) fn is_stall(self) -> bool {
        self.0 == DeadlineKind::Stall
    }
}
