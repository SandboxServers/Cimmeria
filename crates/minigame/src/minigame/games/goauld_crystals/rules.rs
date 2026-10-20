//! GoauldCrystals clock and timeout (spec sections 5, 6 and 8).
//!
//! The countdown runs on the session tick: each tick takes 0.25 s off
//! `time_remaining`, or adds it back while the clock runs backwards, so
//! scripted runs are exact. A monotonic deadline, set when the clock starts or
//! resumes, is checked on the tick and before every command, so a click that
//! arrives after the wall-clock end is refused even if the tick is late. Once
//! `outcome` is set the game takes no further input. `processreset` is refused
//! (deliberate deviation from spec 5, for owner confirmation).

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::Instant;

use super::{loggable, GoauldCrystalsGame};
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Seconds the clock moves on each session tick (spec 8).
const TICK_SECS: f64 = 0.25;

/// Refused commands logged in full before the WARN is sampled.
const REFUSAL_LOG_FULL: u32 = 8;

/// After the full run, one refused command in this many is logged.
const REFUSAL_LOG_EVERY: u32 = 100;

/// Seconds the player has to make the first click before the game is lost.
/// Without it the clock never runs and no other deadline would end the game.
const PRE_CLICK_LIMIT: Duration = Duration::from_secs(60);

impl GoauldCrystalsGame {
    /// Routes one inbound command. After the end nothing is acted on; a clock
    /// past its deadline ends the game before the command is read.
    pub(super) fn dispatch(
        &mut self,
        cmd: &str,
        params: &HashMap<String, SfsValue>,
        now: Instant,
    ) -> Vec<GameOutput> {
        if self.outcome.is_some() {
            self.warn_refused(cmd, "game_over");
            return vec![];
        }
        if self.timed_out(now) || self.pre_click_expired(now) {
            return self.expire();
        }
        match cmd {
            "playfieldclick" => self.playfield_click(params),
            // Server-authoritative: a reset would refill the clock and the
            // ability bits. Deliberate deviation from spec 5, owner to confirm.
            "processreset" => {
                self.warn_refused(cmd, "reset_refused");
                vec![]
            }
            "activateabilitycheat" => self.activate_ability_cheat(params),
            // Item cheats are not ported (M8): they are unknown commands here.
            _ => {
                self.warn_refused(cmd, "unknown_command");
                vec![]
            }
        }
    }

    /// One session tick of the clock (spec 8). A running clock takes off
    /// 0.25 s and ends at zero; a pause or a reverse lasts `timer_ticks_left`
    /// ticks, then the clock runs again with one `timerupdate`.
    pub(super) fn clock_tick(&mut self, now: Instant) -> Vec<GameOutput> {
        if self.outcome.is_some() {
            return vec![];
        }
        if self.timed_out(now) || self.pre_click_expired(now) {
            return self.expire();
        }
        match self.timer_state {
            1 => {
                self.time_remaining -= TICK_SECS;
                if self.time_remaining <= 0.0 {
                    return self.expire();
                }
                vec![]
            }
            2 | 3 => {
                if self.timer_state == 3 {
                    self.time_remaining += TICK_SECS;
                }
                self.timer_ticks_left = self.timer_ticks_left.saturating_sub(1);
                if self.timer_ticks_left > 0 {
                    return vec![];
                }
                self.timer_state = 1;
                vec![GameOutput::Send(self.build_timer_update())]
            }
            _ => vec![],
        }
    }

    /// True when the clock runs and is spent: the countdown has reached zero
    /// or the monotonic deadline has passed. A paused or reversed clock has
    /// no deadline and is never timed out by the clock.
    pub(super) fn timed_out(&self, now: Instant) -> bool {
        self.timer_state == 1
            && (self.time_remaining <= 0.0 || self.deadline.is_some_and(|due| now >= due))
    }

    /// True before the first click once `PRE_CLICK_LIMIT` has passed since the
    /// game started. The clock is stopped until then, so `timed_out` cannot
    /// end such a game.
    pub(super) fn pre_click_expired(&self, now: Instant) -> bool {
        !self.clicked
            && self.outcome.is_none()
            && self
                .started_at
                .is_some_and(|at| now.saturating_duration_since(at) >= PRE_CLICK_LIMIT)
    }

    /// Keeps the deadline in step with the clock. A running clock without a
    /// deadline gets one `time_remaining` from `now`; any other state clears it.
    pub(super) fn sync_deadline(&mut self, now: Instant) {
        if self.timer_state != 1 {
            self.deadline = None;
        } else if self.deadline.is_none() {
            let left = Duration::from_secs_f64(self.time_remaining.max(0.0));
            self.deadline = Some(now + left);
        }
    }

    /// Spec 6 timeout: the clock stops, `failure` goes out and the game ends.
    fn expire(&mut self) -> Vec<GameOutput> {
        self.timer_state = 0;
        self.deadline = None;
        self.outcome = Some("failure");
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("failure".to_string()),
            }),
            GameOutput::Failure,
        ]
    }

    /// Warns about a refused command. Every warning names the player, so the
    /// line can be traced to a session. The first `REFUSAL_LOG_FULL` refusals
    /// are logged, then one in `REFUSAL_LOG_EVERY`, so a flood of refused
    /// frames cannot flood the log. Every row carries the running count.
    pub(super) fn warn_refused(&mut self, cmd: &str, reason: &'static str) {
        self.refusals = self.refusals.saturating_add(1);
        let count = self.refusals;
        if count > REFUSAL_LOG_FULL && !count.is_multiple_of(REFUSAL_LOG_EVERY) {
            return;
        }
        tracing::warn!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            cmd = %loggable(cmd),
            reason = reason,
            refusals = count,
            "GoauldCrystals: command refused"
        );
    }
}
