//! Cell-side lifetime and ordering guards for the internal letter token.

use super::*;
use cimmeria_wire::cell::messages::{
    LetterTurnInEntryEpoch, LetterTurnInSession, LetterTurnInToken,
};
use std::time::{Duration, Instant};

fn manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn session(player_id: i32, generation: Instant, byte: u8) -> LetterTurnInSession {
    LetterTurnInSession {
        player_id,
        token: LetterTurnInToken::from_bytes([byte; 16]),
        generation,
        epoch: LetterTurnInEntryEpoch::new(1),
    }
}

async fn deliver(mgr: &mut SpaceManager, entity_id: u32, session: LetterTurnInSession) {
    let (tx, _rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::LetterTurnInSession { entity_id, session },
        &tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
}

fn stored(mgr: &SpaceManager, entity_id: u32) -> Option<LetterTurnInSession> {
    mgr.get_entity(entity_id)
        .and_then(|e| e.extensions.get::<LetterTurnInSession>().copied())
}

#[tokio::test]
async fn letter_turn_in_session_requires_matching_live_player() {
    let mut mgr = manager();
    mgr.create_entity(1, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let now = Instant::now();
    deliver(&mut mgr, 1, session(42, now, 1)).await;
    assert!(
        stored(&mgr, 1).is_none(),
        "unconnected entity must refuse token"
    );
    mgr.get_entity_mut(1).unwrap().is_player = true;
    mgr.get_entity_mut(1).unwrap().player_id = Some(42);
    mgr.get_entity_mut(2).unwrap().is_player = true;
    mgr.get_entity_mut(2).unwrap().player_id = Some(42);
    deliver(&mut mgr, 1, session(43, now, 1)).await;
    deliver(&mut mgr, 2, session(42, now, 1)).await;
    assert!(stored(&mgr, 1).is_none());
    assert!(stored(&mgr, 2).is_none());
    deliver(&mut mgr, 1, session(42, now, 1)).await;
    assert_eq!(stored(&mgr, 1).unwrap().token.bytes(), [1; 16]);
}

#[tokio::test]
async fn letter_turn_in_session_rejects_older_or_equal_conflict() {
    let mut mgr = manager();
    mgr.create_entity(3, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = mgr.get_entity_mut(3).unwrap();
    entity.is_player = true;
    entity.player_id = Some(42);
    let old = Instant::now();
    let new = old + Duration::from_nanos(1);
    deliver(&mut mgr, 3, session(42, new, 2)).await;
    deliver(&mut mgr, 3, session(42, old, 1)).await;
    deliver(&mut mgr, 3, session(42, new, 3)).await;
    assert_eq!(stored(&mgr, 3).unwrap().token.bytes(), [2; 16]);
    deliver(&mut mgr, 3, session(42, new, 2)).await;
    assert_eq!(stored(&mgr, 3).unwrap().token.bytes(), [2; 16]);
    deliver(&mut mgr, 3, session(42, new + Duration::from_nanos(1), 4)).await;
    assert_eq!(stored(&mgr, 3).unwrap().token.bytes(), [4; 16]);

    // Same login/key generation after returning to the same world.
    let mut reentry = session(42, new + Duration::from_nanos(1), 4);
    reentry.epoch = LetterTurnInEntryEpoch::new(2);
    deliver(&mut mgr, 3, reentry).await;
    deliver(&mut mgr, 3, session(42, new + Duration::from_nanos(1), 4)).await;
    assert_eq!(stored(&mgr, 3).unwrap().epoch.value(), 2);
}

#[tokio::test]
async fn letter_turn_in_session_dies_on_destroy_disconnect_and_reuse() {
    let mut mgr = manager();
    mgr.create_entity(4, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(4).unwrap().is_player = true;
    mgr.get_entity_mut(4).unwrap().player_id = Some(42);
    let now = Instant::now();
    deliver(&mut mgr, 4, session(42, now, 1)).await;
    let (tx, _rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 4 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    assert!(mgr.get_entity(4).is_none());
    mgr.create_entity(4, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(4).unwrap().is_player = true;
    mgr.get_entity_mut(4).unwrap().player_id = Some(42);
    assert!(stored(&mgr, 4).is_none());
    let mut new_entry = session(42, now, 2);
    new_entry.epoch = LetterTurnInEntryEpoch::new(2);
    deliver(&mut mgr, 4, new_entry).await;
    deliver(&mut mgr, 4, session(42, now, 1)).await;
    assert_eq!(stored(&mgr, 4).unwrap().token.bytes(), [2; 16]);
    handle_base_message(
        disconnect_entity_msg(4),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    assert!(mgr.get_entity(4).is_none());
}
