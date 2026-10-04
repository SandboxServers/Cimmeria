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
    /// A player is an entity with either half of its character identity
    /// cached, so a known character name is never paired with an entity ID.
    pub fn discord_entity(&self, entity_id: u32) -> Named {
        match self.get_entity(entity_id) {
            Some(e) if e.player_id.is_some() || e.character_name.is_some() => {
                self.discord_character(entity_id)
            }
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

#[cfg(test)]
mod tests {
    use cimmeria_discord::Named;

    use crate::test_fixtures::make_space_manager;

    const PLAYER: u32 = 7001;
    const NPC: u32 = 7002;

    fn world() -> crate::cell::space_manager::SpaceManager {
        let mut mgr = make_space_manager();
        mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(NPC, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(NPC).unwrap().npc_name = Some("Jaffa Guard".into());
        mgr.worlds.get_mut("Agnos").unwrap().world_id = Some(21);
        mgr
    }

    fn cache_player(
        mgr: &mut crate::cell::space_manager::SpaceManager,
        id: Option<i32>,
        name: Option<&str>,
    ) {
        let e = mgr.get_entity_mut(PLAYER).unwrap();
        e.player_id = id;
        e.character_name = name.map(str::to_string);
    }

    /// A character pairs with its `player_id`, never its entity ID.
    #[test]
    fn character_pairs_player_id_with_name() {
        let mut mgr = world();
        cache_player(&mut mgr, Some(12), Some("alice"));
        assert_eq!(
            mgr.discord_character(PLAYER),
            Named::new(12, Some("alice".into()))
        );
    }

    /// Before `InitPlayerState` lands, and for an unknown entity, the
    /// character is labelled by entity, not given an ID it doesn't have.
    #[test]
    fn uncached_character_is_labelled_by_entity() {
        let mgr = world();
        assert_eq!(
            mgr.discord_character(PLAYER),
            Named::name_only("entity:7001")
        );
        assert_eq!(mgr.discord_character(9999), Named::name_only("entity:9999"));
    }

    #[test]
    fn entity_is_a_character_for_players_and_entity_id_for_npcs() {
        let mut mgr = world();
        cache_player(&mut mgr, Some(12), Some("alice"));
        assert_eq!(
            mgr.discord_entity(PLAYER),
            Named::new(12, Some("alice".into()))
        );
        assert_eq!(
            mgr.discord_entity(NPC),
            Named::new(NPC, Some("Jaffa Guard".into()))
        );
        assert_eq!(mgr.discord_entity(9999), Named::new(9999, None));
    }

    /// A player with only its name cached is still a character: the name
    /// must not sit next to its entity ID (review finding 3).
    #[test]
    fn entity_with_only_a_character_name_stays_a_character() {
        let mut mgr = world();
        cache_player(&mut mgr, None, Some("alice"));
        assert_eq!(mgr.discord_entity(PLAYER), Named::name_only("alice"));
    }

    #[test]
    fn world_pairs_world_id_with_name() {
        let mgr = world();
        assert_eq!(
            mgr.discord_world("Agnos"),
            Named::new(21, Some("Agnos".into()))
        );
        assert_eq!(
            mgr.discord_world_of(NPC),
            Some(Named::new(21, Some("Agnos".into())))
        );
        assert_eq!(mgr.discord_world("Nowhere"), Named::name_only("Nowhere"));
        assert_eq!(mgr.discord_world_of(9999), None);
    }
}
