//! ConverseBasicHumanoid timers, the deal flow, the victory gate and the
//! result (spec sections 6, 7 and 8). The clock is the session's 250 ms tick:
//! each tick subtracts exactly 0.25 s, never wall time.

use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::setup::{self, GAME_TYPES, PLAYER_VICTORIES};
use super::{ConverseHumanoidGame, Outcome, HAND_TIMER, NPC_TIMER};

/// Seconds the countdown loses per tick.
const TICK_SECONDS: f64 = 0.25;
/// Seconds between NPC moves once one has been made (spec section 8).
const NPC_INTERVAL: f64 = 1.5;

impl ConverseHumanoidGame {
    /// Whether the deal is over: both sides have made every play. The NPC and
    /// hand timers are held from `newdeal` until `processreset`.
    pub(super) fn deal_over(&self) -> bool {
        let card_total = GAME_TYPES[self.game_type].card_total as u32;
        self.player_total_plays == card_total && self.npc_total_plays == card_total
    }

    /// One 250 ms tick (spec section 8). The countdown runs first, and at zero
    /// the game ends. Otherwise the NPC moves every 1.5 s and the hand ends
    /// every 6 s; both are held while the deal is over.
    pub(super) fn tick_timers(&mut self) -> Vec<GameOutput> {
        if self.outcome.is_some() {
            return Vec::new();
        }
        if self.timer_state == 1 {
            self.time_remaining -= TICK_SECONDS;
        }
        if self.time_remaining <= 0.0 && self.timer_state >= 1 {
            return self.check_for_victory();
        }
        if self.deal_over() {
            return Vec::new();
        }
        let mut out = Vec::new();
        if self.time_remaining <= self.npc_last_time - self.npc_timer {
            out.extend(self.npc_move());
            self.npc_last_time = self.time_remaining;
            self.npc_timer = NPC_INTERVAL;
        }
        if self.time_remaining <= self.hand_last_time - self.hand_timer {
            out.extend(self.end_hand());
            self.hand_last_time = self.time_remaining;
            self.hand_timer = HAND_TIMER;
        }
        out
    }

    /// `checkForVictory` (spec section 6). The NPC ahead is a defeat; a tie or
    /// a lead for the player is a victory.
    pub(super) fn check_for_victory(&mut self) -> Vec<GameOutput> {
        self.timer_state = 0;
        if self.npc_score > self.player_score {
            return self.fail_message();
        }
        self.outcome = Some(Outcome::Victory);
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("victory".to_string()),
            }),
            GameOutput::Victory,
        ]
    }

    /// `processreset` (spec section 6, deal end). Only after `newdeal`: the
    /// next game type is dealt, scores and time are kept, and the NPC and hand
    /// timers start again. With under a second left the original ends the game
    /// through `checkForVictory`.
    pub(super) fn process_reset(&mut self) -> Vec<GameOutput> {
        if !self.deal_over() {
            return self.reject("processreset", "deal_not_over");
        }
        if self.time_remaining.floor() <= 0.0 {
            return self.check_for_victory();
        }
        self.num_resets += 1;
        self.contests_without_accord += 1;
        self.game_type = setup::next_game_type(self.game_type);
        self.deal();
        self.restart_timers();
        vec![GameOutput::Send(self.full_game_state())]
    }

    /// Starts the NPC and hand timers from the reset, as at the join: the first
    /// NPC move of a deal is 3 s in, and a hand lasts 6 s.
    fn restart_timers(&mut self) {
        self.npc_last_time = self.time_remaining;
        self.npc_timer = NPC_TIMER;
        self.hand_last_time = self.time_remaining;
        self.hand_timer = HAND_TIMER;
    }

    /// `activatevictory` (spec section 7). The cost comes from the server's
    /// table, never the client's `pointCost`, and it is paid from the server's
    /// own score. Refused when the number is not a listed condition, the
    /// condition is used, or the score is below its cost.
    pub(super) fn activate_victory(&mut self, victory: f64) -> Vec<GameOutput> {
        let listed =
            victory.fract() == 0.0 && victory >= 1.0 && victory <= PLAYER_VICTORIES.len() as f64;
        if !listed {
            return self.reject("activatevictory", "victory_out_of_range");
        }
        let index = victory as usize - 1;
        if self.player_victory_used[index] {
            return self.reject("activatevictory", "victory_used");
        }
        let cost = PLAYER_VICTORIES[index].1;
        if self.player_score < cost {
            return self.reject("activatevictory", "victory_unaffordable");
        }
        self.player_score -= cost;
        self.player_victory_used[index] = true;
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("activatevictory".to_string()),
                "pScore" => SfsValue::Number(self.player_score),
                "nScore" => SfsValue::Number(self.npc_score),
                "user" => SfsValue::String("player".to_string()),
                "victoryNum" => SfsValue::Number((index + 1) as f64),
            }),
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("updatescore".to_string()),
                "npcScore" => SfsValue::Number(self.npc_score),
                "playerScore" => SfsValue::Number(self.player_score),
            }),
        ]
    }
}
