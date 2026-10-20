//! UO-29: the crafting vendors (templates 63820-63827, 63839-63842; lists 63817-63820).
//!
//! D-UO8 B (the owner chose every crafting specialty), D-UO27 (one list per
//! specialty, shared by the three factions' vendors), D-UO29 (the crafting
//! price rule) and D-UO35 (one vendor bit per template).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    const BIO: i64 = 262144;
    const ELEC: i64 = 2097152;
    const POWER: i64 = 524288;
    const MAT: i64 = 1048576;
    /// (template, moniker, bit, list)
    const VENDORS: [(i32, i32, i64, i32); 12] = [
        (63820, 27248, BIO, 63817),
        (63821, 27249, ELEC, 63818),
        (63822, 27250, POWER, 63819),
        (63823, 27251, MAT, 63820),
        (63824, 27236, BIO, 63817),
        (63825, 27237, ELEC, 63818),
        (63826, 27238, POWER, 63819),
        (63827, 27239, MAT, 63820),
        (63839, 27232, BIO, 63817),
        (63840, 27233, ELEC, 63818),
        (63841, 27234, POWER, 63819),
        (63842, 27235, MAT, 63820),
    ];
    const LISTS: [i32; 4] = [63817, 63818, 63819, 63820];
    const SPAWNS: [(i32, i32, &str); 12] = [
        (63960, 63820, "UnionOmega_HumanCraftingBiomed"),
        (63961, 63821, "UnionOmega_HumanCraftingElec"),
        (63962, 63822, "UnionOmega_HumanCraftingPowerSys"),
        (63963, 63823, "UnionOmega_HumanCraftingMat"),
        (63964, 63824, "UnionOmega_CommonCraftingBiomed"),
        (63965, 63825, "UnionOmega_CommonCraftingElec"),
        (63966, 63826, "UnionOmega_CommonCraftingPowerSys"),
        (63967, 63827, "UnionOmega_CommonCraftingMat"),
        (63968, 63839, "UnionOmega_JaffaCraftingBiomed"),
        (63969, 63840, "UnionOmega_JaffaCraftingElec"),
        (63970, 63841, "UnionOmega_JaffaCraftingPowerSys"),
        (63971, 63842, "UnionOmega_JaffaCraftingMat"),
    ];

    #[tokio::test]
    async fn omega_crafting_templates_carry_their_specialty() {
        let pool = require_db_or_skip!();
        for (template, moniker, bit, list) in VENDORS {
            assert_vendor_template(&pool, template, bit, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_crafting_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for list in LISTS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Crafting, None);
        }
    }

    #[tokio::test]
    async fn omega_crafting_vendors_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
