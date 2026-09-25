//! Read-only live-state snapshots for the research lab (issue #688, phase 5).
//!
//! These methods back [`crate::cell::messages::BaseToCellMsg::LabQuery`]. They
//! run on the cell loop thread, so every one copies out primitives into an
//! owned [`LabEntitySnapshot`] / [`LabWitnessReport`] — no reference into
//! `SpaceManager` escapes, and an [`LabQuery::EntityQuery`] is capped at
//! [`LAB_ENTITY_QUERY_CAP`] so a query over a dense space cannot stall the
//! tick. Nothing here mutates state.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::HEALTH;

use crate::cell::messages::{
    LabEntityFilter, LabEntitySnapshot, LabQueryReply, LabRadiusCenter, LabWitnessReport,
    LAB_ENTITY_QUERY_CAP,
};

use super::{SpaceInstance, SpaceManager};

impl SpaceManager {
    /// Snapshot one entity by id, or `None` if it does not exist.
    pub fn lab_entity_snapshot(&self, entity_id: u32) -> Option<LabEntitySnapshot> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        let entity = space.entities.get(&entity_id)?;
        Some(snapshot_entity(space, entity))
    }

    /// Answer a [`LabQuery::EntityQuery`]: every entity matching `filter`,
    /// capped at [`LAB_ENTITY_QUERY_CAP`]. Returns `Err` only when the filter's
    /// radius is centered on an entity that does not exist — a query that
    /// simply matches nothing returns an empty `Entities` reply.
    ///
    /// [`LabQuery::EntityQuery`]: crate::cell::messages::LabQuery::EntityQuery
    pub fn lab_query_entities(&self, filter: &LabEntityFilter) -> Result<LabQueryReply, String> {
        // Resolve the radius center up front so a bad anchor fails the whole
        // query rather than silently matching nothing.
        let radius_check = match &filter.radius {
            None => None,
            Some(r) => {
                let center = match &r.center {
                    LabRadiusCenter::Point(p) => *p,
                    LabRadiusCenter::Entity(anchor) => {
                        let entity = self.get_entity(*anchor).ok_or_else(|| {
                            format!("radius anchor entity {anchor} does not exist")
                        })?;
                        [entity.position.x, entity.position.y, entity.position.z]
                    }
                };
                Some((center, r.radius * r.radius))
            }
        };

        let mut entities: Vec<LabEntitySnapshot> = Vec::new();
        let mut total_matched = 0usize;

        for space in self.spaces.values() {
            if filter.space_id.is_some_and(|sid| sid != space.space_id) {
                continue;
            }
            for entity in space.entities.values() {
                if filter
                    .template_id
                    .is_some_and(|t| entity.template_id != Some(t))
                {
                    continue;
                }
                if filter.class_id.is_some_and(|c| entity.class_id != c) {
                    continue;
                }
                if let Some((center, radius_sq)) = radius_check {
                    let dx = entity.position.x - center[0];
                    let dy = entity.position.y - center[1];
                    let dz = entity.position.z - center[2];
                    if dx * dx + dy * dy + dz * dz > radius_sq {
                        continue;
                    }
                }
                total_matched += 1;
                if entities.len() < LAB_ENTITY_QUERY_CAP {
                    entities.push(snapshot_entity(space, entity));
                }
            }
        }

        let capped = total_matched > entities.len();
        Ok(LabQueryReply::Entities {
            entities,
            total_matched,
            capped,
        })
    }

    /// Answer a [`LabQuery::Witnesses`]: who witnesses `entity_id`, and whom it
    /// witnesses. `None` if the entity does not exist.
    ///
    /// [`LabQuery::Witnesses`]: crate::cell::messages::LabQuery::Witnesses
    pub fn lab_witness_report(&self, entity_id: u32) -> Option<LabWitnessReport> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let entity = self.get_entity(entity_id)?;
        // Whom X sees (populated only for players; empty for NPCs).
        let mut witnesses: Vec<u32> = entity.witnesses.iter().map(|e| e.0 as u32).collect();
        witnesses.sort_unstable();
        // Who sees X (observers). Reuses the existing reverse-scan helper.
        let mut witnessed_by = self.get_witnesses_of(entity_id);
        witnessed_by.sort_unstable();
        Some(LabWitnessReport {
            entity_id,
            space_id,
            witnessed_by,
            witnesses,
        })
    }
}

/// Build an owned snapshot from a live entity. All fields are copied out.
fn snapshot_entity(space: &SpaceInstance, entity: &CellEntity) -> LabEntitySnapshot {
    let health = entity.stats.get(HEALTH);
    LabEntitySnapshot {
        entity_id: entity.entity_id.0 as u32,
        space_id: space.space_id,
        world_name: space.world_name.clone(),
        position: [entity.position.x, entity.position.y, entity.position.z],
        direction: [entity.direction.x, entity.direction.y, entity.direction.z],
        velocity: entity.velocity,
        is_on_ground: entity.is_on_ground,
        is_player: entity.is_player,
        class_id: entity.class_id,
        faction: entity.faction,
        alignment: entity.alignment,
        level: entity.level,
        name: entity
            .character_name
            .clone()
            .or_else(|| entity.npc_name.clone()),
        template_id: entity.template_id,
        spawn_id: entity.spawn_id,
        tag: entity.tag.clone(),
        name_id: entity.name_id,
        archetype_id: entity.archetype_id,
        access_level: entity.access_level,
        ai_state: format!("{:?}", entity.ai_state),
        current_target_id: entity.current_target_id,
        aoi_radius: entity.aoi_radius,
        state_field: entity.state_field,
        interaction_type_flags: entity.interaction_type_flags,
        weapon_holstered: entity.weapon_holstered,
        has_static_mesh: entity.static_mesh.is_some(),
        component_count: entity.components.len(),
        witness_count: entity.witnesses.len(),
        health_cur: health.map(|s| s.cur),
        health_max: health.map(|s| s.max),
    }
}
