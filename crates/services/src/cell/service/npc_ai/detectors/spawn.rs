//! `spawner.npc_behaviour event=spawn_off_mesh`: a spawn the pathfinder
//! cannot start a route from (audit S9).
//!
//! The spawn log's `on_navmesh` uses `is_point_valid` (3.0 horizontal, 4.0
//! up), but `find_path` looks the start up with a `±0.5` box. A spawn
//! hovering more than half a unit over its floor passes the first and fails
//! every path it ever asks for — and the leash returns it to that same
//! point. Reported once per `spawn_id`, so respawns stay quiet.

use super::{MoveSource, NpcIdent};
use crate::cell::space_manager::SpaceManager;

/// Check a freshly spawned NPC against the `find_path` start box.
pub(in crate::cell) fn check_spawn(space_mgr: &mut SpaceManager, npc_id: u32) {
    space_mgr
        .npc_detectors
        .note_move_source(npc_id, MoveSource::Spawn);
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let pos = e.position;
    let spawn_id = e.spawn_id;
    let Some(space_id) = space_mgr.get_entity_space_id(npc_id) else {
        return;
    };
    let Some(navmesh) = space_mgr
        .spaces
        .get(&space_id)
        .and_then(|s| s.navmesh.as_ref())
    else {
        return;
    };
    if navmesh.start_poly_snap(&pos).is_some() {
        return;
    }
    let verdict = navmesh.diagnose_point(&pos);
    let snapped_y = navmesh.find_nearest_poly(&pos).map(|(_, p)| p.y);
    let navmesh_hash = navmesh.short_hash().to_owned();
    // Once per spawn id; an unseeded (content / console) spawn is keyed by
    // its entity id through the negative space so the two never collide.
    let key = spawn_id.unwrap_or(-(npc_id as i32));
    if !space_mgr.npc_detectors.spawn_warned.insert(key) {
        return;
    }
    let Some(ident) = NpcIdent::of(space_mgr, npc_id) else {
        return;
    };
    let gate = verdict.gate_label().unwrap_or("start_box");
    cimmeria_observability::counter!(
        "npc_spawn_off_mesh_total",
        "world" => ident.world.clone(),
        "gate" => gate,
    );
    tracing::warn!(
        target: "spawner.npc_behaviour",
        event = "spawn_off_mesh",
        npc_id,
        tag = %ident.tag,
        template_id = ident.template_id,
        world = %ident.world,
        space_id = ident.space_id,
        spawn_id,
        x = pos.x,
        y = pos.y,
        z = pos.z,
        // `is_point_valid` can still say yes: that is the S9 gap.
        on_navmesh = verdict.valid,
        gate,
        horizontal_dist = verdict.horizontal_dist,
        dy = verdict.dy,
        snapped_y,
        navmesh_hash = %navmesh_hash,
        "spawner: NPC spawned outside find_path's +-0.5 start box -- every path \
         it requests will fail with no_start_poly"
    );
}
