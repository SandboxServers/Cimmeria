//! Tests for the Alignment swap and matching (MG-AL2a, spec sections 3, 5, 6
//! and 9).

use std::collections::{HashMap, HashSet};

use super::moves::{self, SwapReject};
use super::*;

fn make_session(difficulty: u32, tech: u32, seed: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        session_instance_id: 0,
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
        terminal_reported: false,
        cancel: tokio::sync::watch::channel(false).0,
    }
}

/// The spec section 9 fixture as pieces: `[4, 5, 6][(x + 2 * (y % 2)) % 3]`.
fn fixture_pieces() -> Vec<Vec<Piece>> {
    (0..8)
        .map(|x| {
            (0..8)
                .map(|y| Piece::plain([4, 5, 6][(x + 2 * (y % 2)) % 3]))
                .collect()
        })
        .collect()
}

/// The fixture with `(2, 2)` set to 4 (transcript A).
fn transcript_a_board() -> Vec<Vec<u8>> {
    let mut board = fixture_pieces();
    board[2][2] = Piece::plain(4);
    board
        .iter()
        .map(|column| column.iter().map(|piece| piece.kind).collect())
        .collect()
}

/// A `swappieces` request: `cells` are `(x, y, type)`, sent as `row<n>` = x,
/// `col<n>` = y, `type<n>` = type.
pub(super) fn swap_params(cells: [(i32, i32, u8); 3]) -> HashMap<String, SfsValue> {
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

/// Transcript A's request: cell 1 `(1, 1)` with its neighbours `(2, 1)` and
/// `(2, 0)`.
pub(super) fn transcript_a_params() -> HashMap<String, SfsValue> {
    swap_params([(1, 1, 4), (2, 1, 5), (2, 0, 6)])
}

/// The `Send` payload of the output at `index`, which must be a message.
fn sent(outputs: &[GameOutput], index: usize) -> &HashMap<String, SfsValue> {
    match outputs.get(index) {
        Some(GameOutput::Send(vars)) => vars,
        other => panic!("expected a Send output at {index}, got {other:?}"),
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

/// The `matches` of a `hexspinner` as `(x, y, type)`, sorted.
fn match_cells(vars: &HashMap<String, SfsValue>) -> Vec<(u32, u32, u32)> {
    let mut cells: Vec<(u32, u32, u32)> = items(&vars["matches"])
        .iter()
        .map(|cell| {
            let values = match cell {
                SfsValue::Object(map) => map,
                other => panic!("expected a cell object, got {other:?}"),
            };
            let get = |key: &str| values[key].as_f64().unwrap() as u32;
            (get("x"), get("y"), get("type"))
        })
        .collect();
    cells.sort_unstable();
    cells
}

/// The board the game holds, as plain kinds.
fn kinds(game: &AlignmentGame) -> Vec<Vec<u8>> {
    game.board
        .iter()
        .map(|column| column.iter().map(|piece| piece.kind).collect())
        .collect()
}

/// A game on the fixture board with `board` written over some cells.
fn game_with(cells: &[(usize, usize, Piece)]) -> AlignmentGame {
    let mut board = fixture_pieces();
    for &(x, y, piece) in cells {
        board[x][y] = piece;
    }
    AlignmentGame::with_pieces(&make_session(1, 1, 7), board)
}

/// A power-up expansion of one matched cell `(4, 4)` holding `power`, on an
/// empty 9x9 board.
fn expanded_from_centre(power: u8) -> HashSet<(usize, usize)> {
    let mut board = vec![vec![Piece::plain(0); 9]; 9];
    board[4][4] = Piece { kind: 4, power };
    let (set, _) = moves::check_power_up_matches(&board, &[(4, 4)]);
    set.into_iter().collect()
}

#[test]
fn neighbour_table_matches_both_row_parities() {
    let board = vec![vec![Piece::plain(0); 8]; 8];
    let even: Vec<_> = (0..6).map(|n| setup::neighbour(&board, 2, 2, n)).collect();
    assert_eq!(
        even,
        [(1, 2), (1, 3), (2, 3), (3, 2), (2, 1), (1, 1)]
            .map(Some)
            .to_vec()
    );
    let odd: Vec<_> = (0..6).map(|n| setup::neighbour(&board, 2, 1, n)).collect();
    assert_eq!(
        odd,
        [(1, 1), (2, 2), (3, 2), (3, 1), (3, 0), (2, 0)]
            .map(Some)
            .to_vec()
    );
    assert_eq!(setup::neighbour(&board, 0, 0, 0), None);
}

#[test]
fn match_pair_rule_needs_two_equal_neighbours() {
    // (2, 1) odd: n1 = (2, 2) and n2 = (3, 2) form a triangle with it.
    let triangle = game_with(&[
        (2, 1, Piece::plain(4)),
        (2, 2, Piece::plain(4)),
        (3, 2, Piece::plain(4)),
    ]);
    let seeds = setup::check_for_matches(&triangle.board);
    assert!(seeds.contains(&(2, 1)) && seeds.contains(&(2, 2)) && seeds.contains(&(3, 2)));

    // The same pair with a different third type makes no match.
    // (1, 1) and (3, 2) become 5, so (2, 1) has one equal neighbour and no pair.
    let open = game_with(&[
        (1, 1, Piece::plain(5)),
        (2, 1, Piece::plain(4)),
        (2, 2, Piece::plain(4)),
        (3, 2, Piece::plain(5)),
    ]);
    assert!(!setup::check_for_matches(&open.board).contains(&(2, 1)));

    // A blocker never matches, even between two equal neighbours.
    let blocked = game_with(&[
        (2, 1, Piece::plain(9)),
        (2, 2, Piece::plain(9)),
        (3, 2, Piece::plain(9)),
    ]);
    assert!(setup::check_for_matches(&blocked.board).is_empty());
}

#[test]
fn transcript_a_swap_matches_and_replies_feedback_then_hexspinner() {
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    game.message("resethinttimer", &HashMap::new());
    let outputs = game.message("swappieces", &transcript_a_params());

    assert_eq!(outputs.len(), 2);
    let feedback = sent(&outputs, 0);
    assert_eq!(feedback["_cmd"].as_str(), Some("feedback"));
    assert_eq!(feedback["spinCount"].as_f64(), Some(1.0));
    assert!(numbers(&feedback["powerUpValues"]).is_empty());

    let spinner = sent(&outputs, 1);
    assert_eq!(spinner["_cmd"].as_str(), Some("hexspinner"));
    assert_eq!(spinner["spinCount"].as_f64(), Some(1.0));
    assert_eq!(
        match_cells(spinner),
        vec![(1, 3, 4), (2, 1, 4), (2, 2, 4), (3, 0, 4), (3, 2, 4)]
    );
    // The request fields are echoed back.
    assert_eq!(spinner["row1"].as_f64(), Some(1.0));
    assert_eq!(spinner["col3"].as_f64(), Some(0.0));
    assert_eq!(spinner["orientation"].as_str(), Some("e"));

    assert_eq!(game.phase, Phase::AwaitCascade);
    // The removed cells are emptied, the rest keeps the rotation.
    assert_eq!(game.board[2][1].kind, 0);
    assert_eq!(game.board[1][1].kind, 6);
}

#[test]
fn swap_matching_only_on_rotation_two_reports_spin_count_two() {
    // Transcript A's triangle with the cells listed in another order: rotation 1
    // makes no match, rotation 2 does.
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    let params = swap_params([(1, 1, 4), (2, 0, 6), (2, 1, 5)]);
    let outputs = game.message("swappieces", &params);

    assert_eq!(outputs.len(), 2);
    assert_eq!(sent(&outputs, 0)["spinCount"].as_f64(), Some(2.0));
    assert_eq!(sent(&outputs, 1)["spinCount"].as_f64(), Some(2.0));
    assert_eq!(
        match_cells(sent(&outputs, 1)),
        vec![(1, 3, 4), (2, 1, 4), (2, 2, 4), (3, 0, 4), (3, 2, 4)]
    );
    assert_eq!(game.phase, Phase::AwaitCascade);
}

#[test]
fn transcript_b_no_match_swap_keeps_the_board() {
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_pieces(&session, fixture_pieces());
    let before = kinds(&game);
    let params = swap_params([(1, 1, 4), (1, 0, 5), (0, 1, 6)]);
    let outputs = game.message("swappieces", &params);

    assert_eq!(outputs.len(), 1, "no feedback on a no-match swap");
    let spinner = sent(&outputs, 0);
    assert_eq!(spinner["spinCount"].as_f64(), Some(3.0));
    assert!(match_cells(spinner).is_empty());
    assert_eq!(kinds(&game), before);
    assert_eq!(game.phase, Phase::Ready);
}

#[test]
fn each_swap_reject_reason_is_its_own() {
    let board = fixture_pieces();
    let reject = |cells| {
        let params = swap_params(cells);
        let cells = moves::read_cells(&params).unwrap();
        moves::check_swap(&board, cells, &params)
    };
    // Out of bounds: x = 8 on an 8x8 board.
    assert_eq!(
        reject([(8, 1, 4), (2, 1, 5), (2, 0, 6)]),
        Err(SwapReject::OutOfBounds)
    );
    // Not a triangle: the cells are distinct but not mutually adjacent.
    assert_eq!(
        reject([(1, 1, 4), (3, 3, 5), (0, 0, 6)]),
        Err(SwapReject::NotTriangle)
    );
    // Two neighbours of (1, 1) that do not touch each other: the third pair fails.
    assert_eq!(
        reject([(1, 1, 4), (0, 1, 6), (2, 2, 6)]),
        Err(SwapReject::NotTriangle)
    );
    // A fractional coordinate is refused, not truncated to cell 1.
    let mut fractional = swap_params([(1, 1, 4), (2, 1, 5), (2, 0, 6)]);
    fractional.insert("row1".to_string(), SfsValue::Number(1.5));
    assert_eq!(moves::read_cells(&fractional), Err(SwapReject::OutOfBounds));
    // Client types that differ from the board.
    assert_eq!(
        reject([(1, 1, 5), (2, 1, 5), (2, 0, 6)]),
        Err(SwapReject::TypeMismatch)
    );
    // A blocker on the board.
    let mut blocked = fixture_pieces();
    blocked[2][1] = Piece::plain(9);
    let params = swap_params([(1, 1, 4), (2, 1, 9), (2, 0, 6)]);
    let cells = moves::read_cells(&params).unwrap();
    assert_eq!(
        moves::check_swap(&blocked, cells, &params),
        Err(SwapReject::Blocker)
    );
    // A valid triangle with the board's types passes.
    assert_eq!(reject([(1, 1, 4), (2, 1, 5), (2, 0, 6)]), Ok(()));
}

#[test]
fn each_swap_reject_leaves_the_board_equal_and_silent() {
    let session = make_session(1, 1, 7);
    let rejected = [
        swap_params([(8, 1, 4), (2, 1, 5), (2, 0, 6)]),
        swap_params([(1, 1, 4), (3, 3, 5), (0, 0, 6)]),
        swap_params([(1, 1, 5), (2, 1, 5), (2, 0, 6)]),
    ];
    for params in rejected {
        let mut game = AlignmentGame::with_pieces(&session, fixture_pieces());
        let before = kinds(&game);
        let outputs = game.message("swappieces", &params);
        assert!(outputs.is_empty());
        assert_eq!(kinds(&game), before);
        assert_eq!(game.phase, Phase::Ready);
    }
}

#[test]
fn swap_while_not_ready_is_refused() {
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    game.phase = Phase::AwaitCascade;
    let before = kinds(&game);
    let outputs = game.message("swappieces", &transcript_a_params());
    assert!(outputs.is_empty());
    assert_eq!(kinds(&game), before);
}

#[test]
fn power_up_shapes_one_to_six() {
    // One matched cell (4, 4) on a 9x9 board; (4, 4) is an even row.
    let neighbours = [(3, 4), (3, 5), (4, 5), (5, 4), (4, 3), (3, 3)];
    let row: Vec<(usize, usize)> = (0..9).map(|x| (x, 4)).collect();
    let diagonal_n1_n4 = [
        (3, 5),
        (3, 6),
        (2, 7),
        (2, 8),
        (4, 3),
        (5, 2),
        (5, 1),
        (6, 0),
    ];
    let diagonal_n2_n5 = [
        (4, 5),
        (5, 6),
        (5, 7),
        (6, 8),
        (3, 3),
        (3, 2),
        (2, 1),
        (2, 0),
    ];
    let set = |cells: &[(usize, usize)]| -> HashSet<(usize, usize)> {
        let mut all: HashSet<_> = cells.iter().copied().collect();
        all.insert((4, 4));
        all
    };
    assert_eq!(expanded_from_centre(1), set(&neighbours));
    assert_eq!(expanded_from_centre(2), set(&row));
    assert_eq!(expanded_from_centre(4), set(&diagonal_n1_n4));
    assert_eq!(expanded_from_centre(5), set(&diagonal_n2_n5));
    // Power 3: the six neighbours and the twelve cells of the outer ring.
    assert_eq!(expanded_from_centre(3).len(), 19);
    // Power 6: the row and both diagonals, the centre counted once.
    let star: Vec<_> = row
        .iter()
        .chain(diagonal_n1_n4.iter())
        .chain(diagonal_n2_n5.iter())
        .copied()
        .collect();
    assert_eq!(expanded_from_centre(6), set(&star));
}

#[test]
fn power_up_pulled_in_by_another_fires_its_own_effect_once() {
    // (4, 4) power 1 pulls in (4, 5), which holds power 3 and fires its own
    // radius-2 shape: (4, 7) is two steps from (4, 5) and not from (4, 4).
    let mut board = vec![vec![Piece::plain(0); 9]; 9];
    board[4][4] = Piece { kind: 4, power: 1 };
    board[4][5] = Piece { kind: 4, power: 3 };
    let (set, values) = moves::check_power_up_matches(&board, &[(4, 4)]);
    assert_eq!(values, vec![1, 3]);
    assert!(set.contains(&(4, 7)));
    let unique: HashSet<_> = set.iter().copied().collect();
    assert_eq!(unique.len(), set.len(), "no cell is added twice");
}

#[test]
fn get_match_power_follows_the_spec_table() {
    assert_eq!(moves::get_match_power(1, 8), Some(1));
    assert_eq!(moves::get_match_power(6, 8), Some(1));
    assert_eq!(moves::get_match_power(7, 8), Some(2));
    assert_eq!(moves::get_match_power(8, 8), Some(3));
    assert_eq!(moves::get_match_power(4, 12), Some(2));
    assert_eq!(moves::get_match_power(5, 12), Some(4));
    assert_eq!(moves::get_match_power(6, 12), Some(5));
    assert_eq!(moves::get_match_power(8, 12), Some(6));
    assert_eq!(moves::get_match_power(3, 12), Some(6));
    // Type 7 under mp 12 is the shipped NaN bug.
    assert_eq!(moves::get_match_power(7, 12), None);
}

#[test]
fn reset_hint_timer_is_silent() {
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_pieces(&session, fixture_pieces());
    assert!(game.message("resethinttimer", &HashMap::new()).is_empty());
    assert_eq!(game.phase, Phase::Ready);
}

#[test]
fn process_reset_is_refused_in_ready_and_mid_cascade() {
    let session = make_session(1, 1, 7);
    let mut game = AlignmentGame::with_board(&session, transcript_a_board());
    let before = kinds(&game);
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(kinds(&game), before);
    assert_eq!(game.num_resets, 0);
    assert_eq!(game.phase, Phase::Ready);

    game.message("swappieces", &transcript_a_params());
    assert_eq!(game.phase, Phase::AwaitCascade);
    let mid = kinds(&game);
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(kinds(&game), mid);
    assert_eq!(game.num_resets, 0);
    assert_eq!(game.phase, Phase::AwaitCascade);
}

#[test]
fn logged_client_text_is_cut_at_the_cap() {
    assert_eq!(log_text(&"x".repeat(1000)).len(), LOG_CAP);
    assert_eq!(log_text(&"é".repeat(100)).chars().count(), LOG_CAP);
    assert_eq!(log_text("processreset"), "processreset");
}

#[test]
fn long_client_command_is_cut_in_the_refusal_log() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut game = AlignmentGame::with_board(&make_session(1, 1, 7), transcript_a_board());
    assert!(game.message(&"x".repeat(1000), &HashMap::new()).is_empty());

    let event = capture
        .find_message(Level::WARN, "command refused")
        .expect("a refusal is logged");
    // Fields are Debug-quoted, so the cut text is `LOG_CAP` characters plus two quotes.
    assert!(
        event.fields["cmd"].chars().count() <= LOG_CAP + 2,
        "the logged command is cut: {event:#?}"
    );
}
