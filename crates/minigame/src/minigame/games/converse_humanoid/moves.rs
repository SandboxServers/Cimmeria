//! ConverseBasicHumanoid plays, the NPC opponent and hand scoring (spec
//! sections 5, 6 and 8). MG-CH3 wires these into `message()` and `tick()`;
//! nothing here reads the clock.

use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::setup::GAME_TYPES;
use super::{ConverseHumanoidGame, Outcome};

/// Seconds until the NPC answers a player's play into a container it has not
/// filled (spec section 8: `playerMove` sets `npcTimer = 0.5`).
const FOCUS_NPC_TIMER: f64 = 0.5;

/// The dominance of player card `player` over NPC card `npc` (spec section 6):
/// 8 is a tie (same suit), 0 to 3 the NPC wins, 4 to 7 the player wins.
fn dominance(player: f64, npc: f64) -> i64 {
    (player.floor() as i64 - npc.floor() as i64 + 8).rem_euclid(9)
}

/// The NPC's weight for `card` against one player card (spec section 8):
/// +0.5 for a tie, +1 when the NPC wins, -1 when the player wins.
fn eval_weight(card: f64, player_card: f64) -> f64 {
    match dominance(player_card, card) {
        8 => 0.5,
        d if d < 4 => 1.0,
        _ => -1.0,
    }
}

/// The index of the largest score. `maxValue` in the original is strict, so
/// a tie goes to the lowest index.
fn argmax(scores: impl Iterator<Item = f64>) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (index, score) in scores.enumerate() {
        match best {
            Some((_, top)) if score <= top => {}
            _ => best = Some((index, score)),
        }
    }
    best.map(|(index, _)| index)
}

/// A container number from the client, if it names one of `total` containers.
fn container_index(value: f64, total: usize) -> Option<usize> {
    if value.fract() == 0.0 && value >= 0.0 && (value as usize) < total {
        Some(value as usize)
    } else {
        None
    }
}

impl ConverseHumanoidGame {
    /// Whether the game still takes plays: the timer runs and no one has won.
    fn is_running(&self) -> bool {
        self.timer_state == 1 && self.outcome.is_none()
    }

    /// The `updatecards` message after a play. `num` is the card's index in
    /// the hand that played, and `container` the container it went into.
    fn update_cards(&self, user: &str, num: f64, suit: f64, container: usize) -> GameOutput {
        GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("updatecards".to_string()),
            "user" => SfsValue::String(user.to_string()),
            "num" => SfsValue::Number(num),
            "suit" => SfsValue::Number(suit),
            "container" => SfsValue::Number(container as f64),
            "playerTotalPlays" => SfsValue::Number(f64::from(self.player_total_plays)),
            "npcTotalPlays" => SfsValue::Number(f64::from(self.npc_total_plays)),
        })
    }

    /// `playermove` (spec section 5). The card is found by its `suit` value,
    /// wherever it sits in the hand, and removed. `num` is the client's index,
    /// which goes back unchanged in `updatecards`. An absent `container` takes
    /// the first empty one; a present one must be in range and empty.
    pub(super) fn player_move(
        &mut self,
        num: f64,
        suit: f64,
        container: Option<f64>,
    ) -> Vec<GameOutput> {
        if !self.is_running() {
            return self.reject("playermove", "not_running");
        }
        if !num.is_finite() {
            return self.reject("playermove", "num_not_finite");
        }
        let row = &GAME_TYPES[self.game_type];
        // A hand fills each container once, so it holds `container_total` plays.
        if self.player_contest_plays.len() >= row.container_total {
            return self.reject("playermove", "contest_full");
        }
        let Some(index) = self.player_hand.iter().position(|&card| card == suit) else {
            return self.reject("playermove", "suit_not_in_hand");
        };
        let c = match container {
            Some(value) => match container_index(value, row.container_total) {
                Some(c) if self.player_empty[c] => c,
                _ => return self.reject("playermove", "container_unavailable"),
            },
            None => match self.player_empty.iter().position(|&empty| empty) {
                Some(c) => c,
                None => return self.reject("playermove", "no_free_container"),
            },
        };

        self.player_hand.remove(index);
        self.player_contest_plays.push(suit);
        self.player_total_plays += 1;
        self.player_empty[c] = false;
        // The NPC answers at once when it has not filled this container.
        if self.npc_empty[c] {
            self.npc_focus = Some((c, suit));
            self.npc_timer = FOCUS_NPC_TIMER;
        }
        vec![self.update_cards("player", num, suit, c)]
    }

    /// `npcMove` (spec section 8). A pending focus is answered with the NPC
    /// card that best beats the focus suit. Otherwise the card whose summed
    /// weight over the player's hand is highest goes into the first empty
    /// container. Nothing happens when the NPC contest is full.
    pub(super) fn npc_move(&mut self) -> Vec<GameOutput> {
        let row = &GAME_TYPES[self.game_type];
        if self.npc_contest_plays.len() >= row.container_total || self.npc_hand.is_empty() {
            return Vec::new();
        }
        let (index, slot) = match self.npc_focus.take() {
            Some((c, suit)) => {
                let index = argmax(self.npc_hand.iter().map(|&card| eval_weight(card, suit)));
                (index, Some(c))
            }
            None => {
                let index = argmax(self.npc_hand.iter().map(|&card| {
                    self.player_hand
                        .iter()
                        .map(|&player_card| eval_weight(card, player_card))
                        .sum::<f64>()
                }));
                (index, self.npc_empty.iter().position(|&empty| empty))
            }
        };
        let Some(index) = index else {
            return Vec::new();
        };

        let card = self.npc_hand.remove(index);
        self.npc_contest_plays.push(card);
        self.npc_total_plays += 1;
        let shown = slot.unwrap_or(0);
        if let Some(c) = slot {
            self.npc_empty[c] = false;
        }
        vec![self.update_cards("npc", index as f64, card, shown)]
    }

    /// `processScore` (spec section 6). The two contests are paired by play
    /// order over the first `container_total` plays. A pair with one side
    /// missing goes to the player, as the original's fall-through does (spec
    /// section 11, question 3). A tie or two empty slots score nothing. A side
    /// that won every container adds one more point. Returns `updatescore`.
    pub(super) fn process_score(&mut self) -> Vec<GameOutput> {
        let total = GAME_TYPES[self.game_type].container_total;
        let mut player_wins = 0;
        let mut npc_wins = 0;
        for i in 0..total {
            match (
                self.player_contest_plays.get(i),
                self.npc_contest_plays.get(i),
            ) {
                (None, None) => {}
                (Some(_), None) | (None, Some(_)) => player_wins += 1,
                (Some(&player), Some(&npc)) => match dominance(player, npc) {
                    8 => {}
                    d if d < 4 => npc_wins += 1,
                    _ => player_wins += 1,
                },
            }
        }
        self.player_score += player_wins as f64;
        self.npc_score += npc_wins as f64;
        if player_wins == total {
            self.player_score += 1.0;
        }
        if npc_wins == total {
            self.npc_score += 1.0;
        }
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("updatescore".to_string()),
            "npcScore" => SfsValue::Number(self.npc_score),
            "playerScore" => SfsValue::Number(self.player_score),
        })]
    }

    /// `clearContainers`: every container is empty again, both contests are
    /// cleared and any pending NPC focus is dropped.
    pub(super) fn clear_containers(&mut self) {
        self.player_empty.fill(true);
        self.npc_empty.fill(true);
        self.player_contest_plays.clear();
        self.npc_contest_plays.clear();
        self.npc_focus = None;
    }

    /// `failMessage` (spec section 6): the game is lost. Sends `failure` and
    /// completes with a defeat.
    pub(super) fn fail_message(&mut self) -> Vec<GameOutput> {
        self.timer_state = 0;
        self.outcome = Some(Outcome::Defeat);
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("failure".to_string()),
            }),
            GameOutput::Failure,
        ]
    }

    /// `newhand` from the client (spec section 5). It ends the hand now, but
    /// only once the NPC has answered every container the player filled this
    /// hand, or the NPC can no longer answer (its contest is full or its hand
    /// is empty). Otherwise it is refused, so a client cannot end a hand
    /// before the NPC has moved and gain a point it did not earn. The hand
    /// timer calls `end_hand` directly.
    pub(super) fn new_hand(&mut self) -> Vec<GameOutput> {
        if !self.is_running() {
            return self.reject("newhand", "not_running");
        }
        // The hand already ended with `newdeal`; a repeat must not score again.
        if self.awaiting_reset {
            return self.reject("newhand", "awaiting_reset");
        }
        let row = &GAME_TYPES[self.game_type];
        let npc_can_answer =
            self.npc_contest_plays.len() < row.container_total && !self.npc_hand.is_empty();
        let answered = (0..row.container_total).all(|c| self.player_empty[c] || !self.npc_empty[c]);
        if npc_can_answer && !answered {
            return self.reject("newhand", "npc_not_answered");
        }
        self.end_hand()
    }

    /// The hand ends, by `newhand` or the hand timer (spec section 6). In
    /// order: the deal is over (`newdeal`); the player has made no play this
    /// deal, or the NPC has no cards left or played this hand while the
    /// player did not (the game is lost); otherwise the hand goes on
    /// (`emptyhand`). Scores are settled before `newdeal` and `emptyhand`.
    pub(super) fn end_hand(&mut self) -> Vec<GameOutput> {
        let card_total = GAME_TYPES[self.game_type].card_total as u32;
        let deal_over = self.player_total_plays == card_total && self.npc_total_plays == card_total;
        if !deal_over {
            if self.player_total_plays == 0 {
                return self.fail_message();
            }
            let player_idle = self.player_contest_plays.is_empty();
            let npc_out = self.npc_total_plays == card_total;
            if player_idle && (npc_out || !self.npc_contest_plays.is_empty()) {
                return self.fail_message();
            }
        }
        let mut out = self.process_score();
        self.clear_containers();
        // Held until processreset when the deal ends: the timers and newhand wait.
        self.awaiting_reset = deal_over;
        let cmd = if deal_over { "newdeal" } else { "emptyhand" };
        out.push(GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String(cmd.to_string()),
        }));
        out
    }
}
