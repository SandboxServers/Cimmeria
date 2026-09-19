//! Destination-side FSM transitions:
//! `Idle → RecvWait → RecvWarmup → RemoteLoadWait → RemoteWarmup → Cooldown → Idle`.
//!
//! Split out of [`super`] so the two halves of the state graph read
//! independently; the source half lives in [`super::source`].

use std::time::Instant;

use super::{Effect, RegionEvent, RingTransporter, State, COOLDOWN_DELAY, REMOTE_WARMUP_DELAY};

impl RingTransporter {
    /// `remoteWait()` from Python — driven by the source transporter when a
    /// destination is selected. Source ring → SendWait, destination → RecvWait.
    pub fn remote_wait(&mut self, source_region_id: i32, now: Instant) {
        debug_assert_eq!(self.state, State::Idle);
        self.state = State::RecvWait;
        self.remote_region_id = Some(source_region_id);
        self.arm_stall(now);
    }

    /// `remoteSend()` from Python: destination's `RecvWait → RecvWarmup`.
    /// Currently a state-only transition because we don't fire any wire
    /// effects on this transition (Python's `__beginTransport` runs on the
    /// source; the destination just notes it's now warming up).
    pub fn remote_send(&mut self, now: Instant) {
        debug_assert_eq!(self.state, State::RecvWait);
        self.state = State::RecvWarmup;
        self.arm_stall(now);
    }

    /// `remoteCountUpdate()` — destination receives the players the source is
    /// teleporting.
    ///
    /// Takes ids rather than the Python's bare count so a passenger that
    /// disappears mid-flight can be dropped from the expectation
    /// ([`RingTransporter::forget_participant`]) instead of decrementing an
    /// opaque counter, and so a `RemoteLoadWait` abort can release the
    /// in-flight-but-not-yet-loaded passengers.
    pub fn remote_expect(&mut self, players: Vec<u32>) {
        self.expected_players = players;
    }

    /// `remoteTransport()` — destination's `RecvWarmup → RemoteLoadWait`.
    /// Pure state transition; effects flow from the source's
    /// `warmup_timer_expired`.
    pub fn remote_transport(&mut self, now: Instant) {
        debug_assert_eq!(self.state, State::RecvWarmup);
        self.state = State::RemoteLoadWait;
        self.players_loaded.clear();
        self.arm_stall(now);
    }

    /// Is `entity_id` one of the passengers this ring is currently holding a
    /// slot for?
    ///
    /// The readiness gate counts `players_loaded` against
    /// [`RingTransporter::num_remote_players`] — a *length* comparison — so
    /// membership is the only thing that makes the two sides describe the
    /// same people. Callers that receive a load notification from outside the
    /// FSM (the cross-world `AdvanceRingDestination` hook) must check this
    /// before recording it.
    pub fn expects_player(&self, entity_id: u32) -> bool {
        self.expected_players.contains(&entity_id)
    }

    /// `playerLoaded()` — destination side. Returns true when the count of
    /// loaded players matches the source's expectation.
    ///
    /// An arrival this ring is not expecting is refused outright (PR #662
    /// review, finding 5). The readiness gate compares *lengths*, so pushing
    /// a stranger in satisfies it against a completely different passenger
    /// list: trip 1 sends A, A's load times out on
    /// `REMOTE_LOAD_WAIT_TIMEOUT`, trip 2 starts expecting `[B]`, and A's
    /// late `AdvanceRingDestination` then makes `len(players_loaded) == 1 ==
    /// num_remote_players()` — firing `all_players_loaded` for a trip whose
    /// only real passenger, B, is still loading. B is then left hidden and
    /// movement-locked with the ring already past the state that would
    /// release them. The caller routes a refused id to the late-arrival
    /// release in
    /// [`crate::cell::ring_transport::handle_remote_player_loaded`] instead.
    pub fn player_loaded(&mut self, entity_id: u32) -> bool {
        if !self.expects_player(entity_id) {
            return false;
        }
        if !self.players_loaded.contains(&entity_id) {
            self.players_loaded.push(entity_id);
        }
        self.players_loaded.len() as u32 == self.num_remote_players()
    }

    /// Drop `entity_id` from **both** destination-side sets.
    ///
    /// Removing it from only `expected_players` would create a fresh stall:
    /// the readiness gate is `players_loaded.len() == num_remote_players()`,
    /// so a passenger that had already loaded and then vanished would leave
    /// loaded=1 against expected=0 and the equality would never hold again.
    ///
    /// Returns `true` if the entity was in either set.
    pub(super) fn forget_participant(&mut self, entity_id: u32) -> bool {
        let before = self.expected_players.len() + self.players_loaded.len();
        self.expected_players.retain(|&e| e != entity_id);
        self.players_loaded.retain(|&e| e != entity_id);
        before != self.expected_players.len() + self.players_loaded.len()
    }

    /// `__allPlayersLoaded()` — destination's `RemoteLoadWait → RemoteWarmup`.
    /// Returns the Teleport_In sequence effect (only the first player triggers
    /// the matinee, see Python comment), and arms the remote-warmup timer.
    /// The caller is responsible for short-circuiting to
    /// [`RingTransporter::remote_warmup_timer_expired`] when no players
    /// loaded — Python does the same.
    pub fn all_players_loaded(&mut self, now: Instant) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::RemoteLoadWait);
        self.state = State::RemoteWarmup;
        // `RemoteWarmup` has its own deadline; drop the RemoteLoadWait stall
        // bound so it can't fire against the rest of this healthy trip.
        self.arm_stall(now);

        if self.players_loaded.is_empty() {
            // Match Python's branch: no sequence, no timer arm; caller will
            // call `remote_warmup_timer_expired` immediately.
            return Vec::new();
        }

        let mut effects = Vec::new();
        // Python: only the first player triggers the kismet matinee.
        // NOTE: `first()` here, not an index — a passenger that dropped
        // mid-load may have been the original first, and re-picking the
        // matinee owner is multi-player Matinee sync (out of scope, see the
        // FIXME in docs/gameplay/ring-transport-system.md).
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

        let effects: Vec<Effect> = self
            .players_loaded
            .iter()
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
        let region_id = self.region_id;

        let mut effects = Vec::with_capacity(self.players_loaded.len() * 2);
        for &eid in &self.players_loaded {
            effects.push(Effect::UnlockMovement { entity_id: eid });
            effects.push(Effect::FireTeleportIn {
                entity_id: eid,
                region_id,
            });
        }
        // Reset destination-side state in preparation for the next run. Full
        // reset rather than field-by-field so no timer (stall included)
        // survives into the next trip.
        self.reset_to_idle();
        effects
    }
}
