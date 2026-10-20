//! UO-11: Human Weapons T1-T4 (templates 63812-63815, lists 63800-63803, spawns 63841-63846).
//! D-UO8 B (every tier), D-UO16 B (extra Common Area spawns), D-UO29 (price rule), D-UO35 (one bit).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_VENDOR_WEAPONS`.
    const BIT: i64 = 16384;
    /// (template, moniker, list, tier)
    const TIERS: [(i32, i32, i32, i32); 4] = [
        (63812, 27240, 63800, 1),
        (63813, 27241, 63801, 2),
        (63814, 27242, 63802, 3),
        (63815, 27243, 63803, 4),
    ];
    /// (spawn, template, tag)
    const SPAWNS: [(i32, i32, &str); 6] = [
        (63841, 63812, "UnionOmega_HumanWeaponsT1"),
        (63842, 63813, "UnionOmega_HumanWeaponsT2"),
        (63843, 63814, "UnionOmega_HumanWeaponsT3"),
        (63844, 63815, "UnionOmega_HumanWeaponsT4"),
        (63845, 63812, "UnionOmega_HumanWeaponsT1_Common"),
        (63846, 63815, "UnionOmega_HumanWeaponsT4_Common"),
    ];

    #[tokio::test]
    async fn omega_human_weapons_templates_sell_one_tier_each() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _) in TIERS {
            assert_vendor_template(&pool, template, BIT, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_human_weapons_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for (_, _, list, tier) in TIERS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Weapon, Some(tier));
        }
    }

    #[tokio::test]
    async fn omega_human_weapons_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
