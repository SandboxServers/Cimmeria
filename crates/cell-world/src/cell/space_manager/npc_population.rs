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

use std::collections::BTreeMap;

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
    // Records whose world is absent from spaces.xml, counted per world name and
    // reported once per world below (a bad world name is one WARN, not one per row).
    let mut unknown_worlds: BTreeMap<&str, usize> = BTreeMap::new();
    // Records for a known instanced world: skipped by design, so no WARN.
    let mut instanced_skipped = 0;
    for record in records {
        // Only spawn in spaces that already exist (startup/non-instanced spaces)
        if !space_mgr.has_space_for_world(&record.world_name) {
            if space_mgr.worlds.contains_key(&record.world_name) {
                instanced_skipped += 1;
            } else {
                *unknown_worlds
                    .entry(record.world_name.as_str())
                    .or_insert(0) += 1;
            }
            continue;
        }

        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record(npc_id, record) {
            Ok(space_id) => {
                tracing::debug!(
                    npc_id,
                    npc_name = space_mgr.entity_label(npc_id),
                    space_id,
                    spawn_id = record.spawn_id, // nt:id-only spawnlist row id, the row has no name column
                    world = %record.world_name,
                    template_id = record.template_id,
                    template_name = %record.template_name,
                    class = %record.class, tag = ?record.tag,
                    "Spawned NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, // nt:id-only spawnlist row id, the row has no name column
                    world = %record.world_name,
                    template_id = record.template_id,
                    template_name = %record.template_name,
                    "Failed to spawn NPC from DB: {e}"
                );
            }
        }
    }
    for (world, records) in unknown_worlds {
        tracing::warn!(
            target: "spawner",
            event = "spawn_world_unknown",
            world,
            records,
            "DB NPC records name a world not in spaces.xml; skipped"
        );
    }
    if instanced_skipped > 0 {
        tracing::debug!(
            target: "spawner",
            event = "spawn_instanced_skipped",
            records = instanced_skipped,
            "DB NPC records for instanced worlds skipped; spawned per instance"
        );
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
    fields(world_name, space_id, record_count = tracing::field::Empty, spawned = tracing::field::Empty),
)]
pub fn spawn_instance_npcs_from_records(
    records: &[SpawnRecord],
    world_name: &str,
    space_id: u32,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut count = 0;
    let mut failed = 0;
    // Only this instance's world counts: the list holds every world's rows.
    let record_count = records
        .iter()
        .filter(|r| r.world_name == world_name)
        .count();
    tracing::Span::current().record("record_count", record_count);
    for record in records {
        if record.world_name != world_name {
            continue;
        }
        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record_in_space(npc_id, record, space_id) {
            Ok(sid) => {
                tracing::debug!(
                    npc_id,
                    npc_name = space_mgr.entity_label(npc_id),
                    space_id = sid,
                    spawn_id = record.spawn_id, // nt:id-only spawnlist row id, the row has no name column
                    world = %record.world_name,
                    template_id = record.template_id,
                    template_name = %record.template_name,
                    tag = ?record.tag, "Spawned instance NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, // nt:id-only spawnlist row id, the row has no name column
                    world = %record.world_name,
                    template_id = record.template_id,
                    template_name = %record.template_name,
                    "Failed to spawn instance NPC: {e}"
                );
                failed += 1;
            }
        }
    }
    tracing::info!(
        target: "spawner",
        event = "instance_populated",
        world = world_name,
        space_id,
        record_count,
        spawned = count,
        failed,
        "instance NPC population summary"
    );
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
    // NA44: which world and instance, what it fights with, and whether each
    // ability can animate (`event_set_ids` 0 = no fire animation).
    let ability_ids = super::npc_identity::sorted_ability_ids(e);
    let event_set_ids = super::npc_identity::ability_event_set_ids(space_mgr, &ability_ids);
    // Optional fields go in bare: a missing value leaves the key out, never `""` or `0`.
    let world = space_mgr.get_entity_world_name(npc_id);
    let names = super::EntityNames::of(e);
    tracing::debug!(
        target: "spawner.npc_behaviour",
        npc_id,
        world = world.as_deref(),
        space_id = space_mgr.get_entity_space_id(npc_id),
        npc_name = names.entity_name,
        tag = e.tag.as_deref(),
        template_id = e.template_id,
        template_name = names.template_name,
        spawn_id = e.spawn_id, // nt:id-only spawnlist row id, the row has no name column
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
        aggression_override = e.aggro.override_level.map(|l| l.level()),
        aggro_radius = crate::cell::combat::aggro_radius(e),
        assist_radius = crate::cell::combat::assist_radius(e),
        use_cover = e.use_cover,
        is_stationary = e.is_stationary,
        move_speed = e.move_speed,
        respawn_secs = e.respawn_secs,
        follow_min_distance = e.follow_min_distance,
        follow_max_distance = e.follow_max_distance,
        patrol_len = e.patrol_path.len(),
        wander_radius = e.wander_radius,
        interaction_flags = e.interaction_type_flags,
        interaction_flags_names = %cimmeria_entity::interaction_flags::INTERACTION_FLAGS.render(e.interaction_type_flags),
        loot_table_id = ?e.loot_table_id, // nt:id-only the NameBook has no loot_tables lookup
        ability_ids = ?ability_ids,
        event_set_ids = ?event_set_ids,
        weapon_visual = %super::npc_identity::weapon_visual(e),
        "NPC spawned -- resolved behaviour"
    );
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::log_spawn_behaviour;
    use crate::cell::space_manager::SpaceManager;
    use crate::test_support::LogCapture;

    /// A manager with one startup space, `Agnos`, and a bare NPC (no template)
    /// at its origin.
    fn agnos_with_npc(npc_id: u32) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
        )
        .unwrap();
        mgr.spawn_npc(npc_id, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr
    }

    /// Revert proof: with `respawn_secs = ?e.respawn_secs` the row carries
    /// the string `"Some(60)"`, so this assertion fails.
    #[test]
    fn npc_behaviour_row_logs_respawn_secs_as_a_number() {
        let mut mgr = agnos_with_npc(500);
        mgr.get_entity_mut(500).unwrap().respawn_secs = Some(60);
        let logs = LogCapture::install();

        log_spawn_behaviour(&mut mgr, 500);

        let row = logs
            .find_message(Level::DEBUG, "resolved behaviour")
            .expect("spawner.npc_behaviour row");
        assert_eq!(row.target, "spawner.npc_behaviour");
        assert!(row.has_field("respawn_secs", "60"), "{:?}", row.fields);
        assert!(row.has_field("world", "Agnos"), "{:?}", row.fields);
    }

    /// Revert proof: with `template_id = e.template_id.unwrap_or(0)` the row
    /// carries `template_id = 0` for this NPC, so the absence check fails.
    /// `space_id` is the bare id, not `"Some(..)"` or `0`.
    #[test]
    fn npc_behaviour_row_leaves_out_an_unknown_template_id() {
        let mut mgr = agnos_with_npc(500);
        let logs = LogCapture::install();

        log_spawn_behaviour(&mut mgr, 500);

        let row = logs
            .find_message(Level::DEBUG, "resolved behaviour")
            .expect("spawner.npc_behaviour row");
        assert!(
            !row.fields.contains_key("template_id"),
            "an NPC with no template leaves the key out: {:?}",
            row.fields
        );
        let space_id = mgr.get_entity_space_id(500).unwrap();
        assert!(
            row.has_field("space_id", &space_id.to_string()),
            "{:?}",
            row.fields
        );
    }
}
