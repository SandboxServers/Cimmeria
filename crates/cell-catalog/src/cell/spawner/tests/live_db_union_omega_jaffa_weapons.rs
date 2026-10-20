//! UO-12: Jaffa Weapons T1-T4 (templates 63831-63834, lists 63808-63811, spawns 63853-63857).
//! D-UO8 B (every tier), D-UO16 B (extra Common Area spawn), D-UO29 (price rule), D-UO35 (one bit).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_VENDOR_WEAPONS`.
    const BIT: i64 = 16384;
    /// (template, moniker, list, tier)
    const TIERS: [(i32, i32, i32, i32); 4] = [
        (63831, 27224, 63808, 1),
        (63832, 27225, 63809, 2),
        (63833, 27226, 63810, 3),
        (63834, 27227, 63811, 4),
    ];
    const SPAWNS: [(i32, i32, &str); 5] = [
        (63853, 63831, "UnionOmega_JaffaWeaponsT1"),
        (63854, 63832, "UnionOmega_JaffaWeaponsT2"),
        (63855, 63833, "UnionOmega_JaffaWeaponsT3"),
        (63856, 63834, "UnionOmega_JaffaWeaponsT4"),
        (63857, 63831, "UnionOmega_JaffaWeaponsT1_Common"),
    ];

    #[tokio::test]
    async fn omega_jaffa_weapons_templates_sell_one_tier_each() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _tier) in TIERS {
            assert_vendor_template(&pool, template, BIT, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_jaffa_weapons_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for (_template, _moniker, list, tier) in TIERS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Weapon, Some(tier));
        }
    }

    #[tokio::test]
    async fn omega_jaffa_weapons_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
