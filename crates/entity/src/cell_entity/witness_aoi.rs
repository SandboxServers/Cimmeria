//! Spatial accessors: position, witness set, and area-of-interest checks.
//!
//! These are the cell entity's core spatial read/write helpers — the
//! position getter/setter, the witness-set mutators, and the AoI radius
//! test used to decide whether another entity can become a witness.

use cimmeria_common::{EntityId, Vector3};

use std::collections::HashSet;

use super::CellEntity;

impl CellEntity {
    /// Update the entity's world-space position.
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// Get the entity's current world-space position.
    pub fn get_position(&self) -> &Vector3 {
        &self.position
    }

    /// Add an entity to the witness set (it can now see this entity).
    pub fn add_witness(&mut self, entity_id: EntityId) {
        self.witnesses.insert(entity_id);
    }

    /// Remove an entity from the witness set (it can no longer see this entity).
    pub fn remove_witness(&mut self, entity_id: EntityId) {
        self.witnesses.remove(&entity_id);
    }

    /// Get the current set of witness entity IDs.
    pub fn get_witnesses(&self) -> &HashSet<EntityId> {
        &self.witnesses
    }

    /// Whether this entity may be introduced into another player's AoI yet.
    ///
    /// NPCs and props always are. A *player's* cell entity is not until its
    /// session has both connected (`ConnectEntity`, which sets `is_player`)
    /// and had `InitPlayerState` land (which sets `archetype_id` and seeds
    /// the archetype stats). The cell entity exists from `CreateEntity`
    /// onward — the whole time the client is loading the map — and in a
    /// shared (non-instanced) world every nearby player's AoI tick sees it.
    /// Introducing it in that window ships it as an NPC-shaped, blank
    /// entity, and because the witness set is marked on introduction it is
    /// never re-introduced once the real state exists.
    ///
    /// `account_id` is the player discriminator here rather than
    /// `is_player` precisely because `is_player` is still `false` for the
    /// window this guards; it is stamped at `CreateEntity` and is `None`
    /// for every server-spawned entity.
    pub fn is_introducible(&self) -> bool {
        self.account_id.is_none() || (self.is_player && self.archetype_id.is_some())
    }

    /// Returns `true` if the given position is within this entity's AoI radius.
    ///
    /// Uses squared distance comparison to avoid a square root.
    pub fn is_in_aoi(&self, other_pos: &Vector3) -> bool {
        self.position.distance_squared_to(other_pos) <= self.aoi_radius * self.aoi_radius
    }
}
