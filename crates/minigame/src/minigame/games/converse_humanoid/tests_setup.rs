//! Tests for the ConverseBasicHumanoid setup (MG-CH1): the game-type table,
//! the seeded deal, the accord threshold, the Trump bonus and `fullgamestate`.

use std::collections::HashMap;

use super::setup::{next_game_type, trump_bonus, FIRST_GAME_TYPE, GAME_TYPES, PLAYER_VICTORIES};
use super::*;
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

/// A dealt game for `seed`, with the player's stats at zero.
fn dealt(seed: u32) -> ConverseHumanoidGame {
    let mut game =
        ConverseHumanoidGame::new(&MinigameSession::offline("ConverseBasicHumanoid", 1, seed));
    game.started();
    game
}

/// The numbers under `key`, which is an SFS array keyed `"0"`, `"1"`, ...
fn list(map: &HashMap<String, SfsValue>, key: &str) -> Vec<f64> {
    let SfsValue::Object(items) = &map[key] else {
        panic!("{key} is not an array");
    };
    (0..items.len())
        .map(|i| {
            items[&i.to_string()]
                .as_f64()
                .expect("array item is a number")
        })
        .collect()
}

#[test]
fn game_type_table_matches_spec() {
    let logic3 = &GAME_TYPES[0];
    assert_eq!(
        (
            logic3.name,
            logic3.card_total,
            logic3.container_total,
            logic3.contest_total
        ),
        ("logic3", 3, 1, 3)
    );
    assert_eq!(logic3.suits, [0, 4, 8]);

    let mediation5 = &GAME_TYPES[1];
    assert_eq!(
        (
            mediation5.name,
            mediation5.card_total,
            mediation5.container_total,
            mediation5.contest_total
        ),
        ("mediation5", 4, 2, 2)
    );
    assert_eq!(mediation5.suits, [0, 3, 4, 5, 8]);

    let courtesy7 = &GAME_TYPES[2];
    assert_eq!((courtesy7.name, courtesy7.card_total), ("courtesy7", 6));
    assert_eq!(courtesy7.suits, [0, 2, 3, 4, 5, 7, 8]);

    let humor9 = &GAME_TYPES[3];
    assert_eq!(
        (humor9.name, humor9.card_total, humor9.container_total),
        ("humor9", 9, 3)
    );
    assert_eq!(humor9.suits, [0, 1, 2, 3, 4, 5, 6, 7, 8]);

    for row in &GAME_TYPES {
        assert_eq!(row.card_total, row.container_total * row.contest_total);
    }
}

#[test]
fn logic3_first_then_mediation5() {
    assert_eq!(FIRST_GAME_TYPE, 0);
    assert_eq!(next_game_type(FIRST_GAME_TYPE), 1);
    assert_eq!(next_game_type(1), 1, "mediation5 stays mediation5");
}

#[test]
fn logic3_deals_three_a_side_and_mediation5_four() {
    let mut game = dealt(7);
    assert_eq!(game.npc_hand.len(), 3);
    assert_eq!(game.player_hand.len(), 3);
    assert_eq!(game.npc_empty.len(), 1, "logic3 has one container");

    game.game_type = next_game_type(game.game_type);
    game.deal();
    assert_eq!(game.npc_hand.len(), 4);
    assert_eq!(game.player_hand.len(), 4);
    assert_eq!(
        game.npc_empty,
        [true, true],
        "mediation5 has two containers"
    );
    assert!(game.npc_contest_plays.is_empty() && game.player_total_plays == 0);
}

#[test]
fn no_card_is_dealt_beyond_its_two_copies() {
    for seed in 0..200 {
        for game_type in [0, 1] {
            let mut game = dealt(seed);
            game.game_type = game_type;
            game.deal();
            let mut copies = [0usize; 9];
            for card in game.npc_hand.iter().chain(&game.player_hand) {
                if card.fract() == 0.0 {
                    copies[*card as usize] += 1;
                }
            }
            assert!(
                copies.iter().all(|&n| n <= 2),
                "seed {seed} type {game_type} dealt a suit more than twice: {copies:?}"
            );
        }
    }
}

#[test]
fn accord_threshold_at_the_boundary() {
    // logic3 (three cards): 33 percent gives 0.99 and 34 gives 1.02.
    let mut game = dealt(7);
    game.tech_competency = 0;
    game.player_level = 28;
    assert_eq!(game.get_accords(), (false, false));
    game.player_level = 29;
    assert_eq!(game.get_accords(), (true, true));

    // Intelligence is the player's alone.
    game.player_level = 0;
    game.intelligence = 28;
    assert_eq!(game.get_accords(), (false, false));
    game.intelligence = 29;
    assert_eq!(game.get_accords(), (false, true));

    // mediation5 (four cards): 24 percent gives 0.96 and 25 gives exactly 1.0.
    game.game_type = 1;
    game.intelligence = 0;
    game.player_level = 19;
    assert_eq!(game.get_accords(), (false, false));
    game.player_level = 20;
    assert_eq!(game.get_accords(), (true, true));
}

#[test]
fn trump_bits_add_their_bonus() {
    assert_eq!(trump_bonus(0), 0.0);
    assert_eq!(trump_bonus(1 << 15), 5.0, "778 Mediator");
    assert_eq!(trump_bonus(1 << 16), 10.0, "779 Negotiator");
    assert_eq!(trump_bonus(1 << 17), 15.0, "792 Diplomat");
    assert_eq!(trump_bonus(1 << 18), 25.0, "793 Ambassador");
    assert_eq!(trump_bonus(0b1111 << 15), 55.0, "the four bonuses add up");
    assert_eq!(trump_bonus(0x7FFF), 0.0, "cheat bits 0 to 14 are not Trump");

    // Bits 15 to 18 reach the player's accord: 5 + 55 = 60 percent of three cards.
    let mut game = dealt(7);
    game.tech_competency = 0;
    game.player_level = 0;
    game.abilities_mask = 0b1111 << 15;
    assert_eq!(game.get_accords(), (false, true));
}

#[test]
fn fixed_seed_gives_fixed_deal() {
    let first = dealt(7);
    let again = dealt(7);
    assert_eq!(first.npc_hand, again.npc_hand);
    assert_eq!(first.player_hand, again.player_hand);

    let hands: std::collections::HashSet<String> = (0..50)
        .map(|seed| format!("{:?}", dealt(seed).npc_hand))
        .collect();
    assert!(
        hands.len() > 1,
        "different seeds should not all deal one hand"
    );
}

#[test]
fn hands_are_sorted_numerically() {
    for seed in 0..50 {
        let game = dealt(seed);
        assert!(game.npc_hand.windows(2).all(|w| w[0] <= w[1]));
        assert!(game.player_hand.windows(2).all(|w| w[0] <= w[1]));
    }
}

#[test]
fn started_sends_one_fullgamestate() {
    let outputs =
        ConverseHumanoidGame::new(&MinigameSession::offline("ConverseBasicHumanoid", 1, 7))
            .started();
    let [GameOutput::Send(map)] = outputs.as_slice() else {
        panic!("started() sends exactly one message");
    };

    assert!(matches!(&map["_cmd"], SfsValue::String(s) if s == "fullgamestate"));
    assert_eq!(map["timeRemaining"].as_f64(), Some(30.0), "30.25 less 0.25");
    assert_eq!(map["timerState"].as_f64(), Some(1.0));
    assert!(matches!(map["isSpectator"], SfsValue::Bool(false)));
    assert!(matches!(&map["npcType"], SfsValue::String(s) if s == "humanoid"));
    assert!(matches!(&map["gameType"], SfsValue::String(s) if s == "logic3"));
    assert_eq!(map["cardTotal"].as_f64(), Some(3.0));
    assert_eq!(map["containerTotal"].as_f64(), Some(1.0));
    assert_eq!(list(map, "npcCardsArray").len(), 3);
    assert_eq!(list(map, "playerCardsArray").len(), 3);
    assert_eq!(map["npcScore"].as_f64(), Some(0.0));
    assert_eq!(map["numResets"].as_f64(), Some(0.0));
    assert!(!map.contains_key("suitTypes"), "suitTypes is never set");
    assert_eq!(map.len(), 18, "the fullgamestate keys of spec section 5");

    let SfsValue::Object(victories) = &map["playerVictoryConditions"] else {
        panic!("playerVictoryConditions is an array");
    };
    let names: Vec<&str> = (0..PLAYER_VICTORIES.len())
        .map(|i| {
            victories[&i.to_string()]
                .as_str()
                .expect("name is a string")
        })
        .collect();
    assert_eq!(names, ["focusdamage1", "focusdamage2"]);
}
