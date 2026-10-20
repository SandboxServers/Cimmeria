//! Tests for the Alignment cascade (MG-AL2b, spec section 6): the gravity
//! walks, the refill grants, `cascade` and `checkForCascade` (phase gates, the
//! one-shot step), and the F4 and F5 regressions of the swap reply.

use std::collections::{HashMap, HashSet};

use rand::rngs::StdRng;
use rand::SeedableRng;

use super::cascade::refill;
use super::moves::Cell;
use super::moves_gravity::{gravity, slide_step, walk_step, Slide};
use super::*;

fn make_session(seed: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "Alignment".to_string(),
        difficulty: 1,
        tech_competency: 0,
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

/// The spec section 9 fixture with `(2, 2)` set to 4: transcript A's board.
fn transcript_a_board() -> Vec<Vec<u8>> {
    (0..8)
        .map(|x| {
            (0..8)
                .map(|y| {
                    if (x, y) == (2, 2) {
                        4
                    } else {
                        [4, 5, 6][(x + 2 * (y % 2)) % 3] as u8
                    }
                })
                .collect()
        })
        .collect()
}

/// An 8x8 board of type 4 pieces with the listed cells emptied.
fn board_with_gaps(gaps: &[Cell]) -> Vec<Vec<Piece>> {
    let mut board = vec![vec![Piece::plain(4); 8]; 8];
    for &(x, y) in gaps {
        board[x][y] = Piece::plain(0);
    }
    board
}

/// The items of an SFS array, in index order.
fn items(value: &SfsValue) -> Vec<&SfsValue> {
    let SfsValue::Object(map) = value else {
        panic!("expected an array object, got {value:?}");
    };
    (0..map.len()).map(|i| &map[&i.to_string()]).collect()
}

/// The numeric fields `keys` of one array entry, in `keys` order.
fn fields(entry: &SfsValue, keys: &[&str]) -> Vec<f64> {
    let SfsValue::Object(map) = entry else {
        panic!("expected an object, got {entry:?}");
    };
    keys.iter().map(|key| map[*key].as_f64().unwrap()).collect()
}

/// The text of a String value.
fn text(value: &SfsValue) -> &str {
    match value {
        SfsValue::String(s) => s,
        other => panic!("expected a String, got {other:?}"),
    }
}

/// The `Send` payload of the output at `index`, which must be a message.
fn sent(outputs: &[GameOutput], index: usize) -> &HashMap<String, SfsValue> {
    match outputs.get(index) {
        Some(GameOutput::Send(vars)) => vars,
        other => panic!("expected a Send output at {index}, got {other:?}"),
    }
}

/// Transcript A's swap request: cell 1 `(1, 1)`, with `(2, 1)` and `(2, 0)`.
fn swap_params(cells: [(i32, i32, u8); 3]) -> HashMap<String, SfsValue> {
    let mut params = HashMap::new();
    for (n, &(x, y, kind)) in cells.iter().enumerate() {
        let n = n + 1;
        params.insert(format!("row{n}"), SfsValue::Number(f64::from(x)));
        params.insert(format!("col{n}"), SfsValue::Number(f64::from(y)));
        params.insert(format!("type{n}"), SfsValue::Number(f64::from(kind)));
    }
    params.insert("xPos".to_string(), SfsValue::Number(0.0));
    params.insert("yPos".to_string(), SfsValue::Number(0.0));
    params.insert("orientation".to_string(), SfsValue::String("e".to_string()));
    params
}

fn transcript_a_params() -> HashMap<String, SfsValue> {
    swap_params([(1, 1, 4), (2, 1, 5), (2, 0, 6)])
}

/// The swap that starts a cascade on transcript A's board.
fn game_after_swap() -> AlignmentGame {
    let session = make_session(7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    let outputs = game.message("swappieces", &transcript_a_params());
    assert_eq!(outputs.len(), 2, "transcript A swap matches");
    game
}

/// Transcript A step 4: the `hexmove` of `cascade` has exactly the eight moves
/// of the transcript, in the walk order, and five new cells.
#[test]
fn transcript_a_cascade_hexmove_matches_step_four() {
    let mut game = game_after_swap();
    let outputs = game.message("cascade", &HashMap::new());
    let vars = sent(&outputs, 0);
    assert_eq!(text(&vars["_cmd"]), "hexmove");
    assert_eq!(vars["direction"].as_f64(), Some(0.0));
    let moves: Vec<Vec<f64>> = items(&vars["hexesToMove"])
        .iter()
        .map(|entry| fields(entry, &["moveTotal", "x", "y", "type"]))
        .collect();
    let expected: Vec<Vec<f64>> = vec![
        vec![1.0, 0.0, 3.0, 6.0],
        vec![2.0, 1.0, 2.0, 5.0],
        vec![2.0, 0.0, 2.0, 4.0],
        vec![1.0, 1.0, 1.0, 6.0],
        vec![1.0, 0.0, 1.0, 6.0],
        vec![1.0, 2.0, 0.0, 5.0],
        vec![1.0, 1.0, 0.0, 5.0],
        vec![1.0, 0.0, 0.0, 4.0],
    ];
    assert_eq!(moves, expected);
    let refilled: HashSet<(u32, u32)> = items(&vars["newHexes"])
        .iter()
        .map(|entry| {
            let xy = fields(entry, &["x", "y"]);
            (xy[0] as u32, xy[1] as u32)
        })
        .collect();
    let cells: HashSet<(u32, u32)> = [(0, 0), (0, 1), (0, 2), (0, 3), (1, 2)].into();
    assert_eq!(refilled, cells);
    assert_eq!(game.phase, Phase::AwaitCheck);
}

/// Gravity direction 0 (spec table): the piece at `(0, 1)` slides over the two
/// gaps at `(1, 1)` and `(2, 1)` to `(2, 1)`.
#[test]
fn gravity_direction_zero_slides_along_the_row() {
    let mut board = board_with_gaps(&[(1, 1), (2, 1)]);
    let slides = gravity(&mut board, 0);
    let slide = Slide {
        gaps: 2,
        cell: (0, 1),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[2][1], Piece::plain(4));
    assert_eq!(board[0][1], Piece::plain(0));
}

/// Gravity direction 3: the piece at `(2, 1)` slides over the gaps at `(0, 1)`
/// and `(1, 1)` to `(0, 1)`.
#[test]
fn gravity_direction_three_slides_along_the_row() {
    let mut board = board_with_gaps(&[(0, 1), (1, 1)]);
    let slides = gravity(&mut board, 3);
    let slide = Slide {
        gaps: 2,
        cell: (2, 1),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[0][1], Piece::plain(4));
}

/// Gravity direction 1: the gap at `(1, 0)` takes the piece at `(0, 1)` (odd
/// row: the slide is `(x + 1, y - 1)`).
#[test]
fn gravity_direction_one_slides_down_the_diagonal() {
    let mut board = board_with_gaps(&[(1, 0)]);
    let slides = gravity(&mut board, 1);
    let slide = Slide {
        gaps: 1,
        cell: (0, 1),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[1][0], Piece::plain(4));
}

/// Gravity direction 2: the gap at `(0, 0)` takes the piece at `(0, 1)` (odd
/// row: the slide is `(x, y - 1)`).
#[test]
fn gravity_direction_two_slides_down_the_diagonal() {
    let mut board = board_with_gaps(&[(0, 0)]);
    let slides = gravity(&mut board, 2);
    let slide = Slide {
        gaps: 1,
        cell: (0, 1),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[0][0], Piece::plain(4));
}

/// Gravity direction 4: the gap at `(0, 7)` takes the piece at `(1, 6)` (even
/// row: the slide is `(x - 1, y + 1)`).
#[test]
fn gravity_direction_four_slides_up_the_diagonal() {
    let mut board = board_with_gaps(&[(0, 7)]);
    let slides = gravity(&mut board, 4);
    let slide = Slide {
        gaps: 1,
        cell: (1, 6),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[0][7], Piece::plain(4));
}

/// Gravity direction 5: the gap at `(1, 7)` takes the piece at `(1, 6)` (even
/// row: the slide is `(x, y + 1)`).
#[test]
fn gravity_direction_five_slides_up_the_diagonal() {
    let mut board = board_with_gaps(&[(1, 7)]);
    let slides = gravity(&mut board, 5);
    let slide = Slide {
        gaps: 1,
        cell: (1, 6),
        piece: Piece::plain(4),
    };
    assert!(slides.contains(&slide), "{slides:?}");
    assert_eq!(board[1][7], Piece::plain(4));
}

/// Every direction keeps the pieces: a slide moves a piece, never makes one.
#[test]
fn gravity_conserves_the_pieces_in_every_direction() {
    let gaps: Vec<Cell> = (0..8)
        .flat_map(|x| (0..8).map(move |y| (x, y)))
        .filter(|&(x, y)| (x * 3 + y * 5) % 4 == 0)
        .collect();
    for d in 0..=5 {
        let mut board = board_with_gaps(&gaps);
        let before = board.iter().flatten().filter(|p| p.kind != 0).count();
        gravity(&mut board, d);
        let after = board.iter().flatten().filter(|p| p.kind != 0).count();
        assert_eq!(before, after, "direction {d}");
    }
}

/// The walk is the inverse of the slide in every direction, so a line and its
/// slides share their cells.
#[test]
fn walk_step_is_the_inverse_of_slide_step() {
    for d in 0..=5 {
        for x in -3..=10 {
            for y in -3..=10 {
                let cell = (x, y);
                assert_eq!(slide_step(d, walk_step(d, cell)), cell, "direction {d}");
            }
        }
    }
}

/// A match of 8 of one type grants that type's power-up as the first refill,
/// in place of a new type (`A` list, `getMatchPower(4, 8)` = 1).
#[test]
fn eight_matched_grants_one_power_up_in_the_refill() {
    let mut board = vec![vec![Piece::plain(0); 8]; 8];
    let matches: Vec<(Cell, Piece)> = (0..8).map(|i| ((i, 0), Piece::plain(4))).collect();
    let mut rng = StdRng::seed_from_u64(1);
    let new_hexes = refill(&mut rng, &mut board, &matches, 0, 1);
    assert_eq!(new_hexes[0], ((7, 0), Piece { kind: 4, power: 1 }));
}

/// Twelve matched grants a `B` power-up: type 5 with `getMatchPower(5, 12)` = 4.
#[test]
fn twelve_matched_grants_the_b_power_up() {
    let mut board = vec![vec![Piece::plain(0); 8]; 8];
    let matches: Vec<(Cell, Piece)> = (0..12).map(|i| ((i % 8, 0), Piece::plain(5))).collect();
    let mut rng = StdRng::seed_from_u64(1);
    let new_hexes = refill(&mut rng, &mut board, &matches, 0, 1);
    assert_eq!(new_hexes[0], ((7, 0), Piece { kind: 5, power: 4 }));
}

/// `cascade` is refused outside `AwaitCascade`, here in `Ready`.
#[test]
fn cascade_is_refused_in_ready() {
    let mut game = AlignmentGame::with_board(&make_session(7), transcript_a_board());
    assert!(game.message("cascade", &HashMap::new()).is_empty());
    assert_eq!(game.phase, Phase::Ready);
}

/// `checkForCascade` is refused outside `AwaitCheck`, here in `AwaitCascade`.
#[test]
fn check_for_cascade_is_refused_in_await_cascade() {
    let mut game = game_after_swap();
    assert!(game.message("checkForCascade", &HashMap::new()).is_empty());
    assert_eq!(game.phase, Phase::AwaitCascade);
}

/// A cascade step is one-shot: a second `cascade` before `checkForCascade` is
/// refused.
#[test]
fn second_cascade_is_refused_until_the_check() {
    let mut game = game_after_swap();
    assert_eq!(game.message("cascade", &HashMap::new()).len(), 1);
    assert!(game.message("cascade", &HashMap::new()).is_empty());
}

/// A match on the board after the refill sends `bypasshexspinner` and waits
/// for the next `cascade`.
#[test]
fn check_for_cascade_with_a_match_loops_to_cascade() {
    let mut game = game_after_swap();
    game.message("cascade", &HashMap::new());
    let outputs = game.message("checkForCascade", &HashMap::new());
    if game.phase == Phase::AwaitCascade {
        let vars = sent(&outputs, 0);
        assert_eq!(text(&vars["_cmd"]), "bypasshexspinner");
        assert_eq!(game.cascade_count, 1);
    } else {
        assert_eq!(game.phase, Phase::AwaitScore);
        assert_eq!(game.drop_index, 1);
    }
}

/// Without a match `checkForCascade` sends `cascadecheckcomplete`, moves the
/// drop index on and awaits the score.
#[test]
fn check_for_cascade_without_a_match_awaits_the_score() {
    // The spec section 9 fixture has no triangle (transcript B's plain board).
    let plain: Vec<Vec<u8>> = (0..8)
        .map(|x| (0..8).map(|y| [4, 5, 6][(x + 2 * (y % 2)) % 3] as u8).collect())
        .collect();
    let mut game = AlignmentGame::with_board(&make_session(7), plain);
    game.phase = Phase::AwaitCheck;
    game.current_matches = vec![((0, 0), Piece::plain(4))];
    let outputs = game.message("checkForCascade", &HashMap::new());
    let vars = sent(&outputs, 0);
    assert_eq!(text(&vars["_cmd"]), "cascadecheckcomplete");
    assert_eq!(game.phase, Phase::AwaitScore);
    assert_eq!(game.drop_index, 1);
    assert_eq!(game.current_scores, vec![4]);
}

/// F4: a swap whose triangle holds an empty cell is refused, even when the
/// client's types match the board (`swap_empty`). Without the check the swap
/// would go through.
#[test]
fn swap_over_an_empty_cell_is_refused() {
    let mut board = transcript_a_board();
    board[1][1] = 0;
    let session = make_session(7);
    let mut game = AlignmentGame::with_board(&session, board);
    let outputs = game.message("swappieces", &swap_params([(1, 1, 0), (2, 1, 5), (2, 0, 6)]));
    assert!(outputs.is_empty());
    assert_eq!(game.phase, Phase::Ready);
    assert_eq!(game.board[1][1], Piece::plain(0));
}

/// F5: a non-number `xPos` is not echoed into `hexspinner`; a number `yPos`
/// and a String `orientation` are.
#[test]
fn hexspinner_echoes_only_typed_fields() {
    let session = make_session(7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    let mut params = transcript_a_params();
    params.insert("xPos".to_string(), SfsValue::String("<script>".to_string()));
    let outputs = game.message("swappieces", &params);
    let vars = sent(&outputs, 1);
    assert!(!vars.contains_key("xPos"));
    assert_eq!(vars["yPos"].as_f64(), Some(0.0));
    assert_eq!(text(&vars["orientation"]), "e");
}
