//! UO-28: Jaffa Armor T1-T4 (templates 63835-63838, lists 63812-63815, spawns 63859-63863).
//! D-UO8 B (every tier), D-UO16 B (extra Common Area spawn), D-UO29 (price rule), D-UO35 (one bit).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_VENDOR_ARMOR`.
    const BIT: i64 = 8192;
    /// (template, moniker, list, tier)
    const TIERS: [(i32, i32, i32, i32); 4] = [
        (63835, 27228, 63812, 1),
        (63836, 27229, 63813, 2),
        (63837, 27230, 63814, 3),
        (63838, 27231, 63815, 4),
    ];
    const SPAWNS: [(i32, i32, &str); 5] = [
        (63859, 63835, "UnionOmega_JaffaArmorT1"),
        (63860, 63836, "UnionOmega_JaffaArmorT2"),
        (63861, 63837, "UnionOmega_JaffaArmorT3"),
        (63862, 63838, "UnionOmega_JaffaArmorT4"),
        (63863, 63835, "UnionOmega_JaffaArmorT1_Common"),
    ];

    #[tokio::test]
    async fn omega_jaffa_armor_templates_sell_one_tier_each() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _) in TIERS {
            assert_vendor_template(&pool, template, BIT, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_jaffa_armor_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for (_, _, list, tier) in TIERS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Armor, Some(tier));
        }
    }

    #[tokio::test]
    async fn omega_jaffa_armor_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
