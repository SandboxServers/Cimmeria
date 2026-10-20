//! Power tests: the breadth-first path and its visit order, the finish check,
//! the `powerstatus` shape, and the loop that never starts a solved board.

use super::power::{build_power_status, finish_powered, update_power};
use super::{Cell, Goal, GoauldCrystalsGame};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

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

/// A board from colour rows, top row first: `rows[y][x]` is stored at
/// `board[x][y]`, the layout the client's `gameArray` uses.
fn hand_board(rows: &[&[u8]]) -> Vec<Vec<Cell>> {
    let w = rows[0].len();
    (0..w)
        .map(|x| {
            rows.iter()
                .map(|row| Cell {
                    color: row[x],
                    var: 0,
                    power: false,
                })
                .collect()
        })
        .collect()
}

fn goal_at(x: usize, y: usize) -> Goal {
    Goal {
        side: 0,
        offset: 0,
        x,
        y,
    }
}

/// Every cell with its `power` flag set, in board order.
fn powered_cells(board: &[Vec<Cell>]) -> Vec<(usize, usize)> {
    board
        .iter()
        .enumerate()
        .flat_map(|(x, column)| {
            column
                .iter()
                .enumerate()
                .filter(|(_, cell)| cell.power)
                .map(move |(y, _)| (x, y))
        })
        .collect()
}

#[test]
fn tutorial_board_powers_the_spec_path_in_visit_order() {
    let mut game = GoauldCrystalsGame::new(&make_session(2, 1, 1));
    game.init_game();
    assert_eq!((game.finish.x, game.finish.y), (3, 2));

    let visited = update_power(&mut game.board, game.start);
    assert_eq!(visited, vec![(0, 0), (0, 1), (1, 1), (2, 1), (3, 1)]);

    let mut flagged = powered_cells(&game.board);
    flagged.sort();
    let mut expected = visited.clone();
    expected.sort();
    assert_eq!(flagged, expected);
    assert!(!finish_powered(&game.board, game.finish));
}

#[test]
fn breadth_first_visit_order_follows_the_neighbour_order() {
    // A full 3x3 green board: the order differs under a reordered neighbour
    // list, a depth-first walk, or a mark-on-dequeue search.
    let mut board = hand_board(&[&[1, 1, 1], &[1, 1, 1], &[1, 1, 1]]);
    let visited = update_power(&mut board, goal_at(1, 1));
    assert_eq!(
        visited,
        vec![
            (1, 1),
            (0, 1),
            (1, 0),
            (2, 1),
            (1, 2),
            (0, 0),
            (0, 2),
            (2, 0),
            (2, 2),
        ]
    );
    assert_eq!(powered_cells(&board).len(), 9);
}

#[test]
fn a_start_that_is_not_green_powers_nothing() {
    // The start is red, so the green neighbours are never reached. A stale
    // flag from an earlier board is cleared too.
    let mut board = hand_board(&[&[4, 1], &[1, 1]]);
    board[1][1].power = true;
    let visited = update_power(&mut board, goal_at(0, 0));
    assert!(visited.is_empty());
    assert!(powered_cells(&board).is_empty());
}

#[test]
fn a_green_row_powers_the_finish_and_an_orange_cell_cuts_it() {
    let mut board = hand_board(&[&[1, 1, 1, 1], &[0, 0, 0, 0]]);
    let finish = goal_at(3, 0);
    update_power(&mut board, goal_at(0, 0));
    assert!(finish_powered(&board, finish));

    // Only colour 1 carries power: an orange cell in the row cuts the path.
    board[2][0].color = 3;
    let visited = update_power(&mut board, goal_at(0, 0));
    assert_eq!(visited, vec![(0, 0), (1, 0)]);
    assert!(!finish_powered(&board, finish));
}

#[test]
fn power_status_lists_the_cells_in_visit_order() {
    let state = build_power_status(&[(0, 0), (0, 1), (1, 1)]);
    assert_eq!(state["_cmd"].as_str(), Some("powerstatus"));
    let SfsValue::Object(array) = &state["powerArray"] else {
        panic!("powerArray must be a list");
    };
    assert_eq!(array.len(), 3);
    for (i, (x, y)) in [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)].into_iter().enumerate() {
        let SfsValue::Object(cell) = &array[&i.to_string()] else {
            panic!("each entry must be an object");
        };
        assert_eq!(cell["xPos"].as_f64(), Some(x));
        assert_eq!(cell["yPos"].as_f64(), Some(y));
    }
}

#[test]
fn generated_boards_never_start_with_the_finish_powered() {
    // Tier L0 difficulty 1 is the 4x4 table row; seed 1 is the tutorial, so
    // the seeds here are the random boards.
    for seed in 2..202 {
        let mut game = GoauldCrystalsGame::new(&make_session(1, 1, seed));
        game.init_game();
        assert!(
            !finish_powered(&game.board, game.finish),
            "seed {seed} started with the finish powered"
        );
    }
}
