//! Tests for the Alignment ability cheats (MG-AL3b, spec sections 6 and 7):
//! each bit's effect, the `cheatNum` check, spend-once, the `Ready` gate, the
//! clear rules and the wall-clock deadline after a pause.

use std::time::Duration;

use super::tests_moves::transcript_a_params;
use super::*;

fn make_session(mask: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "Alignment".to_string(),
        difficulty: 1,
        tech_competency: 0,
        seed: 7,
        abilities_mask: mask,
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

/// An 8x8 board of types 4-6 with no match (the spec section 9 pattern).
fn plain_board() -> Vec<Vec<u8>> {
    (0..8)
        .map(|x| {
            (0..8)
                .map(|y| [4, 5, 6][(x + 2 * (y % 2)) % 3] as u8)
                .collect()
        })
        .collect()
}

/// Transcript A's board: the plain pattern with `(2, 2)` set to 4.
fn transcript_a_board() -> Vec<Vec<u8>> {
    let mut board = plain_board();
    board[2][2] = 4;
    board
}

/// A board of type 5 with one 4 and one 6: type 4 is the least common (ties go
/// to the lower type), so bits 4, 6, 7 and 12 clear the single cell at (0, 0).
fn least_common_board() -> Vec<Vec<u8>> {
    let mut board = vec![vec![5u8; 8]; 8];
    board[0][0] = 4;
    board[0][1] = 6;
    board
}

fn game(mask: u32, board: Vec<Vec<u8>>) -> AlignmentGame {
    AlignmentGame::with_board(&make_session(mask), board)
}

/// The parameters of `activateabilitycheat` for bit `number`.
fn cheat(number: f64) -> HashMap<String, SfsValue> {
    HashMap::from([("cheatNum".to_string(), SfsValue::Number(number))])
}

/// The `_cmd` of each `Send` frame, in order.
fn names(outputs: &[GameOutput]) -> Vec<String> {
    outputs
        .iter()
        .filter_map(|output| match output {
            GameOutput::Send(vars) => vars["_cmd"].as_str().map(str::to_string),
            _ => None,
        })
        .collect()
}

fn powered_cells(game: &AlignmentGame) -> usize {
    game.board.iter().flatten().filter(|p| p.power != 0).count()
}

#[test]
fn bit_zero_adds_five_seconds_and_reports_the_state() {
    let mut game = game(1 << 0, plain_board());
    let before = game.shown_time();
    let outputs = game.message("activateabilitycheat", &cheat(0.0));
    assert_eq!(game.shown_time(), before + 5.0);
    assert_eq!(names(&outputs), ["timerupdate", "abilitycheatstate"]);
    assert_eq!(game.abilities_mask, 0);
}

#[test]
fn bit_one_takes_ten_seconds_and_converts_three_cells() {
    let mut game = game(1 << 1, plain_board());
    let before = game.shown_time();
    let outputs = game.message("activateabilitycheat", &cheat(1.0));
    assert_eq!(game.shown_time(), before - 10.0);
    assert_eq!(
        names(&outputs),
        [
            "timerupdate",
            "powerupconvert",
            "powerupconvert",
            "powerupconvert",
            "abilitycheatstate"
        ]
    );
    assert_eq!(powered_cells(&game), 3);
    assert_eq!(game.hint_timer, 20.0);
    assert_eq!(game.hint_last_time, game.shown_time());
}

#[test]
fn bit_nine_does_what_bit_one_does() {
    let mut game = game(1 << 9, plain_board());
    let before = game.shown_time();
    game.message("activateabilitycheat", &cheat(9.0));
    assert_eq!(game.shown_time(), before - 10.0);
    assert_eq!(powered_cells(&game), 3);
}

#[test]
fn bit_two_stops_the_clock_on_the_next_matching_swap() {
    let mut game = game(1 << 2, transcript_a_board());
    game.message("activateabilitycheat", &cheat(2.0));
    assert!(game.stop_timer_match);
    assert_eq!(game.timer_state, 1, "nothing pauses before a match");
    game.message("swappieces", &transcript_a_params());
    assert_eq!(game.timer_state, 0);
}

#[test]
fn bit_three_adds_half_a_second_per_matching_swap() {
    let mut game = game(1 << 3, transcript_a_board());
    game.message("activateabilitycheat", &cheat(3.0));
    let before = game.shown_time();
    let outputs = game.message("swappieces", &transcript_a_params());
    assert_eq!(game.shown_time(), before + 0.5);
    assert!(names(&outputs).contains(&"timerupdate".to_string()));
}

#[test]
fn bit_eight_adds_half_a_second_per_matching_swap() {
    let mut game = game(1 << 8, transcript_a_board());
    game.message("activateabilitycheat", &cheat(8.0));
    let before = game.shown_time();
    game.message("swappieces", &transcript_a_params());
    assert_eq!(game.shown_time(), before + 0.5);
}

#[test]
fn bit_eleven_grants_two_seconds_only_after_it_is_spent_on_a_later_cheat() {
    // The add-time bonus depends on the state when the cheat starts: spending
    // bit 11 itself gives nothing, a later cheat gets +2 on top of its effect.
    let mut game = game((1 << 11) | (1 << 0), plain_board());
    let before = game.shown_time();
    game.message("activateabilitycheat", &cheat(11.0));
    assert_eq!(game.shown_time(), before, "bit 11 alone adds no time");
    assert!(game.ability_add_time);
    let outputs = game.message("activateabilitycheat", &cheat(0.0));
    assert_eq!(game.shown_time(), before + 5.0 + 2.0);
    assert_eq!(
        names(&outputs),
        ["timerupdate", "abilitycheatstate", "timerupdate"]
    );
}

#[test]
fn bit_thirteen_grants_two_seconds_per_power_up() {
    let mut game = game(1 << 13, plain_board());
    assert!(
        game.power_up_time(2).is_empty(),
        "nothing before bit 13 is spent"
    );
    game.message("activateabilitycheat", &cheat(13.0));
    let before = game.shown_time();
    let outputs = game.power_up_time(2);
    assert_eq!(game.shown_time(), before + 4.0);
    assert_eq!(names(&outputs), ["timerupdate", "timerupdate"]);
}

#[test]
fn bit_ten_is_spent_and_changes_nothing_else() {
    let mut game = game(1 << 10, plain_board());
    let before = game.shown_time();
    let outputs = game.message("activateabilitycheat", &cheat(10.0));
    assert_eq!(names(&outputs), ["abilitycheatstate"]);
    assert_eq!(game.shown_time(), before);
    assert_eq!(game.abilities_mask, 0);
    assert_eq!(powered_cells(&game), 0);
}

#[test]
fn clear_bits_remove_the_least_common_type_and_hand_over_to_the_cascade() {
    for bit in [4.0, 6.0, 7.0, 12.0] {
        let mut game = game(1 << bit as u32, least_common_board());
        let outputs = game.message("activateabilitycheat", &cheat(bit));
        assert_eq!(game.board[0][0].kind, 0, "bit {bit} clears (0, 0)");
        assert_eq!(game.board[0][1].kind, 6, "bit {bit} keeps the 6");
        assert_eq!(game.phase, Phase::AwaitCascade);
        assert!(game.cheat_cleared);
        assert!(names(&outputs).contains(&"bypasshexspinner".to_string()));
    }
}

#[test]
fn power_up_clear_bits_remove_every_power_up_cell() {
    for bit in [5.0, 14.0] {
        let mut board = plain_board();
        board[0][0] = 4;
        let mut game = game(1 << bit as u32, board);
        game.board[0][0].power = 2;
        game.board[3][3].power = 2;
        game.message("activateabilitycheat", &cheat(bit));
        assert_eq!(game.board[0][0].kind, 0, "bit {bit} clears a power-up");
        assert_eq!(game.board[3][3].kind, 0, "bit {bit} clears a power-up");
        assert_eq!(game.phase, Phase::AwaitCascade);
        assert!(game.cheat_cleared);
    }
}

#[test]
fn a_cheat_clear_is_never_scored() {
    let mut game = game(1 << 4, least_common_board());
    game.message("activateabilitycheat", &cheat(4.0));
    game.message("cascade", &HashMap::new());
    game.message("checkForCascade", &HashMap::new());
    assert!(game.current_scores.is_empty(), "cleared types were scored");
    assert!(!game.cheat_cleared, "the check consumes the flag");
}

#[test]
fn cheat_number_must_be_an_integer_from_zero_to_fourteen() {
    let mut game = game(u32::MAX, plain_board());
    let before = game.shown_time();
    let bad = [
        cheat(15.0),
        cheat(-1.0),
        cheat(2.5),
        HashMap::from([("cheatNum".to_string(), SfsValue::String("3".to_string()))]),
        HashMap::new(),
    ];
    for params in &bad {
        let refusals = game.refusals;
        assert!(game.message("activateabilitycheat", params).is_empty());
        assert_eq!(game.refusals, refusals + 1);
    }
    assert_eq!(game.abilities_mask, u32::MAX, "no bit was spent");
    assert_eq!(game.shown_time(), before);
}

#[test]
fn a_bit_not_held_is_refused_and_changes_nothing() {
    let mut game = game(1 << 0, plain_board());
    let before = game.shown_time();
    assert!(game.message("activateabilitycheat", &cheat(3.0)).is_empty());
    assert_eq!(game.abilities_mask, 1 << 0);
    assert_eq!(game.shown_time(), before);
}

#[test]
fn each_bit_is_spent_once_and_a_repeat_is_refused() {
    let mut game = game(1 << 0, plain_board());
    game.message("activateabilitycheat", &cheat(0.0));
    let after_first = game.shown_time();
    assert!(game.message("activateabilitycheat", &cheat(0.0)).is_empty());
    assert_eq!(game.shown_time(), after_first);
}

#[test]
fn cheats_are_refused_outside_ready() {
    let mut game = game(1 << 0, plain_board());
    game.phase = Phase::AwaitCascade;
    let before = game.shown_time();
    assert!(game.message("activateabilitycheat", &cheat(0.0)).is_empty());
    assert_eq!(game.abilities_mask, 1 << 0, "the bit is kept for later");
    assert_eq!(game.shown_time(), before);
}

#[test]
fn item_cheats_produce_no_output() {
    let mut game = game(u32::MAX, plain_board());
    assert!(game.message("activateitemcheat", &cheat(0.0)).is_empty());
}

#[test]
fn a_pause_never_expires_and_a_score_resumes_with_a_fresh_deadline() {
    let mut game = game(1 << 2, transcript_a_board());
    game.message("activateabilitycheat", &cheat(2.0));
    game.message("swappieces", &transcript_a_params());
    assert_eq!(game.timer_state, 0);
    // Pretend the wall clock ran far past its budget while paused.
    game.clock_start = Instant::now() - Duration::from_secs(1000);
    game.run_tick();
    assert_ne!(game.phase, Phase::Ended, "a paused clock cannot expire");
    // A score ends the pause: the clock resumes with a fresh budget.
    game.phase = Phase::AwaitScore;
    game.message("doscore", &HashMap::new());
    assert_eq!(game.timer_state, 1);
    assert!(!game.clock_expired());
    assert!(game.clock_budget > game.shown_time());
    game.run_tick();
    assert_ne!(game.phase, Phase::Ended, "the resumed clock is re-anchored");
}
