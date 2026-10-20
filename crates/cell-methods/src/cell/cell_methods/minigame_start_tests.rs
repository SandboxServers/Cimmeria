//! The start-dialog answers through `dispatch` (MG-21): Play (cell 24) sends
//! the `StartMinigame` for the offer, Cancel (cell 30) drops it. The helper
//! tests call the launch functions directly, so only these reach the arms.

use tokio::sync::mpsc;

use super::{dispatch, START, START_CANCEL};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_cell_world::cell::minigame_launch::{start_or_offer, MinigameLaunchRequest};

fn space_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(100);
    e.character_name = Some("Minigamer".to_string());
    mgr
}

fn request() -> MinigameLaunchRequest {
    MinigameLaunchRequest {
        entity_id: 1,
        player_id: 100,
        game_name: "Livewire".into(),
        difficulty: 1,
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        source: "content_chain",
    }
}

/// Play after an offer: `dispatch(START)` sends the `StartMinigame` for that
/// offer and the call returns true.
/// Revert: the arm back to the UNIMPLEMENTED stub sends nothing and fails the
/// `StartMinigame` assert.
#[tokio::test]
async fn play_after_an_offer_sends_start_minigame() {
    let mut mgr = space_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    assert!(matches!(
        rx.try_recv(),
        Ok(CellToBaseMsg::EntityMethodCall { .. })
    ));

    assert!(dispatch(1, START, &[], &tx, &mut mgr).await);

    match rx.try_recv() {
        Ok(CellToBaseMsg::StartMinigame {
            entity_id,
            player_id,
            game_name,
            ..
        }) => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(game_name, "Livewire");
        }
        other => panic!("expected StartMinigame, got {other:?}"),
    }
}

/// Cancel after an offer: `dispatch(START_CANCEL)` drops the offer, so a
/// later Play has nothing to launch and sends no `StartMinigame`.
/// Revert: the stub leaves the offer in place, and the Play after it sends
/// the `StartMinigame`, failing the assert.
#[tokio::test]
async fn cancel_after_an_offer_drops_it() {
    let mut mgr = space_with_player();
    let (tx, mut rx) = mpsc::channel(8);
    assert!(start_or_offer(request(), true, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_ok());

    assert!(dispatch(1, START_CANCEL, &[], &tx, &mut mgr).await);
    assert!(mgr.minigame_launch.offers.is_empty());

    assert!(dispatch(1, START, &[], &tx, &mut mgr).await);
    assert!(!matches!(
        rx.try_recv(),
        Ok(CellToBaseMsg::StartMinigame { .. })
    ));
}
