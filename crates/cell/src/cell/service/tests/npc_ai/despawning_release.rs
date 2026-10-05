//! The AI `Despawning` state takes the NPC out of its attackers' combat.
//!
//! Bug shape: `npc_ai_despawn` called a bare `despawn_npc`, which removes the
//! entity only, so a player who had hit the NPC kept it in `threatened_mobs`
//! and `BSF_InCombat` stayed set (no regen) until relog. Revert proof: route
//! `npc_ai_despawn` back to `space_mgr.despawn_npc` and this test fails on
//! `threatened_mobs`.

use super::*;
use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const PLAYER: u32 = 101;

#[tokio::test]
async fn npc_ai_despawning_takes_its_attacker_out_of_combat() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.create_entity(PLAYER, "Castle", [10.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    mgr.connect_entity(PLAYER);
    let _ = generate_threat(&mut mgr, PLAYER, NPC, 10.0, AggroCause::Damage);
    assert!(
        mgr.get_entity(PLAYER).unwrap().state_field & BSF_IN_COMBAT != 0,
        "fixture: the hit put the player in combat"
    );
    crate::cell::service::npc_ai::force_ai_state(
        mgr.get_entity_mut(NPC).unwrap(),
        AiState::Despawning,
    );
    let (tx, mut rx) = mpsc::channel(256);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;

    assert!(
        mgr.get_entity(NPC).is_none(),
        "fixture: the AI despawned it"
    );
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(player.threatened_mobs.is_empty(), "the NPC is drained");
    assert_eq!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "the player left combat"
    );
    let told = std::iter::from_fn(|| rx.try_recv().ok()).any(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index: ON_STATE_FIELD_UPDATE,
                ..
            }
        )
    });
    assert!(told, "the player's client is told");
}
