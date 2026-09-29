//! Live-DB guard: every spawn in the Castle_CellBlock stasis room (the debug
//! hub, `docs/content/debug-hub.md`) holds position by flag.
//!
//! The hub's service NPCs, crafting stations and loot crate never walk, and
//! the navmesh covers the room only in patches. A mobile spawn there fails
//! `find_path`'s start box, so the spawner posted a `spawn_off_mesh` WARN for
//! it on every instance of that per-login world: vendor 400, pet trainer 450
//! and Banker 470 filled the Discord errors channel (colo, 2026-09-29).
//! `is_stationary` also keeps the authored floor height and turns cover off,
//! both right for an NPC that stands at a counter.
//!
//! The guard covers the whole room rather than a list of spawn ids, so the
//! next campaign's hub NPC is caught too. Proven to fail with the 18 hub rows'
//! `is_stationary` reverted.

mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The stasis room (point set 2032), the hub's floor.
    const ROOM: &str = "Castle_Cellblock.Region1";

    /// Every hub spawn today: 400-405, 410-414, 430-431, 450, 470-472, 490.
    const HUB_SPAWNS: usize = 18;

    #[tokio::test]
    async fn every_stasis_room_spawn_is_stationary() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");
        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));

        let in_room: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| {
                r.world_name == "Castle_CellBlock" && region_contains_xz(&room.points, r.x, r.z)
            })
            .collect();
        assert!(
            in_room.len() >= HUB_SPAWNS,
            "the hub's {HUB_SPAWNS} spawns must stand in {ROOM}: {:?}",
            in_room.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
        );
        let mobile: Vec<(i32, Option<&str>)> = in_room
            .iter()
            .filter(|r| !r.is_stationary)
            .map(|r| (r.spawn_id, r.tag.as_deref()))
            .collect();
        assert!(
            mobile.is_empty(),
            "every spawn in the stasis room must be is_stationary (service NPCs never \
             path, and a mobile one here warns spawn_off_mesh on every instance): {mobile:?}"
        );
    }
}
