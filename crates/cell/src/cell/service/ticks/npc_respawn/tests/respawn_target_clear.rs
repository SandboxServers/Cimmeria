//! #844: a respawn is a new life under the same entity id, so a player who
//! still has the corpse selected loses the selection — server-side
//! (`current_target_id`) and client-side (`onTargetUpdate(0)`, client
//! method 16, a single INT32).
//!
//! Fails without the `clear_targets_on` block in `npc_respawn_tick`: the
//! player keeps `Some(50)` and no `onTargetUpdate` is sent.

use super::super::*;
use super::fixtures::{drain, make_mgr_with_dead_npc};
use crate::mercury::method_idx;

#[tokio::test]
async fn a_respawn_clears_the_corpse_selection_and_tells_the_client() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    mgr.get_entity_mut(1).unwrap().current_target_id = Some(50);

    let (tx, mut rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().current_target_id,
        None,
        "the respawned NPC must not stay selected"
    );
    let target_updates: Vec<Vec<u8>> = drain(&mut rx)
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if method_index == method_idx::ON_TARGET_UPDATE => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(method_idx::ON_TARGET_UPDATE, 16);
    assert_eq!(
        target_updates,
        vec![vec![0u8, 0, 0, 0]],
        "exactly one onTargetUpdate(INT32 0) to the player"
    );
}

/// A player with some other target is left alone, and gets no update.
#[tokio::test]
async fn a_respawn_leaves_other_selections_alone() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    mgr.get_entity_mut(1).unwrap().current_target_id = Some(1);

    let (tx, mut rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    assert_eq!(mgr.get_entity(1).unwrap().current_target_id, Some(1));
    assert!(!drain(&mut rx).iter().any(|m| matches!(
        m,
        CellToBaseMsg::EntityMethodCall { entity_id: 1, method_index, .. }
            if *method_index == method_idx::ON_TARGET_UPDATE
    )));
}
