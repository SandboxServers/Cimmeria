//! ConverseBasicHumanoid setup: the game-type table, the seeded deal, the
//! accord threshold and the `fullgamestate` builder (spec sections 3, 4, 5
//! and 7).

use std::collections::HashMap;

use rand::RngExt;

use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::ConverseHumanoidGame;

/// One row of the game-type table (spec section 4).
pub(super) struct GameTypeRow {
    pub(super) name: &'static str,
    /// Cards per side for the whole deal: `contest_total * container_total`.
    pub(super) card_total: usize,
    pub(super) container_total: usize,
    pub(super) contest_total: usize,
    /// The deck suits, each dealt twice per side pool.
    pub(super) suits: &'static [u8],
}

/// The game types in progression order. Only logic3 and mediation5 are
/// reachable (`next_game_type`); courtesy7 and humor9 stay as table data.
pub(super) static GAME_TYPES: [GameTypeRow; 4] = [
    GameTypeRow {
        name: "logic3",
        card_total: 3,
        container_total: 1,
        contest_total: 3,
        suits: &[0, 4, 8],
    },
    GameTypeRow {
        name: "mediation5",
        card_total: 4,
        container_total: 2,
        contest_total: 2,
        suits: &[0, 3, 4, 5, 8],
    },
    GameTypeRow {
        name: "courtesy7",
        card_total: 6,
        container_total: 2,
        contest_total: 3,
        suits: &[0, 2, 3, 4, 5, 7, 8],
    },
    GameTypeRow {
        name: "humor9",
        card_total: 9,
        container_total: 3,
        contest_total: 3,
        suits: &[0, 1, 2, 3, 4, 5, 6, 7, 8],
    },
];

/// Index of logic3, the first deal's game type.
pub(super) const FIRST_GAME_TYPE: usize = 0;
/// Index of mediation5, which every later deal uses.
const MEDIATION5: usize = 1;

/// The player's victory options at beginner tier: the client name and the
/// point cost, in the order the client lists them (spec section 7).
pub(super) const PLAYER_VICTORIES: [(&str, f64); 2] =
    [("focusdamage1", 3.0), ("focusdamage2", 4.0)];

/// Accord bonus per Trump bit of `abilityBitfield` (spec section 7): bit 15
/// is 778 Mediator (+5), 16 is 779 Negotiator (+10), 17 is 792 Diplomat (+15)
/// and 18 is 793 Ambassador (+25).
const TRUMP_BONUSES: [(u32, f64); 4] = [
    (1 << 15, 5.0),
    (1 << 16, 10.0),
    (1 << 17, 15.0),
    (1 << 18, 25.0),
];

/// The next game type after a deal: logic3 steps to mediation5, and
/// mediation5 stays (spec section 4).
pub(super) fn next_game_type(current: usize) -> usize {
    if current == FIRST_GAME_TYPE {
        MEDIATION5
    } else {
        current
    }
}

/// The accord bonus, in percent points, that the Trump bits of `mask` add.
pub(super) fn trump_bonus(mask: u32) -> f64 {
    TRUMP_BONUSES
        .iter()
        .filter(|(bit, _)| mask & *bit != 0)
        .map(|(_, bonus)| bonus)
        .sum()
}

/// A list of numbers, as the client reads an array.
fn numbers(values: &[f64]) -> SfsValue {
    SfsValue::list(values.iter().map(|&v| SfsValue::Number(v)))
}

impl ConverseHumanoidGame {
    /// Deal a hand to each side for the current game type. Each side's
    /// containers are reset and the NPC's hand is drawn first, from the same
    /// seeded RNG as the player's (spec section 3).
    pub(super) fn deal(&mut self) {
        let row: &'static GameTypeRow = &GAME_TYPES[self.game_type];
        let (npc_accord, player_accord) = self.get_accords();
        let mut deck: Vec<f64> = row
            .suits
            .iter()
            .flat_map(|&s| [f64::from(s), f64::from(s)])
            .collect();
        let accords: Vec<f64> = row
            .suits
            .iter()
            .flat_map(|&s| [f64::from(s) + 0.1, f64::from(s) + 0.2])
            .collect();
        self.npc_hand = self.draw_hand(&mut deck, &accords, npc_accord, row.card_total);
        self.player_hand = self.draw_hand(&mut deck, &accords, player_accord, row.card_total);
        self.npc_empty = vec![true; row.container_total];
        self.player_empty = vec![true; row.container_total];
        self.npc_contest_plays.clear();
        self.player_contest_plays.clear();
        self.npc_total_plays = 0;
        self.player_total_plays = 0;
    }

    /// Whether the NPC and the player each hold an accord for their next
    /// deal. This is a threshold, not a roll: `pct / 100 * card_total >= 1`
    /// (spec section 7).
    pub(super) fn get_accords(&self) -> (bool, bool) {
        let card_total = GAME_TYPES[self.game_type].card_total as f64;
        let level_gap = f64::from(self.player_level) - f64::from(self.tech_competency);
        let shared = f64::from(self.contests_without_accord) + level_gap;
        let npc_pct = 5.0 + shared;
        let player_pct =
            5.0 + trump_bonus(self.abilities_mask) + f64::from(self.intelligence) + shared;
        let npc = (npc_pct / 100.0).max(0.01) * card_total >= 1.0;
        let player = (player_pct / 100.0) * card_total >= 1.0;
        (npc, player)
    }

    /// One side's hand: `card_total` cards taken from `deck` without
    /// replacement, plus an accord card from `accords` when `has_accord`
    /// (taken without removal, then inserted at a random position). The hand
    /// is sorted numerically, as the original's AS2 sort leaves it.
    fn draw_hand(
        &mut self,
        deck: &mut Vec<f64>,
        accords: &[f64],
        has_accord: bool,
        card_total: usize,
    ) -> Vec<f64> {
        let mut count = card_total;
        let accord = if has_accord {
            count -= 1;
            Some(accords[self.rng.random_range(0..accords.len())])
        } else {
            None
        };
        let mut hand = Vec::with_capacity(card_total);
        for _ in 0..count {
            let index = self.rng.random_range(0..deck.len());
            hand.push(deck.remove(index));
        }
        if let Some(card) = accord {
            let index = self.rng.random_range(0..hand.len());
            hand.insert(index, card);
        }
        hand.sort_by(f64::total_cmp);
        hand
    }

    /// The `fullgamestate` message: the client renders it at join and after
    /// each deal. `suitTypes` is never set in the original, so it is omitted.
    pub(super) fn full_game_state(&self) -> HashMap<String, SfsValue> {
        let row = &GAME_TYPES[self.game_type];
        sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "numResets" => SfsValue::Number(f64::from(self.num_resets)),
            "timeRemaining" => SfsValue::Number(self.time_remaining - 0.25),
            "timerState" => SfsValue::Number(f64::from(self.timer_state)),
            "isSpectator" => SfsValue::Bool(false),
            "npcType" => SfsValue::String("humanoid".to_string()),
            "npcScore" => SfsValue::Number(self.npc_score),
            "npcCardsArray" => numbers(&self.npc_hand),
            "npcTotalPlays" => SfsValue::Number(f64::from(self.npc_total_plays)),
            "npcContestPlays" => numbers(&self.npc_contest_plays),
            "playerCardsArray" => numbers(&self.player_hand),
            "playerScore" => SfsValue::Number(self.player_score),
            "playerTotalPlays" => SfsValue::Number(f64::from(self.player_total_plays)),
            "playerContestPlays" => numbers(&self.player_contest_plays),
            "cardTotal" => SfsValue::Number(row.card_total as f64),
            "containerTotal" => SfsValue::Number(row.container_total as f64),
            "playerVictoryConditions" => SfsValue::list(
                PLAYER_VICTORIES
                    .iter()
                    .map(|(name, _)| SfsValue::String(name.to_string())),
            ),
            "gameType" => SfsValue::String(row.name.to_string()),
        }
    }
}
