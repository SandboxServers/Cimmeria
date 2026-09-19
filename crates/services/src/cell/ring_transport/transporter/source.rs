//! Source-side FSM transitions: `Idle → SendWait → SendWarmup → Idle`.
//!
//! Split out of [`super`] so the two halves of the state graph read
//! independently; the destination half lives in [`super::destination`].

use std::time::Instant;

use super::{Effect, RegionEvent, RingTransporter, State, HIDE_DELAY, WARMUP_DELAY};

impl RingTransporter {
    /// Move to `SendWait` and remember the remote region ID. Caller must
    /// drive the remote transporter into `RecvWait` separately.
    ///
    /// Arms the `SendWait` stall deadline: from here the FSM is waiting on a
    /// player to physically walk onto the pad, which may never happen.
    ///
    /// `entity_id` is the player who picked the destination. Recording it in
    /// [`RingTransporter::reserved_by`] is what lets a disconnect during
    /// `SendWait` free the pad immediately instead of waiting out
    /// [`super::SEND_WAIT_TIMEOUT`] — before H02 a `SendWait` trip had no
    /// participant recorded on either ring.
    pub fn enter_send_wait(&mut self, destination_id: i32, entity_id: u32, now: Instant) {
        self.state = State::SendWait;
        self.remote_region_id = Some(destination_id);
        if !self.reserved_by.contains(&entity_id) {
            self.reserved_by.push(entity_id);
        }
        self.arm_stall(now);
    }

    /// `__startSending()` from Python: transition `SendWait → SendWarmup`,
    /// snapshot the current players, schedule the hide and warmup timers, and
    /// return the side-effect list. The caller must also poke the remote
    /// transporter into `RecvWarmup` via
    /// [`RingTransporter::remote_send`] after dispatching these effects.
    pub fn start_sending(&mut self, now: Instant) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWait);
        self.state = State::SendWarmup;
        // `SendWarmup` carries its own hide/warmup deadlines, so the stall
        // deadline must come back off — otherwise the 60s SendWait bound
        // stays armed and fires against a later, healthy trip.
        self.arm_stall(now);

        // Snapshot players before clearing — we need to keep teleporting them
        // even if more arrive during warmup, and we must not double-teleport
        // anyone who steps off the pad.
        self.send_players = self.players.keys().copied().collect();
        self.send_players.sort_unstable(); // deterministic order for "first player" sequence pick
        self.players.clear();
        // The reservation has been honoured; from here the trip is tracked
        // by `send_players`.
        self.reserved_by.clear();

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

    /// Hide-timer: source-side `setVisible(false)` for every snapshotted player.
    /// Mirrors `__hideTimerExpired`.
    pub fn hide_timer_expired(&mut self) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWarmup);
        self.timers.hide_at = None;
        self.send_players
            .iter()
            .map(|&eid| Effect::HidePlayer { entity_id: eid })
            .collect()
    }

    /// Warmup-timer: source-side teleport. Mirrors `__warmupTimerExpired` +
    /// `__doTransport`.
    ///
    /// After dispatching the teleport effects, the source ring's job is done
    /// for this trip — it goes straight back to `Idle` and clears its transient
    /// state so the next trigger doesn't get rejected as "source ring is busy".
    /// The Python original keeps the source in REMOTE_LOAD_WAIT with no path
    /// out; we tighten this because our Rust state machine rejects re-entry.
    /// The destination ring continues through `RemoteLoadWait → RemoteWarmup →
    /// Cooldown → Idle` independently.
    ///
    /// Caller must use [`RingTransporter::remote_expect`] +
    /// [`RingTransporter::remote_transport`] on the destination ring after
    /// this returns.
    ///
    // H01 integration point: `dst_position` is the destination ring row's own
    // coordinate, copied verbatim into both teleport effects below with no
    // navmesh check. When H01's shared `is_point_valid` + respawner-fallback
    // helper lands, it wraps `dst_position` HERE, before the match — that is
    // the only place a ring arrival position is chosen. Deferred out of H02
    // by coordinator override because H01 owns the helper.
    pub fn warmup_timer_expired(&mut self, dst_position: [f32; 3], dst_world: &str) -> Vec<Effect> {
        debug_assert_eq!(self.state, State::SendWarmup);
        self.timers.warmup_at = None;
        let dst_region = self.remote_region_id.unwrap_or(0);
        let send_world = self.world_name.clone();
        let players: Vec<u32> = std::mem::take(&mut self.send_players);
        let effects: Vec<Effect> = players
            .iter()
            .map(|&eid| {
                if dst_world == send_world {
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
            .collect();

        // Full reset, not a bare `state = Idle`: the source has handed the
        // trip off and must carry no timer (including the stall deadline)
        // into its next trip.
        self.reset_to_idle();
        effects
    }
}
