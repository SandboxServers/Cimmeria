//! Live-DB guards for the ability range unit (#919).
//!
//! `resources.abilities.min_range` / `max_range` are UE3 units, 100 per
//! BigWorld metre, like the client's cooked `MaxRange` (1652 Jaffa: Double
//! Blast ships `MaxRange="3000"` in the 2009 `CookedDataAbilities.pak`).
//! `load_ability_defs` must hand the server metres. Evidence:
//! `docs/reverse-engineering/findings/ability-resolution-pipeline.md`
//! § "Range units".
mod live_db {
    use crate::cell::spawner::load_ability_defs;
    use crate::test_support::require_db_or_skip;

    /// Seeded rows and the metres they must load as.
    const SEEDED: [(i32, &str, f32, f32); 4] = [
        (1652, "Jaffa: Double Blast (3000)", 0.0, 30.0),
        (1653, "Lo'taur: Heal Health (800)", 0.0, 8.0),
        (1205, "Turret Attack: Cone (300 / 3000)", 3.0, 30.0),
        (1259, "Sticky Bomb (2500)", 0.0, 25.0),
    ];

    /// Named abilities load with their ranges in metres. Revert proof: with
    /// the loader passing the column through, 1652 loads as 3000.
    #[tokio::test]
    async fn seeded_ability_ranges_load_in_metres() {
        let pool = require_db_or_skip!();
        let defs = load_ability_defs(&pool).await.expect("ability defs load");
        for (id, what, min_m, max_m) in SEEDED {
            let def = defs
                .get(&id)
                .unwrap_or_else(|| panic!("ability {id} ({what}) must be seeded"));
            assert_eq!(def.min_range, min_m, "{id} {what}: min_range in metres");
            assert_eq!(def.max_range, max_m, "{id} {what}: max_range in metres");
        }
    }

    /// No loaded ability reaches past 100 m (the seed's largest value is
    /// 10000 UE3 units), and none has a minimum above its maximum.
    #[tokio::test]
    async fn no_loaded_ability_range_is_in_raw_ue3_units() {
        let pool = require_db_or_skip!();
        let defs = load_ability_defs(&pool).await.expect("ability defs load");
        for def in defs.values() {
            assert!(
                (0.0..=100.0).contains(&def.max_range),
                "ability {} ({}) max_range {} m: raw UE3 units leaked past the loader",
                def.ability_id,
                def.name,
                def.max_range
            );
            assert!(
                def.max_range == 0.0 || def.min_range <= def.max_range,
                "ability {} ({}): min_range {} m above max_range {} m",
                def.ability_id,
                def.name,
                def.min_range,
                def.max_range
            );
        }
    }
}
