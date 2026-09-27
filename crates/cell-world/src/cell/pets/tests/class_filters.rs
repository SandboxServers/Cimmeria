//! The `queries.rs` class filters per A-22: the AI and movement ticks see a
//! pet; player AoE, cone, respawn (all through `all_npc_entity_ids`) and the
//! NA14 assist fan-out (`npc_ids_in_space_of`) do not.

use super::*;

#[test]
fn ai_tick_query_admits_the_pet() {
    let (mgr, pet) = world_with_pet();
    assert!(
        mgr.ai_driven_npc_entity_ids().contains(&pet),
        "a pet the AI tick never visits can never follow or fight"
    );
}

#[test]
fn aoe_cone_and_respawn_query_excludes_the_pet() {
    let (mut mgr, pet) = world_with_pet();
    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(mob, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let ids = mgr.all_npc_entity_ids();
    assert!(ids.contains(&mob), "control: the mob is listed");
    assert!(
        !ids.contains(&pet),
        "player AoE/cone would hit the owner's pet and the respawn tick would revive it"
    );
}

#[test]
fn assist_query_excludes_the_pet() {
    let (mut mgr, pet) = world_with_pet();
    let mob = mgr.allocate_npc_id();
    mgr.spawn_npc(mob, "Agnos", [10.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    let helper = mgr.allocate_npc_id();
    mgr.spawn_npc(helper, "Agnos", [10.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    let candidates = mgr.npc_ids_in_space_of(mob);
    assert!(candidates.contains(&helper), "control: a mob can assist");
    assert!(
        !candidates.contains(&pet),
        "a hostile mob must not recruit a player's pet"
    );
}
