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

/// The fixture board as pieces with no power-up.
fn fixture_pieces() -> Vec<Vec<Piece>> {
    fixture_board()
        .iter()
        .map(|column| column.iter().map(|&kind| Piece::plain(kind)).collect())
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
    assert_eq!(setup::clamp_tier(0, 1, None), 1);
    assert_eq!(setup::clamp_tier(9, 1, None), 4);
    assert_eq!(setup::clamp_tier(3, 1, None), 3);
}

#[test]
fn techcomp_maps_to_level_and_clamps() {
    assert_eq!(setup::level_for(0, 1, None), 0);
    assert_eq!(setup::level_for(1, 1, None), 0);
    assert_eq!(setup::level_for(6, 1, None), 1);
    assert_eq!(setup::level_for(55, 1, None), 10);
    assert_eq!(setup::level_for(60, 1, None), 10);
}

#[test]
fn first_quarter_of_cells_are_bonus_icons() {
    // Checked before the re-roll, which may move a bonus icon to a goal cell.
    let board = setup::fill_board(&mut StdRng::seed_from_u64(7), 1);
    let size = board.len();
    for (x, column) in board.iter().enumerate() {
        for (y, piece) in column.iter().enumerate() {
            let index = x * size + y;
            let kind = piece.kind;
            if index < size * size / 4 {
                assert!(BONUS_ICONS.contains(&kind), "cell {x},{y} = {kind}");
            } else {
                assert!(!BONUS_ICONS.contains(&kind), "cell {x},{y} = {kind}");
            }
        }
    }
}

#[test]
fn power_ups_only_sit_in_the_bonus_quarter() {
    // Over 50 seeds the 5% roll lands at least once (P(none) is about 1e-18).
    let mut found = false;
    for seed in 0..50 {
        let game = AlignmentGame::new(&make_session(1, 1, seed));
        for (x, column) in game.board.iter().enumerate() {
            for (y, piece) in column.iter().enumerate() {
                assert!(piece.power <= 1, "power {} at {x},{y}", piece.power);
                if piece.power != 0 {
                    assert!(
                        x * 8 + y < 16,
                        "power-up outside the bonus quarter at {x},{y}"
                    );
                    found = true;
                }
            }
        }
    }
    assert!(found, "no power-up was generated");
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

/// `select_icons` draws the same board from the same RNG seed, and another seed
/// draws another one. The seeded path is the test and repl path; `new` seeds
/// from the OS (authority F2).
#[test]
fn same_rng_seed_gives_same_board_and_other_seed_differs() {
    use rand::SeedableRng;
    let first = setup::select_icons(&mut rand::rngs::StdRng::seed_from_u64(7), 1);
    let again = setup::select_icons(&mut rand::rngs::StdRng::seed_from_u64(7), 1);
    let other = setup::select_icons(&mut rand::rngs::StdRng::seed_from_u64(8), 1);
    assert_eq!(first, again);
    assert_ne!(first, other);
}

/// F2: the session seed does not fix the board, so two games of one session
/// seed draw different boards (an 8x8 board of six types never repeats).
#[test]
fn new_boards_are_not_fixed_by_the_session_seed() {
    let first = AlignmentGame::new(&make_session(1, 1, 7));
    let again = AlignmentGame::new(&make_session(1, 1, 7));
    assert_ne!(first.board, again.board);
}

#[test]
fn fixture_board_has_no_match() {
    assert!(setup::check_for_matches(&fixture_pieces()).is_empty());
}

#[test]
fn triangle_seeds_and_flood_take_their_groups() {
    // Type 4 at (0,0), (0,1) and (1,0) is a triangle. (0,2) is in no triangle
    // of its own, but it touches (0,1) and so joins the flood. Blockers fill
    // the rest of the board.
    let mut board = vec![vec![Piece::plain(9); 4]; 4];
    for (x, y) in [(0, 0), (0, 1), (1, 0), (0, 2)] {
        board[x][y] = Piece::plain(4);
    }
    let seeds = setup::check_for_matches(&board);
    assert_eq!(seeds, vec![(0, 0), (0, 1), (1, 0)]);
    let group = setup::check_neighbor_matches(&board, &seeds);
    assert_eq!(group, vec![(0, 0), (0, 1), (1, 0), (0, 2)]);
}

#[test]
fn power_up_matches_by_base_type_and_is_rerolled_as_a_goal_icon() {
    // A power-up on type 1 sits in a triangle of type 1, so it matches. The
    // re-roll replaces it with a plain goal icon: a power-up is never a bonus
    // seed, so it loses its power.
    let mut board = vec![vec![Piece::plain(9); 4]; 4];
    board[0][0] = Piece { kind: 1, power: 1 };
    board[0][1] = Piece::plain(1);
    board[1][0] = Piece::plain(1);
    assert_eq!(
        setup::check_for_matches(&board),
        vec![(0, 0), (0, 1), (1, 0)]
    );
    setup::reroll_matches(&mut StdRng::seed_from_u64(3), &mut board, &[4, 5, 6]);
    assert!(setup::check_for_matches(&board).is_empty());
    let cell = board[0][0];
    assert!((4..=6).contains(&cell.kind), "kind {}", cell.kind);
    assert_eq!(cell.power, 0);
}

#[test]
fn power_up_wire_number_carries_the_power() {
    // `kind + power / 10`: a type 2 piece with power 3 is sent as 2.3.
    let mut board = fixture_pieces();
    board[0][0] = Piece { kind: 2, power: 3 };
    let mut game = AlignmentGame::with_pieces(&make_session(1, 1, 7), board);
    let outputs = game.started();
    let first_column = items(&sent(&outputs)["gameArray"])[0];
    let cells = numbers(first_column);
    assert!((cells[0] - 2.3).abs() < 1e-9, "sent {}", cells[0]);
    assert_eq!(cells[1].fract(), 0.0);
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
fn process_reset_is_refused_and_changes_nothing() {
    let mut game = AlignmentGame::with_board(&make_session(1, 1, 7), fixture_board());
    game.started();
    game.board[0][0] = Piece::plain(9);
    game.time_remaining = 50.25;
    game.goal_scores = vec![9, 9, 9];
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(game.num_resets, 0);
    assert_eq!(game.time_remaining, 50.25);
    assert_eq!(game.goal_scores, vec![9, 9, 9]);
    assert_eq!(game.board[0][0].kind, 9);
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
fn abort_is_silent_and_ends_the_game() {
    let mut game = AlignmentGame::new(&make_session(1, 1, 7));
    game.started();
    let outputs = game.aborted();
    assert!(outputs.is_empty());
    assert_eq!(game.outcome, Some("canceled"));
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert!(game.aborted().is_empty());
}

#[test]
fn port_stays_off_and_the_shell_never_ticks() {
    const { assert!(!PORTED) };
    assert!(!AlignmentGame::new(&make_session(1, 1, 7)).needs_tick());
}
