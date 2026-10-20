//! Move tests: transcripts B (steps 1-5) and C (steps 1-3) of spec section 9
//! message by message, the path search, the yellow rule, refused clicks,
//! the `reds_to_revert` step and a winning move.
//!
//! The tutorial board (`seed 1`, 4x4) is used throughout. Rows are `y`:
//! `1 4 2 2 / 1 1 1 1 / 0 0 0 4 / 0 0 0 1`, so start (0,0), finish (3,2).

use std::collections::HashMap;

use super::{Cell, GoauldCrystalsGame};
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

fn make_session() -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        session_instance_id: 0,
        game_name: "GoauldCrystals".to_string(),
        difficulty: 2,
        tech_competency: 1,
        seed: 1,
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

/// A game that has joined (`started`), so the board is the tutorial board.
fn started() -> GoauldCrystalsGame {
    let mut game = GoauldCrystalsGame::new(&make_session());
    game.started();
    game
}

/// Sends `playfieldclick` with the given coordinates.
fn click(game: &mut GoauldCrystalsGame, x: f64, y: f64) -> Vec<GameOutput> {
    let params = HashMap::from([
        ("x".to_string(), SfsValue::Number(x)),
        ("y".to_string(), SfsValue::Number(y)),
    ]);
    game.message("playfieldclick", &params)
}

/// One line per output: the command with its key fields, so a test reads
/// like the transcript.
fn tags(out: &[GameOutput]) -> Vec<String> {
    out.iter()
        .map(|output| match output {
            GameOutput::Send(map) => tag(map),
            GameOutput::Victory => "Victory".to_string(),
            GameOutput::Failure => "Failure".to_string(),
        })
        .collect()
}

fn tag(map: &HashMap<String, SfsValue>) -> String {
    let cmd = text(map, "_cmd");
    match cmd.as_str() {
        "timerupdate" => format!(
            "timerupdate {} {}",
            num(map, "timeRemaining"),
            num(map, "timerState")
        ),
        "selectedupdate" => format!("selectedupdate {},{}", num(map, "xPos"), num(map, "yPos")),
        "triggersound" => format!("triggersound {}", text(map, "soundName")),
        "squarestate" => format!(
            "squarestate {},{} color {}",
            num(map, "xPos"),
            num(map, "yPos"),
            num(object(map, "info"), "color")
        ),
        "powerstatus" => {
            let cells: Vec<String> = items(map, "powerArray")
                .iter()
                .map(|cell| format!("({},{})", num(cell, "xPos"), num(cell, "yPos")))
                .collect();
            format!("powerstatus {}", cells.join(" "))
        }
        _ => cmd,
    }
}

fn text(map: &HashMap<String, SfsValue>, key: &str) -> String {
    map[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} is not a string"))
        .to_string()
}

fn num(map: &HashMap<String, SfsValue>, key: &str) -> f64 {
    map[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} is not a number"))
}

fn object<'a>(map: &'a HashMap<String, SfsValue>, key: &str) -> &'a HashMap<String, SfsValue> {
    match &map[key] {
        SfsValue::Object(inner) => inner,
        other => panic!("{key} is not an object: {other:?}"),
    }
}

/// The entries of a list value, in index order.
fn items<'a>(map: &'a HashMap<String, SfsValue>, key: &str) -> Vec<&'a HashMap<String, SfsValue>> {
    let list = object(map, key);
    (0..list.len())
        .map(|i| object(list, &i.to_string()))
        .collect()
}

#[test]
fn transcript_b_steps_one_to_five() {
    let mut game = GoauldCrystalsGame::new(&make_session());
    let join = game.started();
    assert_eq!(tags(&join), ["fullgamestate"]);
    let GameOutput::Send(state) = &join[0] else {
        panic!("join sends a map");
    };
    assert_eq!(num(state, "timeRemaining"), 16.0);
    assert_eq!(num(state, "timerState"), 0.0);

    assert_eq!(
        tags(&click(&mut game, 2.0, 0.0)),
        [
            "timerupdate 16 1",
            "selectedupdate 2,0",
            "triggersound select"
        ]
    );
    assert_eq!(
        tags(&click(&mut game, 2.0, 1.0)),
        [
            "powerstatus (0,0) (0,1) (1,1)",
            "squarestate 2,1 color 3",
            "squarestate 2,0 color 1",
            "triggersound action",
            "selectedupdate -1,-1",
        ]
    );
    assert_eq!(
        tags(&click(&mut game, 0.0, 0.0)),
        ["selectedupdate 0,0", "triggersound select"]
    );
    assert_eq!(
        tags(&click(&mut game, 3.0, 3.0)),
        ["selectedupdate -1,-1", "triggersound invalid"]
    );
}

#[test]
fn transcript_c_steps_one_to_three() {
    let mut game = started();
    assert_eq!(
        tags(&click(&mut game, 2.0, 1.0)),
        [
            "timerupdate 16 1",
            "selectedupdate 2,1",
            "triggersound select"
        ]
    );
    assert_eq!(
        tags(&click(&mut game, 2.0, 2.0)),
        [
            "powerstatus (0,0) (0,1) (1,1)",
            "squarestate 2,2 color 1",
            "squarestate 2,1 color 0",
            "triggersound action",
            "selectedupdate -1,-1",
        ]
    );
    assert_eq!(game.board[2][2].color, 1);
    assert_eq!(game.board[2][1].color, 0);
    assert_eq!(game.timer_state, 1);
}

#[test]
fn path_search_passes_through_empty_cells_only() {
    let mut game = started();
    // (2,2) is the only way from (2,1) to (2,3), and a red cell blocks it.
    game.board[2][2] = Cell {
        color: 4,
        var: 0,
        power: false,
    };
    click(&mut game, 2.0, 1.0);
    assert_eq!(
        tags(&click(&mut game, 2.0, 3.0)),
        ["selectedupdate -1,-1", "triggersound invalid"]
    );
    assert_eq!(game.board[2][1].color, 1);
    assert_eq!(game.board[2][3].color, 0);

    // With the cell empty the same click is a path move.
    game.board[2][2] = Cell::default();
    click(&mut game, 2.0, 1.0);
    let out = click(&mut game, 2.0, 3.0);
    assert_eq!(
        tags(&out),
        [
            "powerstatus (0,0) (0,1) (1,1)",
            "squarestate 2,3 color 1",
            "squarestate 2,1 color 0",
            "triggersound action",
            "selectedupdate -1,-1",
        ]
    );
    assert_eq!(game.board[2][3].color, 1);
}

#[test]
fn yellow_turns_orange_unless_a_yellow_is_owed() {
    // No yellow owed: the yellow that moves turns orange (transcript B step 3).
    let mut free = started();
    click(&mut free, 2.0, 0.0);
    click(&mut free, 2.0, 1.0);
    assert_eq!(free.board[2][1].color, 3);
    assert_eq!(free.board[2][0].color, 1);

    // One yellow owed: the yellow stays yellow and the owed count is spent.
    let mut owed = started();
    owed.yellows_without_orange = 1;
    click(&mut owed, 2.0, 0.0);
    click(&mut owed, 2.0, 1.0);
    assert_eq!(owed.board[2][1].color, 2);
    assert_eq!(owed.board[2][0].color, 1);
    assert_eq!(owed.yellows_without_orange, 0);
}

#[test]
fn out_of_bounds_and_fractional_clicks_are_dropped() {
    let mut game = started();
    for (x, y) in [
        (4.0, 0.0),
        (0.0, 4.0),
        (-1.0, 0.0),
        (1.5, 0.0),
        (0.0, f64::NAN),
    ] {
        assert!(
            click(&mut game, x, y).is_empty(),
            "click {x},{y} sent output"
        );
    }
    assert!(game.message("playfieldclick", &HashMap::new()).is_empty());
    assert_eq!(game.timer_state, 0, "a refused click starts no clock");
    assert_eq!(game.selected, None);
}

#[test]
fn reds_to_revert_follow_the_swap_and_undo_a_losing_move() {
    let mut game = started();
    // (1,1) stays where it is; (2,1) is the green that the move carries to (2,2).
    game.reds_to_revert = Some(vec![(1, 1), (2, 1)]);
    click(&mut game, 2.0, 1.0);
    let out = click(&mut game, 2.0, 2.0);
    assert_eq!(
        tags(&out),
        [
            "powerstatus (0,0) (0,1) (1,1)",
            "squarestate 2,2 color 1",
            "squarestate 2,1 color 0",
            "triggersound action",
            "selectedupdate -1,-1",
            "powerstatus (0,0) (0,1)",
            "fullgamestate",
        ]
    );
    assert_eq!(game.board[1][1].color, 4);
    assert_eq!(game.board[2][2].color, 4);
    assert_eq!(game.reds_to_revert, None);
}

#[test]
fn reds_to_revert_follows_a_tracked_destination_cell() {
    let mut game = started();
    // (2,1) is the green destination of the yellow swap from (2,0): the green
    // lands on (2,0), so the tracked coordinate follows it there and reverts it.
    game.reds_to_revert = Some(vec![(2, 1)]);
    click(&mut game, 2.0, 0.0);
    click(&mut game, 2.0, 1.0);
    assert_eq!(game.board[2][0].color, 4);
    assert_eq!(game.board[2][1].color, 3);
    assert_eq!(game.reds_to_revert, None);
}

#[test]
fn reds_to_revert_fires_after_a_refused_click() {
    let mut game = started();
    // (0,0) is green and (3,3) is a distant green: the pair is refused, and
    // step 9 still turns the tracked green (1,1) back to red.
    game.reds_to_revert = Some(vec![(1, 1)]);
    click(&mut game, 0.0, 0.0);
    let out = click(&mut game, 3.0, 3.0);
    assert_eq!(
        tags(&out),
        [
            "selectedupdate -1,-1",
            "triggersound invalid",
            "powerstatus (0,0) (0,1)",
            "fullgamestate",
        ]
    );
    assert_eq!(game.board[1][1].color, 4);
    assert_eq!(game.reds_to_revert, None);
}

#[test]
fn move_that_powers_the_finish_wins_and_then_ignores_input() {
    let mut game = started();
    // Transcript A's removal, done without the item cheat: the red finish
    // cell becomes empty, then the green at (3,3) moves into it.
    game.reds_to_remove = 1;
    assert_eq!(
        tags(&click(&mut game, 3.0, 2.0)),
        [
            "timerupdate 16 1",
            "squarestate 3,2 color 0",
            "selectedupdate 3,2",
            "triggersound select",
        ]
    );
    let out = click(&mut game, 3.0, 3.0);
    assert_eq!(tags(&out), ["fullgamestate", "victory", "Victory"]);
    let GameOutput::Send(state) = &out[0] else {
        panic!("the win sends fullgamestate first");
    };
    assert_eq!(num(state, "timerState"), 0.0);
    assert_eq!(game.timer_state, 0);
    assert_eq!(game.outcome, Some("victory"));

    assert!(click(&mut game, 0.0, 0.0).is_empty(), "input after the win");
    assert!(click(&mut game, 3.0, 3.0).is_empty(), "input after the win");
}

#[test]
fn reds_to_yellows_turns_a_red_yellow_before_the_selection() {
    let mut game = started();
    game.reds_to_yellows = 1;
    assert_eq!(
        tags(&click(&mut game, 1.0, 0.0)),
        [
            "timerupdate 16 1",
            "squarestate 1,0 color 2",
            "selectedupdate 1,0",
            "triggersound select",
        ]
    );
    assert_eq!(game.reds_to_yellows, 0);
}
