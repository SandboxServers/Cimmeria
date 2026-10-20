//! Ability cheat tests (spec sections 5 and 7): the server-side bit check and
//! clear, the `cheatNum` validation, each bit's effect, the edge removal and
//! the bit 13 revert, and the refusals after the end.
//!
//! The tutorial board (`seed 1`, difficulty 2) starts with 16 s on the clock.
//! Tests that need a known layout build a hand board instead.

use std::collections::HashMap;

use super::setup::goal_on;
use super::{Cell, GoauldCrystalsGame};
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::games::{GameKind, Validation};
use crate::minigame::protocol::SfsValue;
use crate::minigame::repl::check_script;
use crate::minigame::session::MinigameSession;

/// Every ability bit held: the session mask the game can grant (bits 0-14).
const ALL_BITS: u32 = 0x7FFF;

fn make_session(abilities_mask: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "GoauldCrystals".to_string(),
        difficulty: 2,
        tech_competency: 1,
        seed: 1,
        abilities_mask,
        intelligence: 0,
        player_level: 1,
        ticket: String::new(),
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        player_name: None,
        client_ip: None,
        created_at: tokio::time::Instant::now(),
        connected: false,
        cancel: tokio::sync::watch::channel(false).0,
    }
}

/// A game that has joined (`started`), so the board is the tutorial board.
fn started(abilities_mask: u32) -> GoauldCrystalsGame {
    let mut game = GoauldCrystalsGame::new(&make_session(abilities_mask));
    game.started();
    game
}

/// Sends a command with one numeric parameter.
fn send_number(game: &mut GoauldCrystalsGame, cmd: &str, key: &str, value: f64) -> Vec<GameOutput> {
    let params = HashMap::from([(key.to_string(), SfsValue::Number(value))]);
    game.message(cmd, &params)
}

/// Sends `activateabilitycheat` with the given `cheatNum`.
fn cheat(game: &mut GoauldCrystalsGame, bit: f64) -> Vec<GameOutput> {
    send_number(game, "activateabilitycheat", "cheatNum", bit)
}

/// Sends `playfieldclick` with the given coordinates.
fn click(game: &mut GoauldCrystalsGame, x: f64, y: f64) -> Vec<GameOutput> {
    let params = HashMap::from([
        ("x".to_string(), SfsValue::Number(x)),
        ("y".to_string(), SfsValue::Number(y)),
    ]);
    game.message("playfieldclick", &params)
}

/// One name per output: the `_cmd` of a message, or the result variant.
fn names(out: &[GameOutput]) -> Vec<String> {
    out.iter()
        .map(|output| match output {
            GameOutput::Send(map) => map["_cmd"].as_str().expect("_cmd is a string").to_string(),
            GameOutput::Victory => "Victory".to_string(),
            GameOutput::Failure => "Failure".to_string(),
        })
        .collect()
}

/// The `abilityBitfield` in the one `abilitycheatstate` of an output.
fn bitfield_sent(out: &[GameOutput]) -> Option<f64> {
    out.iter().find_map(|output| match output {
        GameOutput::Send(map) if map["_cmd"].as_str() == Some("abilitycheatstate") => {
            map["abilityBitfield"].as_f64()
        }
        _ => None,
    })
}

/// Replaces the board with one written as rows (`rows[y][x]`), the layout
/// the tutorial table uses. Goals and clock stay as they are.
fn hand_board(game: &mut GoauldCrystalsGame, rows: [[u8; 4]; 4]) {
    game.board = (0..4)
        .map(|x| {
            (0..4)
                .map(|y| Cell {
                    color: rows[y][x],
                    var: 0,
                    power: false,
                })
                .collect()
        })
        .collect();
}

/// Counts the green cells on the board.
fn greens(game: &GoauldCrystalsGame) -> usize {
    game.board
        .iter()
        .flatten()
        .filter(|cell| cell.color == 1)
        .count()
}

#[test]
fn goauld_crystals_grants_bits_0_to_14_and_is_server_validated() {
    assert_eq!(GameKind::GoauldCrystals.ability_bits(), ALL_BITS);
    assert_eq!(GameKind::GoauldCrystals.validation(), Validation::Server);
}

#[test]
fn a_held_bit_is_spent_before_its_effect_and_the_new_mask_is_sent() {
    let mut game = started(ALL_BITS);
    let out = cheat(&mut game, 0.0);
    assert_eq!(
        names(&out),
        [
            "timerupdate",
            "triggersound",
            "timerupdate",
            "abilitycheatstate"
        ]
    );
    assert_eq!(bitfield_sent(&out), Some(f64::from(ALL_BITS & !1)));
    assert_eq!(game.ability_bitfield, ALL_BITS & !1);
    assert_eq!(game.time_remaining, 21.0);
    // The same bit again is refused: nothing is sent and nothing changes.
    assert!(cheat(&mut game, 0.0).is_empty());
    assert_eq!(game.time_remaining, 21.0);
}

#[test]
fn an_unset_bit_is_refused_with_no_reply() {
    let mut game = started(0b10);
    assert!(cheat(&mut game, 0.0).is_empty());
    assert_eq!(game.ability_bitfield, 0b10);
    assert_eq!(
        game.timer_state, 0,
        "a refused cheat does not start the clock"
    );
    assert_eq!(game.refusals, 1);
}

#[test]
fn cheat_numbers_outside_0_to_14_are_refused_with_no_reply() {
    let mut game = started(ALL_BITS);
    for bad in [15.0, -1.0, 1.5, 1000.0] {
        assert!(cheat(&mut game, bad).is_empty(), "cheatNum {bad}");
    }
    let missing = game.message("activateabilitycheat", &HashMap::new());
    assert!(missing.is_empty());
    let text = HashMap::from([("cheatNum".to_string(), SfsValue::String("0".to_string()))]);
    assert!(game.message("activateabilitycheat", &text).is_empty());
    assert_eq!(game.ability_bitfield, ALL_BITS);
    assert_eq!(game.timer_state, 0);
}

#[test]
fn item_cheats_and_client_victory_give_no_output() {
    let mut game = started(ALL_BITS);
    assert!(send_number(&mut game, "activateitemcheat", "cheatNum", 0.0).is_empty());
    assert!(game.message("victory", &HashMap::new()).is_empty());
    assert_eq!(game.ability_bitfield, ALL_BITS);
    assert_eq!(game.outcome, None);
}

#[test]
fn no_cheat_is_taken_after_the_end() {
    let mut game = started(ALL_BITS);
    game.outcome = Some("victory");
    assert!(cheat(&mut game, 0.0).is_empty());
    assert_eq!(game.ability_bitfield, ALL_BITS);
}

#[test]
fn pause_holds_the_clock_for_twenty_ticks_then_one_timerupdate() {
    let mut game = started(ALL_BITS);
    let out = cheat(&mut game, 2.0);
    assert_eq!(game.timer_state, 2);
    assert_eq!(
        names(&out).last().map(String::as_str),
        Some("abilitycheatstate")
    );
    let held = game.time_remaining;
    for _ in 0..19 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(game.time_remaining, held, "a pause does not count down");
    let resumed = game.tick();
    assert_eq!(names(&resumed), ["timerupdate"]);
    assert_eq!(game.timer_state, 1);
    assert_eq!(
        game.time_remaining, held,
        "the resumed clock starts from the same time"
    );
}

#[test]
fn reverse_adds_a_quarter_second_per_tick_for_twenty_ticks() {
    let mut game = started(ALL_BITS);
    cheat(&mut game, 3.0);
    let held = game.time_remaining;
    for _ in 0..19 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(names(&game.tick()), ["timerupdate"]);
    assert_eq!(game.timer_state, 1);
    assert_eq!(game.time_remaining, held + 5.0);
}

#[test]
fn bit_one_takes_five_seconds_floors_at_zero_and_adds_two_greens() {
    let mut game = started(ALL_BITS);
    game.time_remaining = 3.0;
    let before = greens(&game);
    let out = cheat(&mut game, 1.0);
    assert_eq!(game.time_remaining, 0.0);
    assert_eq!(greens(&game), before + 2);
    assert!(names(&out).contains(&"fullgamestate".to_string()));
    assert!(game.tick().iter().any(|o| matches!(o, GameOutput::Failure)));
}

#[test]
fn bits_six_and_eight_each_owe_five_yellows() {
    let mut game = started(ALL_BITS);
    cheat(&mut game, 6.0);
    assert_eq!(game.yellows_without_orange, 5);
    cheat(&mut game, 8.0);
    assert_eq!(game.yellows_without_orange, 10);
}

#[test]
fn bit_nine_turns_the_selected_red_yellow_or_owes_two() {
    let mut game = started(ALL_BITS);
    click(&mut game, 1.0, 0.0);
    assert_eq!(
        game.board[1][0].color, 4,
        "(1,0) is red on the tutorial board"
    );
    cheat(&mut game, 9.0);
    assert_eq!(game.board[1][0].color, 2);
    assert_eq!(game.reds_to_yellows, 1);
    assert_eq!(game.selected, None);

    let mut blank = started(ALL_BITS);
    cheat(&mut blank, 9.0);
    assert_eq!(blank.reds_to_yellows, 2);
}

#[test]
fn bit_twelve_empties_the_selected_red_or_owes_one_removal() {
    let mut game = started(ALL_BITS);
    click(&mut game, 1.0, 0.0);
    cheat(&mut game, 12.0);
    assert_eq!(game.board[1][0].color, 0);
    assert_eq!(game.reds_to_remove, 0);

    let mut blank = started(ALL_BITS);
    cheat(&mut blank, 12.0);
    assert_eq!(blank.reds_to_remove, 1);
}

#[test]
fn bit_fourteen_adds_five_seconds_and_owes_removals() {
    let mut game = started(ALL_BITS);
    cheat(&mut game, 14.0);
    assert_eq!(game.time_remaining, 21.0);
    assert_eq!(game.reds_to_remove, 2);
}

#[test]
fn bits_ten_and_eleven_spend_the_bit_without_a_board_change() {
    let mut game = started(ALL_BITS);
    let out = cheat(&mut game, 10.0);
    assert_eq!(
        names(&out),
        ["timerupdate", "triggersound", "abilitycheatstate"]
    );
    cheat(&mut game, 11.0);
    assert!(game.item_cheats_add_time);
}

#[test]
fn bit_four_removes_the_bottom_row_and_keeps_the_greens() {
    let mut game = started(ALL_BITS);
    let before = greens(&game);
    let out = cheat(&mut game, 4.0);
    assert_eq!((game.w, game.h), (4, 3));
    assert!(game.board.iter().all(|column| column.len() == 3));
    assert_eq!(
        greens(&game),
        before,
        "the one green on the edge is re-added"
    );
    assert!(names(&out).contains(&"fullgamestate".to_string()));
}

#[test]
fn bit_five_skips_the_top_edge_because_the_start_is_on_it() {
    let mut game = started(ALL_BITS);
    cheat(&mut game, 5.0);
    assert_eq!((game.w, game.h), (4, 3), "the bottom row goes, not the top");
    assert_eq!((game.start.x, game.start.y), (0, 0));
    assert_eq!((game.finish.x, game.finish.y), (3, 2));
}

#[test]
fn bits_four_and_five_pick_their_edges_in_their_own_order() {
    let mut game = started(ALL_BITS);
    // Goals on the left and right sides leave the top and bottom rows free.
    game.start = goal_on(3, 1, 4, 4);
    game.finish = goal_on(1, 2, 4, 4);
    cheat(&mut game, 4.0);
    assert_eq!(game.h, 3);
    assert_eq!(
        (game.start.x, game.start.y),
        (0, 1),
        "bit 4 takes the bottom row"
    );
    cheat(&mut game, 5.0);
    assert_eq!(game.h, 2, "bit 5 takes the top row");
    assert_eq!((game.start.x, game.start.y), (0, 0));
    assert_eq!((game.finish.x, game.finish.y), (3, 1));
}

#[test]
fn bit_four_keeps_the_board_when_every_edge_has_a_goal() {
    let mut game = started(ALL_BITS);
    game.start = goal_on(0, 0, 4, 4);
    game.finish = goal_on(2, 3, 4, 4);
    cheat(&mut game, 4.0);
    assert_eq!((game.w, game.h), (4, 4));
    assert_eq!(
        game.ability_bitfield,
        ALL_BITS & !(1 << 4),
        "the bit is still spent"
    );
}

#[test]
fn removing_the_top_row_moves_goals_selection_and_cells_together() {
    let mut game = started(ALL_BITS);
    hand_board(&mut game, [[0; 4], [0; 4], [0; 4], [0, 0, 0, 1]]);
    game.start = goal_on(2, 2, 4, 4);
    game.finish = goal_on(1, 1, 4, 4);
    game.selected = Some((3, 3));
    cheat(&mut game, 4.0);
    assert_eq!(game.h, 3);
    assert_eq!((game.start.x, game.start.y), (2, 2));
    assert_eq!((game.finish.x, game.finish.y), (3, 0));
    assert_eq!(game.selected, Some((3, 2)));
    assert_eq!(game.board[3][2].color, 1);
}

#[test]
fn bit_thirteen_turns_a_third_of_the_reds_green_and_the_next_refused_move_reverts_them() {
    let mut game = started(ALL_BITS);
    // Three reds on the top row; the finish (3,2) is empty, so no turn can win.
    hand_board(
        &mut game,
        [[1, 4, 4, 4], [1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 1]],
    );
    let out = cheat(&mut game, 13.0);
    let tracked = game.reds_to_revert.clone().expect("reds are tracked");
    assert_eq!(tracked.len(), 1, "ceil(3 / 3) reds turn green");
    let (x, y) = tracked[0];
    assert_eq!(game.board[x][y].color, 1);
    let sent = names(&out);
    assert!(sent.contains(&"powerstatus".to_string()));
    assert!(!sent.contains(&"victory".to_string()));

    // Two greens that do not touch: refused, so the tracked cell turns red.
    click(&mut game, 0.0, 0.0);
    click(&mut game, 3.0, 3.0);
    assert_eq!(
        game.board[x][y].color, 4,
        "the revert runs after a move that does not win"
    );
    assert!(game.reds_to_revert.is_none(), "the tracked list is spent");
}

#[test]
fn a_cheat_that_powers_the_finish_ends_the_game_with_victory() {
    let mut game = started(ALL_BITS);
    // The finish (3,2) is the only red, and the green path reaches it. Bit 13
    // turns ceil(1 / 3) = 1 red green, which is the finish.
    hand_board(
        &mut game,
        [[1, 1, 1, 1], [1, 1, 1, 1], [0, 0, 0, 4], [0, 0, 0, 1]],
    );
    let out = cheat(&mut game, 13.0);
    assert_eq!(game.outcome, Some("victory"));
    assert!(out.iter().any(|o| matches!(o, GameOutput::Victory)));
    assert!(names(&out).contains(&"victory".to_string()));
    assert_eq!(game.timer_state, 0);
    assert!(game.tick().is_empty(), "nothing after the end");
}

#[test]
fn goldens_b_and_c_check_clean() {
    for golden in [
        include_str!("../../../../tests/scripts/goauld_crystals/b_move_timeout.mgs"),
        include_str!("../../../../tests/scripts/goauld_crystals/c_path_move.mgs"),
    ] {
        assert_eq!(check_script(golden), Ok(()));
    }
}

#[test]
fn random_greens_fill_empties_then_overwrite_only_when_asked() {
    let mut game = started(0);
    hand_board(&mut game, [[4; 4]; 4]);
    game.add_random_greens(2, false);
    assert_eq!(
        greens(&game),
        0,
        "no empties and no overwrite: nothing placed"
    );
    game.add_random_greens(2, true);
    assert_eq!(greens(&game), 2, "overwrite replaces two red cells");
}
