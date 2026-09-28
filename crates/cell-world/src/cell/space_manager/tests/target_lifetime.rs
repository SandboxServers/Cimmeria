//! #844: a stored target (`current_target_id`) is dropped when its target
//! leaves the holder's view or is destroyed — see
//! `space_manager::target_lifetime`.
//!
//! Each clearing test fails with its hook removed: the AoI one without the
//! transition check in `compute_player_aoi`, the destroy/despawn ones
//! without the `clear_targets_on` call in `destroy_entity`.

use cimmeria_entity::cell_entity::{AOI_LEAVE_MARGIN, PLAYER_AOI_RADIUS};

use super::super::SpaceManager;
use super::make_manager;

const PLAYER: u32 = 1;
const OTHER: u32 = 2;

fn player_at(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.create_entity(id, "Agnos", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let e = mgr.get_entity_mut(id).unwrap();
        e.account_id = Some(id);
        e.player_id = Some(id as i32);
    }
    mgr.connect_entity(id);
    mgr.get_entity_mut(id).unwrap().archetype_id = Some(4);
}

fn npc_at(mgr: &mut SpaceManager, x: f32) -> u32 {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, "Agnos", [x, 0.0, 0.0], [0.0; 3]).unwrap();
    id
}

fn target_of(mgr: &SpaceManager, id: u32) -> Option<i32> {
    mgr.get_entity(id).and_then(|e| e.current_target_id)
}

/// A player, an NPC 10 m away, one AoI tick, and the NPC selected.
fn selected_npc() -> (SpaceManager, u32) {
    let mut mgr = make_manager();
    player_at(&mut mgr, PLAYER, 0.0);
    let npc = npc_at(&mut mgr, 10.0);
    let _ = mgr.compute_aoi_changes();
    assert!(mgr.target_in_view(PLAYER, npc), "precondition: NPC in view");
    mgr.get_entity_mut(PLAYER).unwrap().current_target_id = Some(npc as i32);
    (mgr, npc)
}

#[test]
fn a_target_that_leaves_view_is_cleared() {
    let (mut mgr, npc) = selected_npc();

    let beyond = PLAYER_AOI_RADIUS + AOI_LEAVE_MARGIN + 10.0;
    mgr.update_position_preserving_facing(npc, [beyond, 0.0, 0.0], [0.0; 3]);
    let _ = mgr.compute_aoi_changes();

    assert!(
        !mgr.target_in_view(PLAYER, npc),
        "precondition: NPC left view"
    );
    assert_eq!(
        target_of(&mgr, PLAYER),
        None,
        "a target outside the witness set must not stay selected"
    );
}

/// A target that stays in view, and a self-target (never in the witness
/// set), both survive AoI ticks.
#[test]
fn a_target_in_view_and_a_self_target_survive_aoi_ticks() {
    let (mut mgr, npc) = selected_npc();
    let _ = mgr.compute_aoi_changes();
    assert_eq!(target_of(&mgr, PLAYER), Some(npc as i32));

    mgr.get_entity_mut(PLAYER).unwrap().current_target_id = Some(PLAYER as i32);
    let _ = mgr.compute_aoi_changes();
    assert_eq!(target_of(&mgr, PLAYER), Some(PLAYER as i32));
}

#[test]
fn a_destroyed_target_is_cleared_for_every_holder() {
    let (mut mgr, npc) = selected_npc();
    player_at(&mut mgr, OTHER, 5.0);
    let _ = mgr.compute_aoi_changes();
    mgr.get_entity_mut(OTHER).unwrap().current_target_id = Some(npc as i32);
    // A holder of some other target is left alone.
    let keep = npc_at(&mut mgr, 12.0);
    let third = 3;
    player_at(&mut mgr, third, 6.0);
    mgr.get_entity_mut(third).unwrap().current_target_id = Some(keep as i32);

    mgr.destroy_entity(npc);

    assert_eq!(target_of(&mgr, PLAYER), None);
    assert_eq!(target_of(&mgr, OTHER), None);
    assert_eq!(target_of(&mgr, third), Some(keep as i32));
}

/// The visible despawn path (GM `.despawn`, content despawns) scrubs
/// witness sets itself before it destroys, so the AoI diff never sees a
/// transition; the destroy hook is what clears the target.
#[tokio::test]
async fn a_despawned_target_is_cleared() {
    let (mut mgr, npc) = selected_npc();
    let (tx, _rx) = tokio::sync::mpsc::channel(16);

    let _ = mgr.despawn_npc(npc, &tx).await;
    let _ = mgr.compute_aoi_changes();

    assert_eq!(target_of(&mgr, PLAYER), None);
}

#[test]
fn target_in_view_is_self_or_a_witnessed_entity() {
    let (mgr, npc) = selected_npc();
    assert!(mgr.target_in_view(PLAYER, PLAYER));
    assert!(mgr.target_in_view(PLAYER, npc));
    assert!(!mgr.target_in_view(PLAYER, 999_999));
}
