//! Live-DB guards for the weapon reach a `UseWeaponRange` ability reads
//! (#1017). The columns are metres and load unconverted, and weapons with
//! no magazine (Jaffa staffs, clip 0) are included.
mod live_db {
    use crate::cell::spawner::load_weapon_ranges;
    use crate::test_support::require_db_or_skip;

    /// Seeded weapons: (item_id, name, min/max ranged, min/max melee).
    const SEEDED: [(i32, &str, [f32; 4]); 4] = [
        (55, "SI 3 9mm Pistol", [0.0, 20.0, 0.0, 2.0]),
        (21, "SGHC 6 SMG", [2.0, 30.0, 0.0, 2.0]),
        (3287, "SR1 .50-Cal Rifle", [2.0, 40.0, 0.0, 2.0]),
        (2797, "Serpent Staff (clip 0)", [3.0, 30.0, 0.0, 3.0]),
    ];

    #[tokio::test]
    async fn seeded_weapon_ranges_load_in_metres() {
        let pool = require_db_or_skip!();
        let map = load_weapon_ranges(&pool).await.expect("weapon ranges load");
        for (id, what, [min_r, max_r, min_m, max_m]) in SEEDED {
            let w = map
                .get(&id)
                .unwrap_or_else(|| panic!("item {id} ({what}) must have a reach"));
            assert_eq!(
                (w.min_ranged, w.max_ranged, w.min_melee, w.max_melee),
                (min_r, max_r, min_m, max_m),
                "{id} {what}"
            );
        }
        // Metres, not UE3 units: no shipped weapon reaches past 50 m.
        assert!(
            map.values()
                .all(|w| w.max_ranged <= 50.0 && w.max_melee <= 50.0),
            "a weapon range above 50 m means the column was scaled"
        );
    }
}
