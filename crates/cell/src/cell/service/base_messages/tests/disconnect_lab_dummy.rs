//! AB-L2 (D-AU6): a GM's lab dummies go with the GM's logout; another GM's
//! stay.

use super::*;

use std::time::{Duration, Instant};

use cimmeria_cell_world::cell::space_manager::LabDummy;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::{MobAggression, PlayerIdentity};
use tokio::sync::mpsc;

fn mark(mgr: &mut SpaceManager, npc: u32, owner: u32) {
    mgr.get_entity_mut(npc)
        .unwrap()
        .extensions
        .insert(LabDummy {
            owner_id: owner,
            owner_identity: PlayerIdentity::UNKNOWN,
            disposition: MobAggression::Hostile,
            expires_at: Instant::now() + Duration::from_secs(600),
        });
}

/// Revert proof: drop the `despawn_lab_dummies_of` call from
/// `handle_disconnect_entity` and the owner's dummy is still standing.
#[tokio::test]
async fn ab_l2_logout_despawns_the_gms_own_dummies() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for gm in [1, 2] {
        mgr.create_entity(gm, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(gm).unwrap().is_player = true;
        mgr.connect_entity(gm);
    }
    let mine = mgr.allocate_npc_id();
    mgr.spawn_npc(mine, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mark(&mut mgr, mine, 1);
    let theirs = mgr.allocate_npc_id();
    mgr.spawn_npc(theirs, "Castle", [4.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mark(&mut mgr, theirs, 2);

    let (tx, _rx) = mpsc::channel(64);
    handle_base_message(
        disconnect_entity_msg(1),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    assert!(
        mgr.get_entity(mine).is_none(),
        "the GM's dummy left with them"
    );
    assert!(mgr.get_entity(theirs).is_some(), "another GM's dummy stays");
}
