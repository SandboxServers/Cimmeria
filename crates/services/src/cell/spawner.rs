//! NPC spawn system for the CellService.
//!
//! Populates world spaces with NPC entities at predefined spawn points.
//! In the original server, spawn data lived in `SGWSpawnSet` / `SGWSpawnRegion`
//! Python entities. For now, we use hardcoded spawn definitions until the full
//! entity data pipeline is built.
//!
//! Reference: `python/base/SGWSpawnSet.py`, `python/cell/SGWMob.py`

use cimmeria_entity::cell_entity::NpcInteractionType;

use super::space_manager::SpaceManager;

/// A spawn point definition.
#[derive(Debug, Clone)]
pub struct SpawnDef {
    /// World name the NPC spawns in.
    pub world_name: &'static str,
    /// World-space position [x, y, z].
    pub position: [f32; 3],
    /// Facing direction [x, y, z] (rotation radians).
    pub direction: [f32; 3],
    /// Display name.
    pub name: &'static str,
    /// Interaction type (None = hostile/no interaction).
    pub interaction: Option<NpcInteractionType>,
    /// Entity level (affects XP granted on kill).
    pub level: u32,
}

/// Hardcoded spawn definitions for initial world population.
///
/// These positions are representative locations in the starter worlds.
/// Real spawn data will come from the database/spawn tables in a later phase.
const SPAWN_DEFS: &[SpawnDef] = &[
    // Agnos — starter area NPCs near the Stargate
    SpawnDef {
        world_name: "Agnos", position: [30.0, 0.0, 50.0], direction: [0.0, 0.0, 0.0],
        name: "Agnos Vendor", interaction: Some(NpcInteractionType::Vendor), level: 1,
    },
    SpawnDef {
        world_name: "Agnos", position: [40.0, 0.0, 55.0], direction: [0.0, 1.57, 0.0],
        name: "Agnos Guide", interaction: Some(NpcInteractionType::Dialog { dialog_id: 1 }), level: 1,
    },
    SpawnDef {
        world_name: "Agnos", position: [25.0, 0.0, 65.0], direction: [0.0, 3.14, 0.0],
        name: "Jaffa Warrior", interaction: None, level: 2,
    },
    // Castle — training area NPCs
    SpawnDef {
        world_name: "Castle", position: [100.0, 0.0, 100.0], direction: [0.0, 0.0, 0.0],
        name: "Soldier Trainer", interaction: Some(NpcInteractionType::Trainer { archetype_id: 1 }), level: 3,
    },
    SpawnDef {
        world_name: "Castle", position: [110.0, 0.0, 95.0], direction: [0.0, 0.78, 0.0],
        name: "Castle Guard", interaction: None, level: 4,
    },
];

/// Per-instance spawn definitions for instanced worlds.
///
/// These NPCs are spawned when a player first enters an instanced space (e.g.,
/// Castle_CellBlock tutorial). They differ from SPAWN_DEFS which are for
/// persistent startup spaces.
const INSTANCE_SPAWN_DEFS: &[SpawnDef] = &[
    // Castle_CellBlock — tutorial instance
    SpawnDef {
        world_name: "Castle_CellBlock",
        position: [-328.3, 73.472, -210.27],
        direction: [0.0, 1.5708, 0.0],
        name: "Frost's Body",
        interaction: Some(NpcInteractionType::Dialog { dialog_id: 3995 }),
        level: 1,
    },
];

/// Spawn instance-specific NPCs for a world.
///
/// Called when an instanced space is first created. Returns the number of NPCs spawned.
pub fn spawn_npcs_for_world(world_name: &str, space_mgr: &mut SpaceManager) -> usize {
    let mut count = 0;

    for def in INSTANCE_SPAWN_DEFS {
        if def.world_name != world_name {
            continue;
        }

        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc(npc_id, def.world_name, def.position, def.direction) {
            Ok(space_id) => {
                if let Some(entity) = space_mgr.get_entity_mut(npc_id) {
                    entity.interaction_type = def.interaction.clone();
                    entity.npc_name = Some(def.name.to_string());
                    entity.level = def.level;
                }
                tracing::info!(
                    npc_id, space_id, world = def.world_name, name = def.name,
                    pos = ?def.position, "Spawned instance NPC"
                );
                count += 1;
            }
            Err(e) => {
                tracing::warn!(world = def.world_name, name = def.name, "Failed to spawn instance NPC: {e}");
            }
        }
    }

    count
}

/// Spawn all predefined NPCs into the space manager.
///
/// Returns the number of NPCs successfully spawned.
pub fn spawn_initial_npcs(space_mgr: &mut SpaceManager) -> usize {
    let mut count = 0;

    for def in SPAWN_DEFS {
        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc(npc_id, def.world_name, def.position, def.direction) {
            Ok(space_id) => {
                // Set interaction data on the entity
                if let Some(entity) = space_mgr.get_entity_mut(npc_id) {
                    entity.interaction_type = def.interaction.clone();
                    entity.npc_name = Some(def.name.to_string());
                    entity.level = def.level;
                }
                tracing::info!(
                    npc_id, space_id, world = def.world_name, name = def.name,
                    pos = ?def.position, "Spawned NPC"
                );
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    world = def.world_name,
                    pos = ?def.position,
                    "Failed to spawn NPC: {e}"
                );
            }
        }
    }

    tracing::info!(count, "Initial NPC population spawned");
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager_with_worlds() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
</Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Castle" />
</Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr
    }

    #[test]
    fn spawn_initial_npcs_creates_entities() {
        let mut mgr = make_manager_with_worlds();
        let count = spawn_initial_npcs(&mut mgr);
        assert_eq!(count, 5); // 3 in Agnos + 2 in Castle
    }

    #[test]
    fn spawned_npcs_have_correct_class_id() {
        let mut mgr = make_manager_with_worlds();
        spawn_initial_npcs(&mut mgr);

        // First NPC should have ID 100_000
        let npc = mgr.get_entity(100_000).unwrap();
        assert_eq!(npc.class_id, 0x04); // SGWMob
        assert!(!npc.is_player);
    }

    #[test]
    fn spawned_npcs_visible_in_aoi() {
        let mut mgr = make_manager_with_worlds();
        spawn_initial_npcs(&mut mgr);

        // Create a player near the Agnos NPCs
        mgr.create_entity(1, "Agnos", [30.0, 0.0, 50.0], [0.0; 3]).unwrap();
        mgr.connect_entity(1);

        // First AoI tick should detect the NPCs
        let events = mgr.compute_aoi_changes();
        let entered: Vec<_> = events.iter().filter(|e| {
            matches!(e, crate::cell::messages::CellToBaseMsg::EnteredAoI { .. })
        }).collect();

        // Player should see all 3 Agnos NPCs (they're within default 100m AoI)
        assert_eq!(entered.len(), 3);
    }

    #[test]
    fn npc_ids_are_sequential() {
        let mut mgr = make_manager_with_worlds();
        let id1 = mgr.allocate_npc_id();
        let id2 = mgr.allocate_npc_id();
        assert_eq!(id1, 100_000);
        assert_eq!(id2, 100_001);
    }

    #[test]
    fn spawned_npcs_have_level() {
        let mut mgr = make_manager_with_worlds();
        spawn_initial_npcs(&mut mgr);
        // Jaffa Warrior (3rd spawn in Agnos) should be level 2
        let npc = mgr.get_entity(100_002).unwrap();
        assert_eq!(npc.level, 2);
        // Castle Guard (5th spawn) should be level 4
        let npc = mgr.get_entity(100_004).unwrap();
        assert_eq!(npc.level, 4);
    }

    #[test]
    fn spawn_in_unknown_world_skipped() {
        let mut mgr = SpaceManager::new(1);
        // No worlds loaded
        let npc_id = mgr.allocate_npc_id();
        let result = mgr.spawn_npc(npc_id, "Nonexistent", [0.0; 3], [0.0; 3]);
        assert!(result.is_err());
    }
}
