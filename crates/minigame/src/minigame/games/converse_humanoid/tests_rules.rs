//! Tests for the ConverseBasicHumanoid timers, deal flow, victory gate and
//! result (MG-CH3): the goldens, the held timers, the refusals and the ends.

use std::collections::HashMap;

use super::setup::GAME_TYPES;
use super::*;
use crate::minigame::game::GameOutput;
use crate::minigame::games::{ability_bits, validation};
use crate::minigame::repl::check_script;
use crate::minigame::session::MinigameSession;

fn started(seed: u32) -> ConverseHumanoidGame {
    let mut game =
        ConverseHumanoidGame::new(&MinigameSession::offline("ConverseBasicHumanoid", 1, seed));
    game.started();
    game
}

/// The command's fields, as numbers.
fn fields(pairs: &[(&str, f64)]) -> HashMap<String, SfsValue> {
    pairs
        .iter()
        .map(|&(key, value)| (key.to_string(), SfsValue::Number(value)))
        .collect()
}

fn tick_n(game: &mut ConverseHumanoidGame, n: u32) -> Vec<GameOutput> {
    (0..n).flat_map(|_| game.tick()).collect()
}

fn has_victory(out: &[GameOutput]) -> bool {
    out.iter().any(|o| matches!(o, GameOutput::Victory))
}

fn has_failure(out: &[GameOutput]) -> bool {
    out.iter().any(|o| matches!(o, GameOutput::Failure))
}

/// The `_cmd` of each message sent, in order.
fn sent_commands(out: &[GameOutput]) -> Vec<String> {
    out.iter()
        .filter_map(|o| match o {
            GameOutput::Send(vars) => vars.get("_cmd").and_then(|v| v.as_str()).map(String::from),
            _ => None,
        })
        .collect()
}

/// Every play made and `newdeal` sent: the deal is over and the timers are held.
fn finish_deal(game: &mut ConverseHumanoidGame) {
    let card_total = GAME_TYPES[game.game_type].card_total as u32;
    game.player_total_plays = card_total;
    game.npc_total_plays = card_total;
    game.awaiting_reset = true;
}

/// One tick as the session runs it: the player plays the first card of its
/// hand while its contest has room, then the tick. Returns both outputs.
fn play_and_tick(game: &mut ConverseHumanoidGame) -> Vec<GameOutput> {
    let mut out = Vec::new();
    let container_total = GAME_TYPES[game.game_type].container_total;
    if let Some(&suit) = game.player_hand.first() {
        if game.player_contest_plays.len() < container_total {
            let params = fields(&[("num", 0.0), ("suit", suit)]);
            out.extend(game.message("playermove", &params));
        }
    }
    out.extend(game.tick());
    out
}

#[test]
fn golden_a_idle_hand_checks_clean() {
    let golden = include_str!("../../../../tests/scripts/converse_humanoid/a_idle_hand.mgs");
    assert_eq!(check_script(golden), Ok(()));
}

#[test]
fn golden_b_first_deal_checks_clean() {
    let golden = include_str!("../../../../tests/scripts/converse_humanoid/b_first_deal.mgs");
    assert_eq!(check_script(golden), Ok(()));
}

#[test]
fn golden_c_rejected_inputs_check_clean() {
    let golden = include_str!("../../../../tests/scripts/converse_humanoid/c_rejected.mgs");
    assert_eq!(check_script(golden), Ok(()));
}

/// A one-off search for golden B's seed (spec section 9): the first seed whose
/// first deal is NPC `[0, 4, 8]` against player `[0, 4, 8]`. Run it with
/// `--run-ignored only --no-capture`.
#[test]
#[ignore = "one-off seed search; run with --run-ignored only"]
fn find_golden_b_seed() {
    for seed in 0..100_000 {
        let mut game =
            ConverseHumanoidGame::new(&MinigameSession::offline("ConverseBasicHumanoid", 1, seed));
        game.started();
        if game.npc_hand == [0.0, 4.0, 8.0] && game.player_hand == [0.0, 4.0, 8.0] {
            println!("golden B seed {seed}");
            return;
        }
    }
    panic!("no seed below 100000 deals [0, 4, 8] to both sides");
}

#[test]
fn countdown_subtracts_exactly_a_quarter_second() {
    let mut game = started(7);
    game.tick();
    assert_eq!(game.time_remaining, 30.0);
}

#[test]
fn timers_hold_after_newdeal_until_processreset() {
    let mut game = started(7);
    finish_deal(&mut game);
    // 24 ticks is one hand: with the hold, no NPC move and no hand end.
    assert!(tick_n(&mut game, 24).is_empty());
    assert!(game.needs_tick());
    let out = game.message("processreset", &HashMap::new());
    assert_eq!(sent_commands(&out), ["fullgamestate"]);
    assert_eq!(game.num_resets, 1);
    assert_eq!(game.contests_without_accord, 1);
    assert_eq!(game.game_type, 1, "logic3 steps to mediation5");
    assert_eq!(game.npc_last_time, game.time_remaining);
    assert_eq!(game.hand_last_time, game.time_remaining);
}

#[test]
fn a_deal_played_by_ticks_ends_in_newdeal_then_holds() {
    let mut game = started(7);
    let mut out = Vec::new();
    for _ in 0..200 {
        out.extend(play_and_tick(&mut game));
        if sent_commands(&out).iter().any(|c| c.as_str() == "newdeal") {
            break;
        }
    }
    assert!(
        sent_commands(&out).iter().any(|c| c.as_str() == "newdeal"),
        "the deal ends on the hand timer"
    );
    assert!(game.awaiting_reset);
    assert!(
        game.player_score + game.npc_score > 0.0,
        "the last hand was scored before newdeal"
    );
    assert!(tick_n(&mut game, 24).is_empty());
    assert!(game.message("newhand", &HashMap::new()).is_empty());
    let out = game.message("processreset", &HashMap::new());
    assert_eq!(sent_commands(&out), ["fullgamestate"]);
    assert!(!game.awaiting_reset);
    assert_eq!(game.game_type, 1, "logic3 steps to mediation5");
}

#[test]
fn processreset_before_newdeal_is_refused_even_when_all_cards_are_played() {
    let mut game = started(7);
    let card_total = GAME_TYPES[game.game_type].card_total as u32;
    game.player_total_plays = card_total;
    game.npc_total_plays = card_total;
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(game.num_resets, 0);
}

#[test]
fn npc_winning_last_hand_is_scored_by_the_hand_timer() {
    let mut game = started(7);
    // The last card of each side: the player's 0 against the NPC's 5, which
    // wins (spec section 6). The NPC has not answered yet.
    game.player_hand = vec![0.0];
    game.npc_hand = vec![5.0];
    game.player_total_plays = 2;
    game.npc_total_plays = 2;
    let fields_play = fields(&[("num", 0.0), ("suit", 0.0)]);
    assert!(!game.message("playermove", &fields_play).is_empty());
    // Before the hand ends the deal is not held: processreset is refused.
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    let out = tick_n(&mut game, 30);
    assert!(sent_commands(&out).iter().any(|c| c.as_str() == "newdeal"));
    assert_eq!(game.npc_score, 2.0, "the NPC's point and its sweep");
    assert_eq!(game.player_score, 0.0);
    assert!(game.awaiting_reset);
}

#[test]
fn processreset_before_the_deal_ends_is_refused() {
    let mut game = started(7);
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(game.num_resets, 0);
    assert_eq!(game.game_type, 0);
}

#[test]
fn client_victory_is_refused_with_no_output() {
    let mut game = started(7);
    assert!(game.message("victory", &HashMap::new()).is_empty());
    assert_eq!(game.outcome, None);
    assert!(game.needs_tick());
}

#[test]
fn client_failure_is_a_defeat() {
    let mut game = started(7);
    let out = game.message("failure", &HashMap::new());
    assert_eq!(sent_commands(&out), ["failure"]);
    assert!(has_failure(&out));
    assert_eq!(game.outcome, Some(Outcome::Defeat));
    assert!(!game.needs_tick());
}

#[test]
fn refusals_are_logged_only_up_to_the_cap() {
    // Counted through `reject`, so the WARN rows are what the gate limits.
    let capture = crate::test_support::LogCapture::install();
    let mut game = started(7);
    for _ in 0..REFUSAL_LOG_CAP + 24 {
        assert!(game.message("unknown_command", &HashMap::new()).is_empty());
    }
    let rows = capture
        .all()
        .iter()
        .filter(|row| row.level == tracing::Level::WARN && row.message_contains("command refused"))
        .count();
    assert_eq!(rows, REFUSAL_LOG_CAP as usize);
}

#[test]
fn commands_and_ticks_after_the_end_give_nothing() {
    let mut game = started(7);
    game.message("failure", &HashMap::new());
    assert!(game
        .message("playermove", &fields(&[("num", 0.0), ("suit", 0.0)]))
        .is_empty());
    assert!(game.message("newhand", &HashMap::new()).is_empty());
    assert!(game.tick().is_empty());
}

#[test]
fn unknown_and_item_cheat_commands_give_nothing() {
    let mut game = started(7);
    assert!(game.message("nosuchcommand", &HashMap::new()).is_empty());
    assert!(game
        .message("activateitemcheat", &HashMap::new())
        .is_empty());
    assert_eq!(game.outcome, None);
}

#[test]
fn timeout_with_a_tie_is_a_victory() {
    let mut game = started(7);
    game.time_remaining = 0.25;
    let out = game.tick();
    assert_eq!(sent_commands(&out), ["victory"]);
    assert!(has_victory(&out));
    assert_eq!(game.outcome, Some(Outcome::Victory));
    assert!(!game.needs_tick());
}

#[test]
fn timeout_with_the_npc_ahead_is_a_failure() {
    let mut game = started(7);
    game.time_remaining = 0.25;
    game.npc_score = 1.0;
    let out = game.tick();
    assert_eq!(sent_commands(&out), ["failure"]);
    assert!(has_failure(&out));
    assert_eq!(game.outcome, Some(Outcome::Defeat));
}

#[test]
fn aborted_ends_the_game_silently() {
    let mut game = started(7);
    assert!(game.aborted().is_empty());
    assert_eq!(game.outcome, Some(Outcome::Canceled));
    assert!(!game.needs_tick());
}

#[test]
fn activatevictory_refusals_then_one_success() {
    let mut game = started(7);
    // Unaffordable: score 0 against cost 3.
    assert!(game
        .message("activatevictory", &fields(&[("victoryNum", 1.0)]))
        .is_empty());
    // Out of range and not a whole number.
    game.player_score = 10.0;
    for bad in [0.0, 3.0, 1.5] {
        assert!(game
            .message("activatevictory", &fields(&[("victoryNum", bad)]))
            .is_empty());
    }
    assert!(game.message("activatevictory", &HashMap::new()).is_empty());
    assert_eq!(game.player_score, 10.0);
    // Success: cost 4 for the second condition, then `updatescore`.
    let out = game.message(
        "activatevictory",
        &fields(&[("victoryNum", 2.0), ("pointCost", 0.0)]),
    );
    assert_eq!(sent_commands(&out), ["activatevictory", "updatescore"]);
    assert_eq!(game.player_score, 6.0);
    assert!(game.player_victory_used[1]);
    // Used: the same condition is refused.
    assert!(game
        .message("activatevictory", &fields(&[("victoryNum", 2.0)]))
        .is_empty());
    assert_eq!(game.player_score, 6.0);
}

#[test]
fn converse_has_the_trump_bits_and_server_validation() {
    assert_eq!(ability_bits("ConverseBasicHumanoid"), 0x78000);
    assert_eq!(validation("ConverseBasicHumanoid"), "server");
}
