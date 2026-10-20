//! Tests for the ConverseBasicHumanoid plays, the NPC opponent and hand
//! scoring (MG-CH2): the dominance table, the NPC's choices, the refusals and
//! each hand-end branch.

use super::*;
use crate::minigame::game::GameOutput;
use crate::minigame::session::MinigameSession;

/// A logic3 game for seed 7 with its containers reset. Each test sets the
/// hands and contests it needs.
fn fresh() -> ConverseHumanoidGame {
    let mut game =
        ConverseHumanoidGame::new(&MinigameSession::offline("ConverseBasicHumanoid", 1, 7));
    game.started();
    game
}

/// A logic3 game in the middle of a hand, with the given totals and contests.
fn mid_hand(
    player_total: u32,
    npc_total: u32,
    player: Vec<f64>,
    npc: Vec<f64>,
) -> ConverseHumanoidGame {
    let mut game = fresh();
    game.player_total_plays = player_total;
    game.npc_total_plays = npc_total;
    game.player_contest_plays = player;
    game.npc_contest_plays = npc;
    game
}

/// A string field of an outgoing message.
fn text(output: &GameOutput, key: &str) -> String {
    match output {
        GameOutput::Send(map) => map[key].as_str().expect("string field").to_string(),
        other => panic!("not a message: {other:?}"),
    }
}

/// A numeric field of an outgoing message.
fn number(output: &GameOutput, key: &str) -> f64 {
    match output {
        GameOutput::Send(map) => map[key].as_f64().expect("number field"),
        other => panic!("not a message: {other:?}"),
    }
}

#[test]
fn dominance_table_matches_spec_for_all_81_pairs() {
    for player in 0..9_i32 {
        for npc in 0..9_i32 {
            let mut game = fresh();
            game.player_contest_plays = vec![f64::from(player)];
            game.npc_contest_plays = vec![f64::from(npc)];
            game.process_score();
            // Spec section 6: d = (p - n + 8) % 9; 8 is a tie, below 4 the NPC
            // wins, otherwise the player wins. logic3 wins a sweep, worth 2.
            let d = (player - npc + 8).rem_euclid(9);
            let expected = match d {
                8 => (0.0, 0.0),
                d if d < 4 => (0.0, 2.0),
                _ => (2.0, 0.0),
            };
            assert_eq!(
                (game.player_score, game.npc_score),
                expected,
                "player {player}, npc {npc}"
            );
        }
    }
}

#[test]
fn process_score_pairs_by_play_order_in_mediation5() {
    let mut game = fresh();
    game.game_type = 1; // mediation5, the second row of the table
    game.player_contest_plays = vec![8.0, 4.0];
    game.npc_contest_plays = vec![0.0, 8.0];
    game.process_score();
    // Play 0: 8 beats 0. Play 1: 4 beats 8 (d = 4). Two wins is a sweep: 3.
    assert_eq!((game.player_score, game.npc_score), (3.0, 0.0));
}

#[test]
fn npc_plays_index_zero_on_transcript_b_hands() {
    let mut game = fresh();
    game.npc_hand = vec![0.0, 4.0, 8.0];
    game.player_hand = vec![0.0, 4.0, 8.0];
    let out = game.npc_move();
    assert_eq!(out.len(), 1);
    assert_eq!(text(&out[0], "_cmd"), "updatecards");
    assert_eq!(text(&out[0], "user"), "npc");
    assert_eq!(number(&out[0], "num"), 0.0);
    assert_eq!(number(&out[0], "suit"), 0.0);
    assert_eq!(number(&out[0], "container"), 0.0);
    assert_eq!(game.npc_hand, vec![4.0, 8.0]);
}

#[test]
fn npc_answers_focus_with_the_card_that_beats_it() {
    let mut game = fresh();
    game.npc_hand = vec![0.0, 4.0, 8.0];
    game.npc_focus = Some((0, 8.0));
    let out = game.npc_move();
    // Weights against 8: 0 is -1, 4 is +1 (4 beats 8), 8 is +0.5.
    assert_eq!(number(&out[0], "num"), 1.0);
    assert_eq!(number(&out[0], "suit"), 4.0);
    assert_eq!(game.npc_focus, None);
}

#[test]
fn npc_ties_go_to_the_lowest_index() {
    let mut game = fresh();
    game.npc_hand = vec![4.0, 4.0, 0.0];
    game.npc_focus = Some((0, 8.0));
    let out = game.npc_move();
    // Indexes 0 and 1 both weigh +1 against 8; the strict argmax keeps 0.
    assert_eq!(number(&out[0], "num"), 0.0);
    assert_eq!(game.npc_hand, vec![4.0, 0.0]);
}

#[test]
fn npc_full_contest_gives_no_output() {
    let mut game = fresh();
    game.npc_contest_plays = vec![0.0, 0.0, 0.0];
    assert!(game.npc_move().is_empty());
    assert_eq!(game.npc_hand.len(), 3);
}

#[test]
fn player_move_echoes_num_and_removes_the_card_by_suit() {
    let mut game = fresh();
    game.player_hand = vec![0.0, 4.0, 8.0];
    // The client's num is stale; it goes back unchanged while the card is
    // found by its suit value.
    let out = game.player_move(2.0, 8.0, None);
    assert_eq!(out.len(), 1);
    assert_eq!(text(&out[0], "_cmd"), "updatecards");
    assert_eq!(text(&out[0], "user"), "player");
    assert_eq!(number(&out[0], "num"), 2.0);
    assert_eq!(number(&out[0], "suit"), 8.0);
    assert_eq!(number(&out[0], "container"), 0.0);
    assert_eq!(game.player_hand, vec![0.0, 4.0]);
    assert_eq!(game.player_contest_plays, vec![8.0]);
    assert_eq!(game.player_total_plays, 1);
}

#[test]
fn player_move_refuses_a_suit_not_in_hand_and_bad_containers() {
    let mut game = fresh();
    game.player_hand = vec![0.0, 4.0, 8.0];
    assert!(
        game.player_move(0.0, 7.0, None).is_empty(),
        "suit not in hand"
    );
    assert!(
        game.player_move(0.0, 0.0, Some(5.0)).is_empty(),
        "container out of range"
    );
    game.player_empty[0] = false;
    assert!(
        game.player_move(0.0, 0.0, Some(0.0)).is_empty(),
        "container already used"
    );
    assert_eq!(game.player_hand, vec![0.0, 4.0, 8.0]);
    assert_eq!(game.player_total_plays, 0);
}

#[test]
fn player_move_refuses_a_full_contest_or_a_finished_game() {
    let mut game = fresh();
    game.player_hand = vec![0.0, 4.0, 8.0];
    game.player_contest_plays = vec![0.0, 4.0, 8.0];
    assert!(game.player_move(0.0, 0.0, None).is_empty(), "contest full");
    game.player_contest_plays.clear();
    game.outcome = Some(Outcome::Victory);
    assert!(game.player_move(0.0, 0.0, None).is_empty(), "game over");
    assert_eq!(game.player_hand.len(), 3);
}

#[test]
fn player_play_into_an_unfilled_npc_container_sets_the_focus() {
    let mut game = fresh();
    game.player_hand = vec![0.0, 4.0, 8.0];
    game.player_move(0.0, 8.0, Some(0.0));
    assert_eq!(game.npc_focus, Some((0, 8.0)));
    assert_eq!(game.npc_timer, 0.5);
}

#[test]
fn player_play_into_a_filled_npc_container_sets_no_focus() {
    let mut game = fresh();
    game.player_hand = vec![0.0, 4.0, 8.0];
    game.npc_empty[0] = false;
    game.player_move(0.0, 8.0, Some(0.0));
    assert_eq!(game.npc_focus, None);
    assert_eq!(game.npc_timer, NPC_TIMER);
}

#[test]
fn fail_message_sends_failure_and_defeats() {
    let mut game = fresh();
    let out = game.fail_message();
    assert_eq!(text(&out[0], "_cmd"), "failure");
    assert!(matches!(out[1], GameOutput::Failure));
    assert_eq!(game.outcome, Some(Outcome::Defeat));
    assert_eq!(game.timer_state, 0);
}

#[test]
fn hand_end_deal_over_scores_then_sends_newdeal() {
    let mut game = mid_hand(3, 3, vec![8.0], vec![0.0]);
    let out = game.end_hand();
    assert_eq!(text(&out[0], "_cmd"), "updatescore");
    assert_eq!(text(&out[1], "_cmd"), "newdeal");
    // One container won in logic3 is a sweep: 2 points.
    assert_eq!(number(&out[0], "playerScore"), 2.0);
    assert!(game.player_contest_plays.is_empty());
}

#[test]
fn hand_end_player_never_played_fails() {
    let mut game = mid_hand(0, 1, vec![], vec![0.0]);
    let out = game.end_hand();
    assert_eq!(text(&out[0], "_cmd"), "failure");
    assert!(matches!(out[1], GameOutput::Failure));
}

#[test]
fn hand_end_npc_out_of_cards_and_player_idle_fails() {
    let mut game = mid_hand(1, 3, vec![], vec![]);
    let out = game.end_hand();
    assert_eq!(text(&out[0], "_cmd"), "failure");
    assert!(matches!(out[1], GameOutput::Failure));
}

#[test]
fn hand_end_npc_played_and_player_idle_fails() {
    let mut game = mid_hand(1, 1, vec![], vec![0.0]);
    let out = game.end_hand();
    assert_eq!(text(&out[0], "_cmd"), "failure");
    assert!(matches!(out[1], GameOutput::Failure));
}

#[test]
fn hand_end_otherwise_scores_resets_containers_and_sends_emptyhand() {
    let mut game = mid_hand(1, 1, vec![8.0], vec![0.0]);
    game.player_empty[0] = false;
    game.npc_focus = Some((0, 8.0));
    let out = game.end_hand();
    assert_eq!(text(&out[0], "_cmd"), "updatescore");
    assert_eq!(text(&out[1], "_cmd"), "emptyhand");
    assert_eq!(game.player_empty, vec![true]);
    assert!(game.npc_contest_plays.is_empty());
    assert_eq!(game.npc_focus, None);
    assert_eq!(game.outcome, None);
}
