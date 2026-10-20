//! Live-DB guard: the Prison Boot Control (spawn 284, cellblock-boot-lock
//! D-BL1) stands where the boot-lock hold's anchor is.
//!
//! Chain 1023's `movement_lock` hold pins the player to
//! `BOOT_ANCHOR_XZ` (its `at`, pinned in `mission_689_boot_lock.rs`) within the
//! hold's `max_distance`. The control is the prop the player right-clicks, so
//! it must stand within that same radius of the anchor, and it must stay
//! `is_stationary` so the room's navmesh gap does not matter.

mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The boot-lock hold's anchor, horizontal (x, z). Must match chain 1023's
    /// `at` (`[-334.231, 73.472, -228.026]`) in
    /// `db/resources/Content/Seed/castle_cellblock_chains.sql`.
    const BOOT_ANCHOR_XZ: [f32; 2] = [-334.231, -228.026];

    /// The hold's `max_distance` (chain 1023). The control must stand inside it.
    const HOLD_MAX_DISTANCE: f32 = 3.0;

    #[tokio::test]
    async fn the_boot_control_stands_by_the_hold_anchor() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        let controls: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.tag.as_deref() == Some("Cellblock_PrisonBootControl"))
            .collect();
        assert_eq!(
            controls.len(),
            1,
            "exactly one spawn must carry the Prison Boot Control tag; got {:?}",
            controls.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
        );
        let control = controls[0];

        assert_eq!(control.world_name, "Castle_CellBlock");
        assert_eq!(control.spawn_id, 284);
        assert_eq!(
            control.template_id, 1250,
            "the boot control is template 1250"
        );
        assert!(
            control.is_stationary,
            "the boot control must stay is_stationary (D-BL1)"
        );

        let dx = control.x - BOOT_ANCHOR_XZ[0];
        let dz = control.z - BOOT_ANCHOR_XZ[1];
        let distance = (dx * dx + dz * dz).sqrt();
        assert!(
            distance <= HOLD_MAX_DISTANCE,
            "the boot control ({}, {}) must stand within the hold's {HOLD_MAX_DISTANCE} u of \
             the anchor {BOOT_ANCHOR_XZ:?}, is {distance:.2} u away",
            control.x,
            control.z
        );
    }
}
