//! Populating world spaces with NPCs from DB spawn records.
//!
//! `spawn_npcs_from_records` fills the startup spaces at boot; instanced
//! spaces go through `spawn_instance_npcs_from_records`, which spawns into
//! the space just created rather than recreating it. The records come from
//! the spawner's DB loader (`cell::spawner::load_spawns_from_db`).
//!
//! Moved here from `spawner/npcs.rs` so the DB loaders do not depend on
//! `SpaceManager` (docs/architecture/services-crate-split.md §2B). The
//! events keep landing in `spawner.log`: the file layer names this module.

use super::super::spawner::SpawnRecord;
use super::SpaceManager;

/// Spawn NPCs from DB records into all currently-loaded startup spaces.
///
/// Only spawns records whose `world_name` matches a space that already exists
/// in the SpaceManager (i.e., non-instanced startup spaces). Instanced spaces
/// are handled by `spawn_instance_npcs_from_records`.
#[tracing::instrument(
    name = "spawner.spawn_startup",
    level = "info",
    skip_all,
    fields(record_count = records.len(), spawned = tracing::field::Empty),
)]
pub fn spawn_npcs_from_records(records: &[SpawnRecord], space_mgr: &mut SpaceManager) -> usize {
    let mut count = 0;
    for record in records {
        // Only spawn in spaces that already exist (startup/non-instanced spaces)
        if !space_mgr.has_space_for_world(&record.world_name) {
            continue;
        }

        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record(npc_id, record) {
            Ok(space_id) => {
                tracing::debug!(
                    npc_id, space_id, spawn_id = record.spawn_id,
                    world = %record.world_name, name = %record.template_name,
                    class = %record.class, tag = ?record.tag,
                    "Spawned NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, world = %record.world_name,
                    name = %record.template_name, "Failed to spawn NPC from DB: {e}"
                );
            }
        }
    }
    tracing::Span::current().record("spawned", count);
    tracing::info!(count, "DB-driven NPC population spawned (startup spaces)");
    count
}

/// Spawn NPCs from DB records for a specific instanced world into a given space.
///
/// Called when a new instanced space is created for a player (e.g., Castle_CellBlock,
/// SGC_W1). Each instance gets its own set of NPCs. The `space_id` parameter is the
/// space that was just created — NPCs are spawned directly into it rather than going
/// through `find_or_create_space` (which would create yet another new instance).
#[tracing::instrument(
    name = "spawner.spawn_instance",
    level = "info",
    skip_all,
    fields(world_name, space_id, record_count = records.len(), spawned = tracing::field::Empty),
)]
pub fn spawn_instance_npcs_from_records(
    records: &[SpawnRecord],
    world_name: &str,
    space_id: u32,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut count = 0;
    for record in records {
        if record.world_name != world_name {
            continue;
        }
        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record_in_space(npc_id, record, space_id) {
            Ok(sid) => {
                tracing::debug!(
                    npc_id, space_id = sid, spawn_id = record.spawn_id,
                    world = %record.world_name, name = %record.template_name,
                    tag = ?record.tag, "Spawned instance NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, name = %record.template_name,
                    "Failed to spawn instance NPC: {e}"
                );
            }
        }
    }
    tracing::Span::current().record("spawned", count);
    // `cover.coverage` for the new instance, now that its NPCs exist.
    space_mgr.log_cover_coverage(space_id);
    count
}

/// The resolved behaviour of a freshly spawned NPC, so "why does this NPC act
/// like that" is one query instead of a seed read: an `aggression` other than
/// 1 (HOSTILE) means it will never notice a player on its own; `use_cover = false` means the
/// loaded cover nodes are irrelevant to it; `respawn_secs = None` is one-shot.
fn log_spawn_behaviour(space_mgr: &mut SpaceManager, npc_id: u32) {
    // NA02: a spawn that passes `is_point_valid` below but fails
    // `find_path`'s tight start box (audit S9) — WARN, once per spawn id.
    crate::cell::service::npc_ai::detectors::spawn::check_spawn(space_mgr, npc_id);
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    tracing::debug!(
        target: "spawner.npc_behaviour",
        npc_id,
        npc_name = e.npc_name.as_deref().unwrap_or(""),
        tag = e.tag.as_deref().unwrap_or(""),
        template_id = e.template_id.unwrap_or(0),
        spawn_id = e.spawn_id.unwrap_or(0),
        x = e.position.x,
        y = e.position.y,
        z = e.position.z,
        spawn_yaw_rad = e.direction.y,
        on_navmesh = space_mgr.is_position_valid(npc_id, &e.position),
        navmesh_loaded = space_mgr.space_has_navmesh(npc_id),
        ground_y = ?space_mgr.get_navmesh_height(npc_id, e.position.x, e.position.y, e.position.z),
        level = e.level,
        faction = e.faction,
        // Effective level toward players (1 = hostile, NA13), whether it is
        // an override, and the radius the Idle scan uses.
        aggression = crate::cell::combat::aggression_toward_players(e).level(),
        aggression_override = ?e.aggro.override_level.map(|l| l.level()),
        aggro_radius = crate::cell::combat::aggro_radius(e),
        assist_radius = crate::cell::combat::assist_radius(e),
        use_cover = e.use_cover,
        is_stationary = e.is_stationary,
        move_speed = e.move_speed,
        respawn_secs = ?e.respawn_secs,
        follow_min_distance = e.follow_min_distance,
        follow_max_distance = e.follow_max_distance,
        patrol_len = e.patrol_path.len(),
        wander_radius = e.wander_radius,
        interaction_flags = e.interaction_type_flags,
        loot_table_id = ?e.loot_table_id,
        "NPC spawned -- resolved behaviour"
    );
}
