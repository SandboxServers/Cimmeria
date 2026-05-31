//! NPC spawning: from a basic position, or from a full database spawn record.
//!
//! NPCs use `class_id = 0x04` (SGWMob) and a reserved entity ID range starting
//! at 100_000 to avoid collision with player IDs. Template-driven spawns
//! (`spawn_npc_from_record`) populate interaction flags, faction, alignment,
//! visual components, and stats.

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;

use super::SpaceManager;

impl SpaceManager {
    /// Spawn an NPC entity in the named world at the given position.
    ///
    /// Returns the space_id the NPC was placed in.
    /// NPC entities have `is_player = false` and `class_id = 0x04` (SGWMob).
    /// They participate in AoI but don't generate AoI queries themselves.
    pub fn spawn_npc(
        &mut self,
        entity_id: u32,
        world_name: &str,
        position: [f32; 3],
        direction: [f32; 3],
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(world_name)?;
        let pos = Vector3::new(position[0], position[1], position[2]);
        let dir = Vector3::new(direction[0], direction[1], direction[2]);

        let mut cell_entity =
            CellEntity::new(EntityId(entity_id as i32), SpaceId(space_id as i32), pos);
        cell_entity.direction = dir;
        cell_entity.class_id = 0x04; // SGWMob
        cell_entity.is_player = false;
        cell_entity.spawn_position = Some(pos);
        cell_entity.spawn_direction = Some(dir);
        cell_entity
            .abilities
            .add_ability(super::super::combat::NPC_DEFAULT_ABILITY);

        let space = self
            .spaces
            .get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, cell_entity);
        self.entity_space.insert(entity_id, space_id);

        tracing::debug!(entity_id, space_id, ?position, "NPC entity spawned");
        Ok(space_id)
    }

    /// Get the next NPC entity ID from a reserved range.
    ///
    /// NPC IDs start at 100_000 to avoid collision with player entity IDs
    /// (which are allocated sequentially from 1 by the EntityManager).
    pub fn allocate_npc_id(&mut self) -> u32 {
        let id = self.next_npc_id;
        self.next_npc_id += 1;
        id
    }

    /// Spawn an NPC entity from a database spawn record with full template data.
    ///
    /// Sets class_id, interaction flags, name, faction, alignment, and all other
    /// template-driven fields. Returns the space_id the NPC was placed in.
    ///
    /// For non-instanced worlds, uses `find_or_create_space` to resolve the space.
    /// For instanced worlds, callers should use `spawn_npc_from_record_in_space`
    /// instead to target a specific space_id.
    pub fn spawn_npc_from_record(
        &mut self,
        entity_id: u32,
        record: &super::super::spawner::SpawnRecord,
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(&record.world_name)?;
        self.spawn_npc_from_record_into(entity_id, record, space_id)
    }

    /// Spawn an NPC entity from a database spawn record into a specific space.
    ///
    /// Used for instanced spaces where each player gets their own space_id.
    /// The caller is responsible for providing the correct space_id.
    pub fn spawn_npc_from_record_in_space(
        &mut self,
        entity_id: u32,
        record: &super::super::spawner::SpawnRecord,
        space_id: u32,
    ) -> Result<u32, String> {
        self.spawn_npc_from_record_into(entity_id, record, space_id)
    }

    /// Internal: spawn an NPC from a record into a given space_id.
    fn spawn_npc_from_record_into(
        &mut self,
        entity_id: u32,
        record: &super::super::spawner::SpawnRecord,
        space_id: u32,
    ) -> Result<u32, String> {
        let pos = Vector3::new(record.x, record.y, record.z);
        // heading is yaw (rotation.y), x and z rotation are 0
        let dir = Vector3::new(0.0, record.heading, 0.0);

        let mut e = CellEntity::new(EntityId(entity_id as i32), SpaceId(space_id as i32), pos);
        e.direction = dir;
        e.class_id = super::super::spawner::class_id_for_class(&record.class);
        e.is_player = false;
        // Default-on cover use for spawned NPCs: the cover scorer + AI
        // tick respect `use_cover`, and existing mob templates don't
        // (yet) carry a DB-driven override. Per-template tuning is a
        // follow-up issue once the `entity_templates.use_cover` column
        // lands; for v1 every NPC opts in. Issue #209.
        e.use_cover = true;
        e.level = record.level.unwrap_or(1) as u32;
        e.npc_name = Some(record.template_name.clone());

        // Template-driven fields
        e.template_id = Some(record.template_id);
        e.tag = record.tag.clone();
        e.name_id = record.name_id;
        e.speaker_id = record.speaker_id;
        e.event_set_id = record.event_set_id;
        e.interaction_type_flags = record.interaction_type;
        e.entity_flags = record.flags as u64;
        e.faction = record.faction.unwrap_or(0) as u8;
        e.alignment = record.alignment.unwrap_or(0) as u8;
        e.static_interaction_sets = record.static_interaction_sets.clone();
        e.has_dynamic_properties = record.has_dynamic_properties;
        e.static_mesh = record.static_mesh.clone();
        e.body_set = Some(record.body_set.clone());
        e.components = record.components.clone().unwrap_or_default();
        e.spawn_position = Some(pos);
        e.spawn_direction = Some(dir);
        e.is_stationary = record.is_stationary;
        e.loot_table_id = record.loot_table_id;
        // `respawn_secs` is the resolved template/spawn precedence
        // already collapsed by the loader's COALESCE.
        // `original_interaction_type_flags` snapshots the template's
        // interaction bits *before* death OR-merges `INT_NormalLoot`,
        // so the respawn tick can restore the pre-death state cleanly
        // without dragging the loot bit forward.
        e.respawn_secs = record.respawn_secs;
        e.original_interaction_type_flags = record.interaction_type;
        // Patrol waypoints loaded from `entity_templates.patrol_path_id`
        // → `point_set_points` by the spawner SQL. Empty for the
        // common case (non-patrolling NPC). Non-empty paths cause
        // the AI tick to admit the NPC into the Patrol state once
        // it reaches Idle and has no threats.
        e.patrol_path = record.patrol_path.clone();
        e.patrol_point_delay_secs = record.patrol_point_delay_secs;
        // Wander config: 0.0 radius → no wander. Positive value
        // opts the NPC into AiState::Wander when it reaches Idle
        // without a patrol path / aggression.
        e.wander_radius = record.wander_radius;
        e.wander_min_dwell_secs = record.wander_min_dwell_secs;
        e.wander_max_dwell_secs = record.wander_max_dwell_secs;
        e.follow_min_distance = record.follow_min_distance;
        e.follow_max_distance = record.follow_max_distance;

        // Per-template ability bucket. Empty `ability_ids` (template has
        // no `ability_set_id`) falls back to `NPC_DEFAULT_ABILITY` so
        // unspecified mobs still have something to fire. Matches the
        // Python convention where un-assigned NPCs default to Pistol Shot.
        if record.ability_ids.is_empty() {
            e.abilities
                .add_ability(super::super::combat::NPC_DEFAULT_ABILITY);
        } else {
            for &ability_id in &record.ability_ids {
                e.abilities.add_ability(ability_id);
            }
        }
        // Initialize NPC health based on level (simple scaling)
        use cimmeria_entity::stats::{FOCUS, HEALTH};
        let hp = 200 + (e.level as i32 * 50);
        if let Some(stat) = e.stats.get_mut(HEALTH) {
            stat.max = hp;
            stat.set_current(hp);
        }
        if let Some(stat) = e.stats.get_mut(FOCUS) {
            stat.max = 200;
            stat.set_current(200);
        }

        let space = self
            .spaces
            .get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, e);
        self.entity_space.insert(entity_id, space_id);

        Ok(space_id)
    }
}
