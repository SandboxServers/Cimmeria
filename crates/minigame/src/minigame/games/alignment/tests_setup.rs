//! Tests for the Alignment state shell and setup (MG-AL1).

use std::collections::HashMap;

use super::setup::{self, BONUS_ICONS};
use super::*;

fn make_session(difficulty: u32, tech: u32, seed: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "Alignment".to_string(),
        difficulty,
        tech_competency: tech,
        seed,
        abilities_mask: 0,
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

/// The spec section 9 fixture board: `[4, 5, 6][(x + 2 * (y % 2)) % 3]`.
fn fixture_board() -> Vec<Vec<u8>> {
    (0..8)
        .map(|x| (0..8).map(|y| [4, 5, 6][(x + 2 * (y % 2)) % 3]).collect())
        .collect()
}

/// The `Send` payload of the first output, which must be a message.
fn sent(outputs: &[GameOutput]) -> &HashMap<String, SfsValue> {
    match outputs.first() {
        Some(GameOutput::Send(vars)) => vars,
        other => panic!("expected a Send output, got {other:?}"),
    }
}

/// The items of an SFS array, in index order.
fn items(value: &SfsValue) -> Vec<&SfsValue> {
    let SfsValue::Object(map) = value else {
        panic!("expected an array object, got {value:?}");
    };
    (0..map.len()).map(|i| &map[&i.to_string()]).collect()
}

fn numbers(value: &SfsValue) -> Vec<f64> {
    items(value).iter().map(|v| v.as_f64().unwrap()).collect()
}

fn board_of(vars: &HashMap<String, SfsValue>) -> Vec<Vec<u8>> {
    items(&vars["gameArray"])
        .iter()
        .map(|column| numbers(column).iter().map(|&v| v as u8).collect())
        .collect()
}

#[test]
fn goal_totals_match_spec_spot_rows() {
    assert_eq!(setup::goal_totals(1, 0), vec![45, 15, 6]);
    assert_eq!(setup::goal_totals(1, 10), vec![180, 125, 60]);
    assert_eq!(setup::goal_totals(2, 0), vec![63, 30, 18]);
    assert_eq!(setup::goal_totals(3, 10), vec![216, 140, 72]);
    assert_eq!(setup::goal_totals(4, 0), vec![63, 30, 23, 6]);
    assert_eq!(setup::goal_totals(4, 10), vec![216, 140, 90, 60]);
}

#[test]
fn tier_table_matches_spec_section_4() {
    assert_eq!(setup::tier_spec(1).time, 120);
    assert_eq!(setup::tier_spec(1).goal_icons, &[4, 5, 6]);
    assert_eq!(setup::tier_spec(3).time, 100);
    assert_eq!(setup::tier_spec(4).time, 80);
    assert_eq!(setup::tier_spec(4).goal_icons, &[4, 5, 6, 8, 7]);
    assert_eq!(BONUS_ICONS, [1, 2, 3]);
}

#[test]
fn difficulty_clamps_to_tier_range() {
    assert_eq!(setup::clamp_tier(0), 1);
    assert_eq!(setup::clamp_tier(9), 4);
    assert_eq!(setup::clamp_tier(3), 3);
}

#[test]
fn techcomp_maps_to_level_and_clamps() {
    assert_eq!(setup::level_for(0), 0);
    assert_eq!(setup::level_for(1), 0);
    assert_eq!(setup::level_for(6), 1);
    assert_eq!(setup::level_for(55), 10);
    assert_eq!(setup::level_for(60), 10);
}

#[test]
fn first_quarter_of_cells_are_bonus_icons() {
    let game = AlignmentGame::new(&make_session(1, 1, 7));
    let size = game.board.len();
    for x in 0..size {
        for y in 0..size {
            let index = x * size + y;
            let value = game.board[x][y];
            if index < size * size / 4 {
                assert!(BONUS_ICONS.contains(&value), "cell {x},{y} = {value}");
            } else {
                assert!(!BONUS_ICONS.contains(&value), "cell {x},{y} = {value}");
            }
        }
    }
}

#[test]
fn generated_boards_have_no_initial_match() {
    for tier in 1..=4 {
        for seed in 0..5 {
            let game = AlignmentGame::new(&make_session(tier, 1, seed));
            assert!(setup::check_for_matches(&game.board).is_empty());
        }
    }
}

#[test]
fn same_seed_gives_same_board_and_other_seed_differs() {
    let first = AlignmentGame::new(&make_session(1, 1, 7));
    let again = AlignmentGame::new(&make_session(1, 1, 7));
    let other = AlignmentGame::new(&make_session(1, 1, 8));
    assert_eq!(first.board, again.board);
    assert_ne!(first.board, other.board);
}

#[test]
fn fixture_board_has_no_match() {
    assert!(setup::check_for_matches(&fixture_board()).is_empty());
}

#[test]
fn triangle_matches_and_flood_takes_its_group() {
    // A 2x2 corner of type 4 forms a triangle at (0, 0), (0, 1), (1, 0); the
    // flood also takes (1, 1), a neighbour of the triangle with the same type.
    let mut board = vec![vec![5u8; 3]; 3];
    board[0][0] = 4;
    board[0][1] = 4;
    board[1][0] = 4;
    board[1][1] = 4;
    let matched = setup::check_for_matches(&board);
    assert!(matched.contains(&(0, 0)) && matched.contains(&(0, 1)) && matched.contains(&(1, 0)));
    assert!(matched.contains(&(1, 1)));
    assert!(!matched.contains(&(2, 2)));
}

#[test]
fn started_sends_full_game_state_with_every_key() {
    let mut game = AlignmentGame::new(&make_session(1, 1, 7));
    let outputs = game.started();
    let vars = sent(&outputs);
    for key in [
        "_cmd",
        "numResets",
        "timeRemaining",
        "timerState",
        "isSpectator",
        "gameArray",
        "playfieldWidth",
        "playfieldHeight",
        "goalIcons",
        "goalTotals",
        "goalScores",
        "techcomp",
        "difficulty",
        "minigamelevel",
        "instcc",
        "CA0",
        "CA1",
        "CA2",
        "CA3",
        "CA4",
        "abilityBitfield",
    ] {
        assert!(vars.contains_key(key), "missing {key}");
    }
    assert_eq!(vars["_cmd"].as_str(), Some("fullgamestate"));
    assert_eq!(vars["timeRemaining"].as_f64(), Some(120.0));
    assert_eq!(vars["timerState"].as_f64(), Some(1.0));
    assert_eq!(numbers(&vars["goalTotals"]), vec![45.0, 15.0, 6.0]);
    assert_eq!(numbers(&vars["goalScores"]), vec![0.0, 0.0, 0.0]);
    assert_eq!(vars["instcc"].as_f64(), Some(-1.0));
    assert_eq!(vars["playfieldWidth"].as_f64(), Some(8.0));
}

#[test]
fn process_reset_resends_the_initial_board() {
    let mut game = AlignmentGame::with_board(&make_session(1, 1, 7), fixture_board());
    game.started();
    game.board[0][0] = 9;
    let outputs = game.message("processreset", &HashMap::new());
    let vars = sent(&outputs);
    assert_eq!(vars["numResets"].as_f64(), Some(1.0));
    assert_eq!(vars["timeRemaining"].as_f64(), Some(120.0));
    assert_eq!(board_of(vars), fixture_board());
}

#[test]
fn with_board_sends_the_injected_board() {
    let mut game = AlignmentGame::with_board(&make_session(1, 1, 7), fixture_board());
    let outputs = game.started();
    assert_eq!(board_of(sent(&outputs)), fixture_board());
}

#[test]
fn client_victory_gives_no_output() {
    let mut game = AlignmentGame::new(&make_session(1, 1, 7));
    game.started();
    assert!(game.message("victory", &HashMap::new()).is_empty());
}

#[test]
fn abort_sends_failure_and_then_ignores_messages() {
    let mut game = AlignmentGame::new(&make_session(1, 1, 7));
    game.started();
    let outputs = game.aborted();
    assert_eq!(sent(&outputs)["_cmd"].as_str(), Some("failure"));
    assert_eq!(game.outcome, Some("canceled"));
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert!(game.aborted().is_empty());
}

#[test]
fn port_stays_off_and_the_shell_never_ticks() {
    const { assert!(!PORTED) };
    assert!(!AlignmentGame::new(&make_session(1, 1, 7)).needs_tick());
}
