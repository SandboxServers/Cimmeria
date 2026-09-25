//! Regression guards for logout position persistence.
//!
//! Bug shape: `sgw_player.pos_*` / `world_location` were written by gate
//! travel and the GM teleport only. A character that walked across Castle
//! and logged out came back at the last gate arrival (or, for a Cellblock
//! character, at the start cell of a fresh instance with every door shut
//! while their missions were already past those doors). Logout must hand
//! the cell's live position to the base for persistence, and it must do so
//! BEFORE `disconnect_entity` tears the entity down — afterwards there is
//! no position left to read.

use super::*;

use crate::cell::messages::BaseToCellMsg;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

fn persist_messages(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(i32, String, [f32; 3])> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| match m {
            CellToBaseMsg::PersistPosition {
                player_id,
                world_name,
                position,
            } => Some((player_id, world_name, position)),
            _ => None,
        })
        .collect()
}

/// Revert-verifier: removing the `persist_last_position` call from the
/// `DisconnectEntity` arm (or moving it after `space_mgr.disconnect_entity`)
/// leaves the channel with no `PersistPosition` at all, because the entity
/// is gone before anything can read its position.
#[tokio::test]
async fn disconnect_entity_persists_the_live_position_before_teardown() {
    let mut mgr = make_mgr();
    mgr.create_entity(
        1,
        "Castle_CellBlock",
        [-334.231, 73.472, -228.026],
        [0.0; 3],
    )
    .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }
    mgr.connect_entity(1);
    // The player walked away from the creation point during the session.
    mgr.update_entity_position(1, [-198.421, 48.374, -112.592], [0, 0, 0], [0.0; 3]);

    let (tx, mut rx) = mpsc::channel(32);
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    assert!(
        mgr.get_entity(1).is_none(),
        "fixture precondition: disconnect must have destroyed the entity"
    );
    assert_eq!(
        persist_messages(&mut rx),
        vec![(
            100,
            "Castle_CellBlock".to_string(),
            [-198.421, 48.374, -112.592]
        )],
        "exactly one PersistPosition, carrying the persistent player_id, the entity's \
         world and its LIVE position (not the creation point) — otherwise the next login \
         resumes at the last gate arrival instead of where the player logged out"
    );
}

/// NPCs and half-initialised sessions have no `sgw_player` row to update.
/// The arm must stay silent for them rather than queue a write that the
/// base would reject with a "no rows updated" warn on every NPC despawn.
#[tokio::test]
async fn disconnect_of_a_non_player_entity_persists_nothing() {
    let mut mgr = make_mgr();
    mgr.create_entity(7, "Castle_CellBlock", [1.0, 2.0, 3.0], [0.0; 3])
        .unwrap();
    // No is_player, no player_id — the shape of an NPC.

    let (tx, mut rx) = mpsc::channel(32);
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 7 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    assert!(
        persist_messages(&mut rx).is_empty(),
        "a non-player entity must not queue a PersistPosition"
    );
}
