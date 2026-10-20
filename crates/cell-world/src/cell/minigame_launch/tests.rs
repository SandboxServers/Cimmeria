use super::modifiers::{abilities_mask_from, ABILITY_BITS};
use super::*;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

fn knows_only(ids: &'static [i32]) -> impl Fn(i32) -> bool {
    move |id| ids.contains(&id)
}

#[test]
fn mask_sets_one_bit_per_granting_group() {
    assert_eq!(abilities_mask_from(knows_only(&[677, 685])), 0b1001);
    assert_eq!(abilities_mask_from(knows_only(&[])), 0);
    assert_eq!(
        abilities_mask_from(knows_only(&[778, 779, 792, 793])),
        0x78000
    );
}

#[test]
fn a_second_ability_on_the_same_bit_sets_that_bit_once() {
    assert_eq!(abilities_mask_from(knows_only(&[690])), 1);
    assert_eq!(abilities_mask_from(knows_only(&[677, 690, 698])), 1);
}

#[test]
fn bit_five_has_no_ability() {
    let all: Vec<i32> = ABILITY_BITS
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let mask = abilities_mask_from(|id| all.contains(&id));
    assert_eq!(mask & (1 << 5), 0);
}

fn mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(1).unwrap().is_player = true;
    mgr
}

/// The launch carries the player's known ability as bit 0 and sets the hold.
/// Revert: a zero `abilities_mask` in `launch_minigame` fails the assert.
#[tokio::test]
async fn launch_sends_the_players_ability_mask_and_takes_the_hold() {
    let mut mgr = mgr_with_player();
    mgr.get_entity_mut(1).unwrap().abilities.add_ability(677);
    let (tx, mut rx) = mpsc::channel(8);
    let req = MinigameLaunchRequest {
        entity_id: 1,
        player_id: 7,
        game_name: "Livewire".into(),
        difficulty: 3,
        on_victory_chains: vec![1001],
        on_failure_chains: vec![1002],
        source: "content_chain",
    };

    assert!(launch_minigame(req.clone(), &tx, &mut mgr).await);

    match rx.try_recv() {
        Ok(CellToBaseMsg::StartMinigame {
            entity_id,
            player_id,
            game_name,
            difficulty,
            on_victory_chains,
            on_failure_chains,
            tech_competency,
            abilities_mask,
            ..
        }) => {
            assert_eq!(entity_id, req.entity_id);
            assert_eq!(player_id, req.player_id);
            assert_eq!(game_name, req.game_name);
            assert_eq!(difficulty, req.difficulty);
            assert_eq!(on_victory_chains, req.on_victory_chains);
            assert_eq!(on_failure_chains, req.on_failure_chains);
            assert_eq!(tech_competency, MINIGAME_TECH_COMPETENCY);
            assert_eq!(abilities_mask, 1);
        }
        other => panic!("expected StartMinigame, got {other:?}"),
    }
    assert_ne!(
        mgr.get_entity(1).unwrap().state_field & BSF_PLAYING_MINIGAME,
        0
    );
}

/// A player who is gone gets a zero mask and no hold: the send still goes.
#[tokio::test]
async fn launch_for_a_missing_entity_sends_a_zero_mask() {
    let mut mgr = mgr_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    let req = MinigameLaunchRequest {
        entity_id: 99,
        player_id: 7,
        game_name: "Livewire".into(),
        difficulty: 1,
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        source: "effect_script",
    };

    assert!(launch_minigame(req, &tx, &mut mgr).await);
    match rx.try_recv() {
        Ok(CellToBaseMsg::StartMinigame {
            abilities_mask,
            player_level,
            intelligence,
            ..
        }) => {
            assert_eq!(abilities_mask, 0);
            assert_eq!((player_level, intelligence), (1, 0));
        }
        other => panic!("expected StartMinigame, got {other:?}"),
    }
}
