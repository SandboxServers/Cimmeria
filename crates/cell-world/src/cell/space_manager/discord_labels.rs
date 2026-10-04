//! The cell's objects as Discord pairs (Rule 6, "Discord"; NT-10): the
//! character, entity and world an event names, each its ID and its name,
//! read from fields the entity and space already carry.

use cimmeria_discord::Named;

use super::SpaceManager;

impl SpaceManager {
    /// The character on `entity_id`: its `player_id` and the name the base
    /// threaded in with `InitPlayerState`, or `entity:<id>` before either
    /// is cached.
    pub fn discord_character(&self, entity_id: u32) -> Named {
        self.get_entity(entity_id)
            .map_or_else(Named::default, |e| {
                Named::from_parts(e.player_id.map(i64::from), e.character_name.clone())
            })
            .or_entity(entity_id)
    }

    /// Any entity: a player as [`discord_character`](Self::discord_character)
    /// (`player_id` + name), anything else as its `entity_id` and NPC name.
    pub fn discord_entity(&self, entity_id: u32) -> Named {
        match self.get_entity(entity_id) {
            Some(e) if e.player_id.is_some() => self.discord_character(entity_id),
            Some(e) => Named::new(entity_id, e.npc_name.clone()),
            None => Named::new(entity_id, None),
        }
    }

    /// The world `entity_id` is in: its `world_id` and name.
    pub fn discord_world_of(&self, entity_id: u32) -> Option<Named> {
        self.get_entity_world_name(entity_id)
            .map(|w| self.discord_world(&w))
    }

    /// A world by name: its `world_id` (when the spawner stamped one) and
    /// the name.
    pub fn discord_world(&self, world_name: &str) -> Named {
        Named::from_parts(
            self.world_id_for_world(world_name).map(i64::from),
            Some(world_name.to_string()),
        )
    }
}
