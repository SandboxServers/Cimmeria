//! A minimal template-shaped `SpawnRecord`, for tests that spawn an NPC
//! through `spawn_npc_from_record` (so the spawn-time derivations run) but
//! need no seed row.

use cimmeria_entity::cell_entity::VaultScope;

use crate::cell::spawner::SpawnRecord;

/// A friendly (`faction = 1`), stationary `mob` template at `pos` in
/// `world`, carrying `interaction_type` and `vault_scope`; every other field
/// is the loader's default. `spawn_id = -1` is the non-DB sentinel.
pub fn npc_spawn_record(
    world: &str,
    pos: [f32; 3],
    interaction_type: i64,
    vault_scope: VaultScope,
) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: world.to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        heading: 0.0,
        tag: None,
        template_id: 1,
        template_name: "Test NPC".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(1),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
        is_stationary: true,
        ability_ids: vec![],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
        vault_scope,
    }
}
