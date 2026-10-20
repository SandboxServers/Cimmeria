//! Tests for the Alignment rules after the cascade (MG-AL3a, spec sections 6
//! and 8): the score and its multiplier, the one-shot `doscore`, the victory
//! gate, the countdown and its wall clock, the idle hint prompt and `callhint`.

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

/// An 8x8 board of types 4-6 with no match or move (transcript B's board).
fn plain_board() -> Vec<Vec<u8>> {
    (0..8)
        .map(|x| {
            (0..8)
                .map(|y| [4, 5, 6][(x + 2 * (y % 2)) % 3] as u8)
                .collect()
        })
        .collect()
}

/// A game in `AwaitScore` holding `scores` and `cascades` after a cascade.
fn await_score(scores: Vec<Piece>, cascades: u32) -> AlignmentGame {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    game.phase = Phase::AwaitScore;
    game.current_scores = scores;
    game.cascade_count = cascades;
    game
}

/// The pieces that meet all three tier 1 goals at once (`goalTotals` 45, 15, 6).
fn goal_pieces() -> Vec<Piece> {
    let mut pieces = vec![Piece::plain(4); 45];
    pieces.extend(vec![Piece::plain(5); 15]);
    pieces.extend(vec![Piece::plain(6); 6]);
    pieces
}

/// The `Send` frames of an output, in order.
fn sends(outputs: &[GameOutput]) -> Vec<&HashMap<String, SfsValue>> {
    outputs
        .iter()
        .filter_map(|output| match output {
            GameOutput::Send(vars) => Some(vars),
            _ => None,
        })
        .collect()
}

fn text(value: &SfsValue) -> String {
    match value {
        SfsValue::String(s) => s.clone(),
        other => panic!("expected a string, got {other:?}"),
    }
}

/// The `_cmd` names of an output's frames, in order.
fn names(outputs: &[GameOutput]) -> Vec<String> {
    sends(outputs)
        .iter()
        .map(|vars| text(&vars["_cmd"]))
        .collect()
}

fn numbers(value: &SfsValue) -> Vec<f64> {
    let SfsValue::Object(map) = value else {
        panic!("expected an array object, got {value:?}");
    };
    (0..map.len())
        .map(|i| map[&i.to_string()].as_f64().unwrap())
        .collect()
}

/// The `(x, y, type)` of a `{x, y, type}` cell object.
fn cell(value: &SfsValue) -> (f64, f64, f64) {
    let SfsValue::Object(map) = value else {
        panic!("expected a cell object, got {value:?}");
    };
    let get = |key: &str| map[key].as_f64().unwrap();
    (get("x"), get("y"), get("type"))
}

#[test]
fn transcript_a_step_six_updates_the_score_then_the_timer() {
    let mut game = await_score(vec![Piece::plain(4); 5], 0);
    let out = game.do_score();
    assert_eq!(names(&out), ["updatescore", "timerupdate"]);
    let score = sends(&out)[0];
    assert_eq!(numbers(&score["goalScores"]), [5.0, 0.0, 0.0]);
    assert_eq!(numbers(&score["goalTotals"]), [45.0, 15.0, 6.0]);
    assert_eq!(numbers(&score["goalIcons"]), [4.0, 5.0, 6.0]);
    let timer = sends(&out)[1];
    assert_eq!(timer["timeRemaining"].as_f64(), Some(120.0));
    assert_eq!(timer["timerState"].as_f64(), Some(1.0));
    assert_eq!(game.phase, Phase::Ready);
    assert!(game.current_scores.is_empty());
    assert!(game.current_matches.is_empty());
    assert_eq!(game.cascade_count, 0);
}

#[test]
fn multiplier_and_bonus_follow_the_cascade_count() {
    // Four type-4 pieces at each cascade count: floor(4 * mult) and the bonus.
    let table = [
        (0, 4.0, 0.0),
        (1, 6.0, 2.0),
        (2, 12.0, 5.0),
        (3, 20.0, 8.0),
        (4, 40.0, 12.0),
        (7, 40.0, 12.0),
    ];
    for (cascades, points, bonus) in table {
        let mut game = await_score(vec![Piece::plain(4); 4], cascades);
        let out = game.do_score();
        assert_eq!(
            numbers(&sends(&out)[0]["goalScores"])[0],
            points,
            "cascades {cascades}"
        );
        assert_eq!(
            sends(&out)[1]["timeRemaining"].as_f64(),
            Some(120.0 + bonus),
            "cascades {cascades}"
        );
    }
}

#[test]
fn power_ups_and_type_seven_never_score() {
    let scores = vec![
        Piece { kind: 4, power: 2 },
        Piece::plain(7),
        Piece::plain(1),
        Piece::plain(5),
    ];
    let mut game = await_score(scores, 0);
    game.do_score();
    assert_eq!(game.goal_scores, [0, 1, 0]);
}

#[test]
fn activate_is_sent_once_when_every_goal_is_met() {
    let mut game = await_score(goal_pieces(), 0);
    let out = game.do_score();
    assert_eq!(names(&out), ["activate", "updatescore", "timerupdate"]);
    assert!(game.armed);
    game.phase = Phase::AwaitScore;
    game.current_scores = Vec::new();
    let again = game.do_score();
    assert_eq!(names(&again), ["updatescore", "timerupdate"]);
}

#[test]
fn doscore_is_one_shot_and_ignores_the_client_fields() {
    let mut game = await_score(vec![Piece::plain(4)], 0);
    let mut params = HashMap::new();
    params.insert("goalScores".to_string(), SfsValue::Number(999.0));
    let first = game.message("doscore", &params);
    assert_eq!(names(&first), ["updatescore", "timerupdate"]);
    assert_eq!(game.goal_scores[0], 1);
    let second = game.message("doscore", &params);
    assert!(second.is_empty());
    assert_eq!(game.goal_scores[0], 1);
}

#[test]
fn doscore_is_refused_outside_await_score() {
    let mut ready = AlignmentGame::with_board(&make_session(1), plain_board());
    assert!(ready.do_score().is_empty());
    ready.phase = Phase::AwaitCheck;
    assert!(ready.do_score().is_empty());
    assert_eq!(ready.goal_scores, [0, 0, 0]);
}

#[test]
fn dovictory_needs_the_activate_then_ends_the_game() {
    let mut game = await_score(goal_pieces(), 0);
    assert!(game.message("dovictory", &HashMap::new()).is_empty());
    assert_eq!(game.outcome, None);
    game.do_score();
    let out = game.message("dovictory", &HashMap::new());
    assert_eq!(names(&out), ["victory"]);
    assert!(out.iter().any(|o| matches!(o, GameOutput::Victory)));
    assert_eq!(game.outcome, Some("victory"));
    assert!(!game.needs_tick());
    assert!(game.message("cascade", &HashMap::new()).is_empty());
}

#[test]
fn transcript_a_hint_is_the_first_triangle_in_the_scan_order() {
    let mut game = AlignmentGame::with_board(&make_session(1), transcript_a_board());
    let out = game.call_hint();
    assert_eq!(names(&out), ["hint", "timerupdate"]);
    let hint = sends(&out)[0];
    assert_eq!(cell(&hint["hex1"]), (1.0, 1.0, 4.0));
    assert_eq!(cell(&hint["hex2"]), (0.0, 1.0, 6.0));
    assert_eq!(cell(&hint["hex3"]), (1.0, 2.0, 5.0));
}

#[test]
fn hint_index_rotates_through_the_four_orders() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    let seen: Vec<u8> = (0..5)
        .map(|_| {
            game.call_hint();
            game.hint_index
        })
        .collect();
    assert_eq!(seen, [1, 2, 3, 0, 1]);
}

#[test]
fn every_hint_scan_order_keeps_n1_in_bounds() {
    let game = AlignmentGame::with_board(&make_session(1), transcript_a_board());
    for index in 0..4 {
        if let Some([_, _, (x, y)]) = game.find_hint(index) {
            assert!(
                y < game.board[x].len(),
                "order {index} returned n1 off the board"
            );
        }
    }
}

#[test]
fn transcript_b_callhint_costs_five_seconds_and_finds_nothing() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    let out = game.call_hint();
    assert_eq!(names(&out), ["timerupdate"]);
    assert_eq!(sends(&out)[0]["timeRemaining"].as_f64(), Some(115.0));
    assert_eq!(sends(&out)[0]["timerState"].as_f64(), Some(1.0));
}

#[test]
fn idle_prompt_comes_at_tick_twenty_then_ten_seconds_later() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    let mut prompts = Vec::new();
    for tick in 1..=60 {
        if !game.tick().is_empty() {
            prompts.push(tick);
        }
    }
    assert_eq!(prompts, [20, 60]);
}

#[test]
fn idle_prompt_is_paused_by_a_swap_until_the_score() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    game.hint_pause = true;
    for _ in 0..40 {
        assert!(game.tick().is_empty());
    }
}

#[test]
fn tick_to_zero_gives_failure_then_nothing_is_processed() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    for _ in 0..479 {
        game.tick();
    }
    assert_eq!(game.outcome, None);
    let last = game.tick();
    assert_eq!(names(&last), ["failure"]);
    assert!(last.iter().any(|o| matches!(o, GameOutput::Failure)));
    assert_eq!(game.outcome, Some("failure"));
    assert!(!game.needs_tick());
    assert!(game.message("swappieces", &HashMap::new()).is_empty());
    assert!(game.tick().is_empty());
}

#[test]
fn await_cascade_still_runs_the_clock_to_a_failure() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    game.phase = Phase::AwaitCascade;
    game.hint_pause = true;
    let outputs: Vec<GameOutput> = (0..480).flat_map(|_| game.tick()).collect();
    assert!(outputs.iter().any(|o| matches!(o, GameOutput::Failure)));
    assert_eq!(game.outcome, Some("failure"));
}

#[test]
fn wall_clock_past_its_budget_ends_the_game_on_every_path() {
    let past = |game: &mut AlignmentGame| game.clock_budget = -1.0;
    let mut hint = AlignmentGame::with_board(&make_session(1), plain_board());
    past(&mut hint);
    let out = hint.message("callhint", &HashMap::new());
    assert!(matches!(out.last(), Some(GameOutput::Failure)));
    let mut victory = AlignmentGame::with_board(&make_session(1), plain_board());
    past(&mut victory);
    let out = victory.message("dovictory", &HashMap::new());
    assert!(matches!(out.last(), Some(GameOutput::Failure)));
    let mut swap = AlignmentGame::with_board(&make_session(1), plain_board());
    past(&mut swap);
    let out = swap.message("swappieces", &HashMap::new());
    assert!(matches!(out.last(), Some(GameOutput::Failure)));
    let mut ticking = AlignmentGame::with_board(&make_session(1), plain_board());
    past(&mut ticking);
    assert!(matches!(ticking.tick().last(), Some(GameOutput::Failure)));
    assert_eq!(ticking.outcome, Some("failure"));
}

#[test]
fn abort_ends_the_game_and_the_clock_stops() {
    let mut game = AlignmentGame::with_board(&make_session(1), plain_board());
    assert!(game.needs_tick());
    assert!(game.aborted().is_empty());
    assert!(!game.needs_tick());
    assert!(game.message("callhint", &HashMap::new()).is_empty());
    assert!(game.tick().is_empty());
}
