//! Read-only accessors over the space/entity tables.

use cimmeria_common::EntityId;
use cimmeria_entity::cell_entity::CellEntity;

use super::{RegionData, SpaceManager};

impl SpaceManager {
    /// Return all active spaces as (space_id, world_name) pairs.
    pub fn all_spaces(&self) -> Vec<(u32, String)> {
        self.spaces
            .values()
            .map(|s| (s.space_id, s.world_name.clone()))
            .collect()
    }

    /// Number of loaded world definitions.
    pub fn world_count(&self) -> usize {
        self.worlds.len()
    }

    /// Number of active space instances.
    pub fn space_count(&self) -> usize {
        self.spaces.len()
    }

    /// Look up the space_id for a world name.
    pub fn space_id_for_world(&self, world_name: &str) -> Option<u32> {
        self.world_spaces.get(world_name).copied()
    }

    /// Get a mutable reference to a cell entity by its entity ID.
    ///
    /// Searches across all spaces using the entity→space index.
    pub fn get_entity_mut(&mut self, entity_id: u32) -> Option<&mut CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get_mut(&space_id)?;
        space.entities.get_mut(&entity_id)
    }

    /// Get an immutable reference to a cell entity by its entity ID.
    pub fn get_entity(&self, entity_id: u32) -> Option<&CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        space.entities.get(&entity_id)
    }

    /// Get the world name for an entity's current space.
    pub fn get_entity_world_name(&self, entity_id: u32) -> Option<String> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        Some(space.world_name.clone())
    }

    /// Get the objectives for a given step from the step_objectives cache.
    ///
    /// Returns an empty vec if the step has no objectives in the cache.
    pub fn get_step_objectives(
        &self,
        step_id: i32,
    ) -> Vec<super::super::spawner::MissionObjectiveDef> {
        self.step_objectives
            .get(&step_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Look up a registered region by its runtime ID.
    pub fn get_region(&self, runtime_id: u32) -> Option<&RegionData> {
        self.regions.get(&runtime_id)
    }

    /// Return all registered regions for a given world name.
    pub fn regions_for_world(&self, world_name: &str) -> Vec<&RegionData> {
        self.regions
            .values()
            .filter(|r| r.world_name == world_name)
            .collect()
    }

    /// Collect all NPC entity IDs (class_id=0x04, not players) across all spaces.
    pub fn all_npc_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            for entity in space.entities.values() {
                if !entity.is_player && entity.class_id == 0x04 {
                    ids.push(entity.entity_id.0 as u32);
                }
            }
        }
        ids
    }

    /// Collect all player entity IDs (entries in each space's `players` set)
    /// across all spaces. Returned as a `Vec` so callers can iterate without
    /// holding a borrow on `SpaceManager`.
    pub fn all_player_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            ids.extend(space.players.iter().copied());
        }
        ids
    }

    /// Find an NPC entity by its spawn tag within the same space as `source_entity_id`.
    ///
    /// Used by content chain actions (SetInteractionType, DestroyTaggedEntity, etc.)
    /// to locate entities by their `spawnlist.tag` value. Restricting the search
    /// to the source's space prevents instanced worlds from leaking entity
    /// resolution across instance boundaries.
    pub fn find_entity_by_tag(&self, source_entity_id: u32, tag: &str) -> Option<u32> {
        let &space_id = self.entity_space.get(&source_entity_id)?;
        let space = self.spaces.get(&space_id)?;
        space
            .entities
            .iter()
            .find_map(|(&eid, entity)| (entity.tag.as_deref() == Some(tag)).then_some(eid))
    }

    /// Find all entities with a given `template_id` in the same space as
    /// `source_entity_id`.
    ///
    /// Used by `add_dialog_set` to locate NPC entities that match the slot
    /// (template_id) so per-player InteractionType updates can be sent.
    /// Restricted to a single space so instanced worlds don't cross-pollinate.
    pub fn find_entities_by_template(&self, source_entity_id: u32, template_id: i32) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&source_entity_id) else {
            return Vec::new();
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return Vec::new();
        };
        space
            .entities
            .iter()
            .filter(|(_, e)| e.template_id == Some(template_id))
            .map(|(&eid, _)| eid)
            .collect()
    }

    /// Return all player entity IDs that currently have `target_entity_id` in their AoI.
    ///
    /// Used for broadcasting property updates (InteractionType, SetVisible, etc.)
    /// to players who can see the entity.
    ///
    /// In this codebase `entity.witnesses` is populated only for players (see
    /// [`super::aoi::SpaceManager::compute_aoi_changes`]), and stores the set
    /// of entities the player currently sees. The reverse mapping (observers
    /// of a target) isn't materialized, so we have to scan player witness
    /// sets. Restricted to the target's own space, so the scan is bounded by
    /// players in that space rather than the whole world.
    pub fn get_witnesses_of(&self, target_entity_id: u32) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&target_entity_id) else {
            return vec![];
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return vec![];
        };
        let target_eid = EntityId(target_entity_id as i32);
        space
            .players
            .iter()
            .filter(|&&pid| {
                space
                    .entities
                    .get(&pid)
                    .is_some_and(|p| p.witnesses.contains(&target_eid))
            })
            .copied()
            .collect()
    }
}
