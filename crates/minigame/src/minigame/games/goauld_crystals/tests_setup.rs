//! Setup tests: the difficulty table and its clamps, the tutorial and random
//! goals, board generation, and the `fullgamestate` / `timerupdate` shape.

use std::collections::{HashMap, HashSet};

use super::setup::{clamp_difficulty, clamp_level, Tier, TABLE};
use super::GoauldCrystalsGame;
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

/// Numeric `fullgamestate` keys (spec section 5), every one of them.
const NUMBER_KEYS: [&str; 23] = [
    "playfieldWidth",
    "playfieldHeight",
    "startGoalSide",
    "startGoalOffset",
    "startGoalX",
    "startGoalY",
    "finishGoalSide",
    "finishGoalOffset",
    "finishGoalX",
    "finishGoalY",
    "selectedXPos",
    "selectedYPos",
    "timeRemaining",
    "maxTime",
    "timerState",
    "activeItemCheat",
    "instcc",
    "CA0",
    "CA1",
    "CA2",
    "CA3",
    "CA4",
    "abilityBitfield",
];

fn make_session(difficulty: u32, tech: u32, seed: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "GoauldCrystals".to_string(),
        difficulty,
        tech_competency: tech,
        seed,
        abilities_mask: 5,
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

/// A game with its first board generated, as `started()` leaves it.
fn started_game(difficulty: u32, tech: u32, seed: u32) -> GoauldCrystalsGame {
    let mut game = GoauldCrystalsGame::new(&make_session(difficulty, tech, seed));
    game.init_game();
    game
}

/// Squared start-to-finish distance, the measure the goal rule uses.
fn goal_gap(game: &GoauldCrystalsGame) -> usize {
    game.start.x.abs_diff(game.finish.x).pow(2) + game.start.y.abs_diff(game.finish.y).pow(2)
}

fn count_color(game: &GoauldCrystalsGame, color: u8) -> usize {
    game.board
        .iter()
        .flatten()
        .filter(|c| c.color == color)
        .count()
}

fn filled_cells(game: &GoauldCrystalsGame) -> usize {
    game.board.iter().flatten().filter(|c| c.color != 0).count()
}

#[test]
fn table_ends_match_the_spec() {
    assert_eq!(
        TABLE[0][0],
        Tier {
            w: 4,
            h: 4,
            time: 24,
            obstacles: 2
        }
    );
    assert_eq!(
        TABLE[0][3],
        Tier {
            w: 7,
            h: 5,
            time: 16,
            obstacles: 5
        }
    );
    assert_eq!(
        TABLE[10][0],
        Tier {
            w: 11,
            h: 11,
            time: 48,
            obstacles: 12
        }
    );
    assert_eq!(
        TABLE[10][3],
        Tier {
            w: 14,
            h: 12,
            time: 40,
            obstacles: 22
        }
    );
}

#[test]
fn tech_competency_maps_to_rows_and_clamps_at_ten() {
    assert_eq!(clamp_level(1), 0);
    assert_eq!(clamp_level(5), 0);
    assert_eq!(clamp_level(6), 1);
    assert_eq!(clamp_level(55), 10);
    assert_eq!(clamp_level(60), 10);
}

#[test]
fn difficulty_clamps_to_one_to_four() {
    assert_eq!(clamp_difficulty(0), 1);
    assert_eq!(clamp_difficulty(2), 2);
    assert_eq!(clamp_difficulty(7), 4);
    let game = GoauldCrystalsGame::new(&make_session(7, 60, 9));
    assert_eq!((game.level, game.difficulty), (10, 4));
}

#[test]
fn seed_one_on_4x4_uses_the_tutorial_goals_and_board() {
    let game = started_game(2, 1, 1);
    assert_eq!((game.w, game.h), (4, 4));
    assert_eq!(
        (game.start.side, game.start.x, game.start.y),
        (3, 0, 0),
        "start is side 3, offset 0"
    );
    assert_eq!(
        (game.finish.side, game.finish.x, game.finish.y),
        (1, 3, 2),
        "finish is side 1, offset 2"
    );
    // Rows are y, columns are x (spec section 3).
    let rows: Vec<Vec<u8>> = (0..4)
        .map(|y| (0..4).map(|x| game.board[x][y].color).collect())
        .collect();
    assert_eq!(
        rows,
        vec![
            vec![1, 4, 2, 2],
            vec![1, 1, 1, 1],
            vec![0, 0, 0, 4],
            vec![0, 0, 0, 1],
        ]
    );
}

#[test]
fn finish_keeps_its_distance_on_the_smallest_and_largest_tiers() {
    for seed in 2..60 {
        // 4x4 at L0 difficulty 1: min distance 3.
        let small = started_game(1, 1, seed);
        assert!(goal_gap(&small) >= 9, "seed {seed}: 4x4 goals too close");
        // 14x12 at L10 difficulty 4: min distance 13.
        let large = started_game(4, 56, seed);
        assert_eq!((large.w, large.h), (14, 12));
        assert!(
            goal_gap(&large) >= 169,
            "seed {seed}: 14x12 goals too close"
        );
    }
}

#[test]
fn capacity_loop_keeps_h_cells_empty() {
    for seed in 2..80 {
        // On 4x4 the budget starts over capacity, so the shrink loop runs.
        let small = started_game(1, 1, seed);
        assert!(
            filled_cells(&small) <= small.w * small.h - small.h,
            "seed {seed}: 4x4 board has fewer than h empty cells"
        );
        let large = started_game(4, 56, seed);
        assert!(
            filled_cells(&large) <= large.w * large.h - large.h,
            "seed {seed}: 14x12 board has fewer than h empty cells"
        );
    }
}

#[test]
fn fractional_counts_place_their_ceiling() {
    // 9x8 at L5 difficulty 3: yellows = 8.5 + 1 = 9.5, placed as 10;
    // reds = 18 + 10 = 28; greens = manhattan gap + 3. Capacity 64 is not hit.
    for seed in 2..40 {
        let game = started_game(3, 26, seed);
        assert_eq!((game.w, game.h), (9, 8));
        let manhattan = game.start.x.abs_diff(game.finish.x) + game.start.y.abs_diff(game.finish.y);
        assert_eq!(count_color(&game, 1), manhattan + 3, "seed {seed}: greens");
        assert_eq!(count_color(&game, 2), 10, "seed {seed}: yellows");
        assert_eq!(count_color(&game, 4), 28, "seed {seed}: reds");
    }
}

#[test]
fn started_sends_one_full_game_state_with_every_key() {
    let mut game = GoauldCrystalsGame::new(&make_session(1, 1, 7));
    let out = game.started();
    let [GameOutput::Send(state)] = out.as_slice() else {
        panic!("started must send exactly one message");
    };

    let expected: HashSet<&str> = NUMBER_KEYS
        .iter()
        .copied()
        .chain(["_cmd", "gameArray"])
        .collect();
    let keys: HashSet<&str> = state.keys().map(String::as_str).collect();
    assert_eq!(keys, expected);

    assert_eq!(state["_cmd"].as_str(), Some("fullgamestate"));
    for key in NUMBER_KEYS {
        assert!(
            matches!(state[key], SfsValue::Number(_)),
            "{key} must be a number"
        );
    }
    assert_eq!(state["selectedXPos"].as_f64(), Some(-1.0));
    assert_eq!(state["selectedYPos"].as_f64(), Some(-1.0));
    assert_eq!(state["timerState"].as_f64(), Some(0.0));
    assert_eq!(state["activeItemCheat"].as_f64(), Some(-1.0));
    assert_eq!(state["instcc"].as_f64(), Some(-1.0));
    assert_eq!(state["abilityBitfield"].as_f64(), Some(5.0));

    // gameArray: a list of columns, each a list of {color, var, power} cells.
    let SfsValue::Object(columns) = &state["gameArray"] else {
        panic!("gameArray must be a list");
    };
    assert_eq!(columns.len(), game.w);
    let SfsValue::Object(column) = &columns["0"] else {
        panic!("column must be a list");
    };
    assert_eq!(column.len(), game.h);
    let SfsValue::Object(cell) = &column["0"] else {
        panic!("cell must be an object");
    };
    assert!(matches!(cell["color"], SfsValue::Number(_)));
    assert!(matches!(cell["var"], SfsValue::Number(_)));
    assert!(matches!(cell["power"], SfsValue::Bool(false)));
}

#[test]
fn full_game_state_reports_a_selection() {
    let mut game = started_game(1, 1, 7);
    game.selected = Some((2, 1));
    let state = game.build_full_game_state();
    assert_eq!(state["selectedXPos"].as_f64(), Some(2.0));
    assert_eq!(state["selectedYPos"].as_f64(), Some(1.0));
}

#[test]
fn timer_update_carries_clock_and_state() {
    let game = started_game(1, 1, 7);
    let update = game.build_timer_update();
    assert_eq!(update["_cmd"].as_str(), Some("timerupdate"));
    assert_eq!(update["timeRemaining"].as_f64(), Some(24.0));
    assert_eq!(update["timerState"].as_f64(), Some(0.0));
}

#[test]
fn unported_commands_are_ignored_and_no_tick_is_needed() {
    let mut game = started_game(1, 1, 7);
    assert!(game.message("playfieldclick", &HashMap::new()).is_empty());
    assert!(game.tick().is_empty());
    assert!(!game.needs_tick());
}
