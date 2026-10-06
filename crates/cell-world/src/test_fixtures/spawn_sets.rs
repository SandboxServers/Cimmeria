//! Two switchable spawn sets of one kind, for the DA-10 Visual NPC Lineup
//! tests: built through [`partition_spawn_sets`], the loader's own path, so
//! a fixture world holds them exactly as a booted cell does.

use cimmeria_entity::cell_entity::VaultScope;

use super::npc_spawn_record;
use crate::cell::space_manager::{partition_spawn_sets, SpaceManager};
use crate::cell::spawner::{SpawnRecord, SpawnSetDef};

/// The kind both fixture sets share (the seeded lineup's `type`).
pub const LINEUP_KIND: &str = "visual_lineup";
/// The fixture sets' `world_id`.
pub const LINEUP_WORLD_ID: i32 = 1300;
/// Set ids and their member counts.
pub const LINEUP_SET_A: i32 = 1301;
pub const LINEUP_SET_A_SIZE: usize = 3;
pub const LINEUP_SET_B: i32 = 1302;
pub const LINEUP_SET_B_SIZE: usize = 2;
/// The name chat lines show for set A / set B.
pub const LINEUP_SET_A_NAME: &str = "Lineup A";
pub const LINEUP_SET_B_NAME: &str = "Lineup B";

/// Install the two fixture sets on `mgr`, members in `world`. Returns the
/// startup records left over: one ordinary NPC, which must still spawn at
/// boot.
pub fn install_lineup_sets(mgr: &mut SpaceManager, world: &str) -> Vec<SpawnRecord> {
    let member = |spawn_id: i32, x: f32| SpawnRecord {
        spawn_id,
        template_id: 1400 + spawn_id,
        ..npc_spawn_record(world, [x, 0.0, 5.0], 0, VaultScope::Personal)
    };
    let mut records = vec![
        member(1, 1.0),
        member(2, 2.0),
        member(3, 3.0),
        member(4, 4.0),
        member(5, 5.0),
        member(9, 9.0),
    ];
    let def = |set_id: i32, name: &str, spawn_ids: Vec<i32>| SpawnSetDef {
        set_id,
        name: name.to_string(),
        kind: LINEUP_KIND.to_string(),
        world_id: LINEUP_WORLD_ID,
        spawn_ids,
    };
    mgr.spawn_sets = partition_spawn_sets(
        &mut records,
        vec![
            def(LINEUP_SET_A, LINEUP_SET_A_NAME, vec![1, 2, 3]),
            def(LINEUP_SET_B, LINEUP_SET_B_NAME, vec![4, 5]),
        ],
    );
    records
}
