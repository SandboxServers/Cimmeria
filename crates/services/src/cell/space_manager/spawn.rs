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

    /// Spawn a **mission-scoped** NPC from a cached `entity_templates`
    /// prototype into a specific space, applying the per-spawn overrides a
    /// content `spawn_entity` action carries.
    ///
    /// Returns the new NPC's entity id (the caller does not allocate it —
    /// keeping the allocation in here means a failed template lookup can't
    /// burn an id).
    ///
    /// # Why `respawn_secs` is forced to `None`
    ///
    /// Not defensiveness — a correctness requirement. `npc_respawn_tick`
    /// selects purely on `ai_state == Dead && respawn_at <= now`, and
    /// `mark_npc_dead` stamps `respawn_at` from `entity.respawn_secs`.
    /// Neither consults `spawn_id`, so the `-1` non-DB sentinel gives no
    /// protection at all (the comment on the GM path that credits the
    /// sentinel is describing the right behaviour via the wrong mechanism —
    /// its `respawn_secs: None` is doing all the work).
    ///
    /// A revived mission NPC re-fires its `entity_dead_tag` chain, so a
    /// kill objective can complete twice. That is worse than no respawn, and
    /// the template's own `respawn_secs` would opt every mission spawn into
    /// it *silently* — hence the unconditional override rather than a
    /// "only if the action didn't ask" one. Content-scoped respawn needs
    /// instance-lifetime awareness in the respawn tick and is a recorded gap.
    pub fn spawn_npc_from_template(
        &mut self,
        template_id: i32,
        space_id: u32,
        world_name: &str,
        position: [f32; 3],
        heading: f32,
        tag: &str,
        is_stationary: bool,
        aggression: Option<cimmeria_entity::cell_entity::MobAggression>,
    ) -> Result<u32, String> {
        let Some(prototype) = self.spawn_templates.get(&template_id) else {
            return Err(format!(
                "template {template_id} not in the entity_templates cache"
            ));
        };
        let mut record = prototype.clone();
        record.world_name = world_name.to_string();
        record.x = position[0];
        record.y = position[1];
        record.z = position[2];
        record.heading = heading;
        record.tag = Some(tag.to_string());
        record.is_stationary = is_stationary;
        record.respawn_secs = None;

        let entity_id = self.allocate_npc_id();
        self.spawn_npc_from_record_into(entity_id, &record, space_id)?;

        // The action's `aggression` is a per-spawn override, the runtime
        // twin of `spawnlist.aggression_override` (NA13). `None` leaves the
        // record's value (templates carry none), i.e. faction-derived. Set
        // here rather than making content author a second action, so the
        // NPC's disposition holds from the first AI tick.
        //
        // No ordering hazard: the AI tick runs from the cell message loop
        // on the same task, and this whole function holds `&mut self`.
        if aggression.is_some() {
            if let Some(e) = self.get_entity_mut(entity_id) {
                e.aggro.override_level = aggression;
            }
        }
        Ok(entity_id)
    }

    /// The seeded spawn point, with its Y moved onto the navmesh floor when
    /// the seed is close enough to it (NA11, audit S9).
    ///
    /// Seeded Y is often a model origin a little above or below the floor.
    /// The pathfinder looks up an NPC's start polygon in a ±0.5 box, so a
    /// guard seeded 0.6 u over a flat floor could never route, and every
    /// leash and respawn put it back on the same bad point. Snapping here,
    /// and storing the result as `spawn_position`, fixes all three at once:
    /// the leash, the respawn tick and wander all read `spawn_position`.
    ///
    /// "Close enough" is the `is_point_valid` band (up to 4 u above the
    /// floor, or `2 * agent_radius` below it). A seed outside the band is
    /// kept as authored: it is either on geometry the mesh does not cover
    /// or a data error, and moving it several units would hide which.
    ///
    /// Props (a `static_mesh`) and stationary NPCs keep their authored Y.
    /// They never path, and a height above the mesh is often deliberate: a
    /// console on a desk, a turret on a platform.
    fn grounded_spawn_position(
        &self,
        space_id: u32,
        record: &super::super::spawner::SpawnRecord,
    ) -> Vector3 {
        let seeded = Vector3::new(record.x, record.y, record.z);
        if record.static_mesh.is_some() || record.is_stationary {
            return seeded;
        }
        let Some(navmesh) = self.spaces.get(&space_id).and_then(|s| s.navmesh.as_ref()) else {
            return seeded;
        };
        if !navmesh.is_point_valid(&seeded) {
            return seeded;
        }
        match navmesh.get_height_near(seeded.x, seeded.y, seeded.z) {
            Some(floor) => {
                if (floor - seeded.y).abs() > 0.01 {
                    tracing::debug!(
                        target: "spawner.npc_behaviour",
                        event = "spawn_grounded",
                        spawn_id = record.spawn_id,
                        template_id = record.template_id,
                        seeded_y = seeded.y,
                        floor_y = floor,
                        "NPC spawn Y moved onto the navmesh floor"
                    );
                }
                Vector3::new(seeded.x, floor, seeded.z)
            }
            None => seeded,
        }
    }

    /// Internal: spawn an NPC from a record into a given space_id.
    fn spawn_npc_from_record_into(
        &mut self,
        entity_id: u32,
        record: &super::super::spawner::SpawnRecord,
        space_id: u32,
    ) -> Result<u32, String> {
        let pos = self.grounded_spawn_position(space_id, record);
        // heading is yaw (rotation.y), x and z rotation are 0
        let dir = Vector3::new(0.0, record.heading, 0.0);

        let mut e = CellEntity::new(EntityId(entity_id as i32), SpaceId(space_id as i32), pos);
        e.direction = dir;
        e.class_id = super::super::spawner::class_id_for_class(&record.class);
        e.is_player = false;
        // Cover use from the template (NA22, audit C5): the
        // `entity_templates.use_cover` column, or the default rule when it
        // is NULL. Melee-only NPCs are excluded at fight time, where the
        // ability defs are known (the startup population spawns before
        // they load).
        e.use_cover = resolve_use_cover(record);
        e.level = record.level.unwrap_or(1) as u32;
        e.npc_name = Some(record.template_name.clone());

        // Template-driven fields
        e.template_id = Some(record.template_id);
        // `-1` is the GM/command-spawn sentinel (no spawnlist row) — keep only
        // real positive ids so authoring commands don't target a phantom row.
        e.spawn_id = (record.spawn_id > 0).then_some(record.spawn_id);
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
        // without a patrol path and is not hostile on sight.
        e.wander_radius = record.wander_radius;
        e.wander_min_dwell_secs = record.wander_min_dwell_secs;
        e.wander_max_dwell_secs = record.wander_max_dwell_secs;
        e.follow_min_distance = record.follow_min_distance;
        e.follow_max_distance = record.follow_max_distance;
        // Per-template movement speed override. `CellEntity::new`
        // already defaults `move_speed` to 0.6, and the spawner's SQL
        // COALESCEs a NULL template column to the same 0.6 — this
        // assignment is a no-op for every template except ones that
        // opt into a faster (or slower) pace, e.g. escort NPCs that
        // need to keep up with a following player (GC1b-0).
        e.move_speed = record.move_speed;
        // Per-template leash radius (NA12). `None` keeps the server default
        // (`combat::LEASH_DISTANCE`), resolved where the leash is measured.
        e.leash.distance_override = record.leash_distance;
        // Seeded aggression override (`spawnlist.aggression_override`) and
        // per-template aggro radius (`entity_templates.aggro_radius`), NA13.
        // `None` on either means faction-derived / the server default.
        e.aggro.override_level = record.aggression_override;
        e.aggro.radius_override = record.aggro_radius;
        // Per-template assist radius (`entity_templates.assist_radius`),
        // NA14. `None` means the server default (10 u).
        e.aggro.assist_radius_override = record.assist_radius;

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

        // An NPC authored standing at a cover marker spawns holding it
        // (NA22, audit C4). A no-op for the startup population, which
        // spawns before cover loads; `cover_loaded` sweeps it instead.
        crate::cell::cover::hold_spawn_cover(self, entity_id, "spawn");

        Ok(space_id)
    }
}

/// Whether a spawned NPC takes cover (NA22). The template's
/// `entity_templates.use_cover` decides; NULL means "a hostile NPC does"
/// (`faction = HOSTILE_FACTION`, the only faction a player can fight). A
/// stationary NPC or a prop (`static_mesh`) never does, whatever the
/// column says: it cannot walk to a slot. `SGWMob.def` `useCover` is an
/// `INT8` defaulting to 0 on the client, but no template data for it
/// shipped, so the default rule stands in for the missing per-mob values.
pub(crate) fn resolve_use_cover(record: &super::super::spawner::SpawnRecord) -> bool {
    if record.is_stationary || record.static_mesh.is_some() {
        return false;
    }
    record
        .use_cover
        .unwrap_or(record.faction == Some(i32::from(super::super::combat::HOSTILE_FACTION)))
}
