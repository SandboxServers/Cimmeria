//! UO-40: the Asgard Armor T1-T4 and Asgard crafting vendors (templates 63847-63854,
//! lists 63825-63828, spawns 63828-63835).
//!
//! D-UO8 B (the owner chose every tier and every crafting specialty), D-UO27 (the
//! Asgard crafting vendors share the lists 63817-63820), D-UO29 (the armor price
//! rule), D-UO35 (one vendor bit per template) and D-UO37 (the Asgard body).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;
    use sqlx::Row;

    const ARMOR: i64 = 8192;
    const BIO: i64 = 262144;
    const ELEC: i64 = 2097152;
    const POWER: i64 = 524288;
    const MAT: i64 = 1048576;
    const ASGARD_BODY: &str = "BS_Asgard.BS_Asgard";
    /// (template, moniker, list, tier)
    const ARMOR_VENDORS: [(i32, i32, i32, i32); 4] = [
        (63847, 27256, 63825, 1),
        (63848, 27257, 63826, 2),
        (63849, 27258, 63827, 3),
        (63850, 27259, 63828, 4),
    ];
    /// (template, moniker, bit, shared list)
    const CRAFTING: [(i32, i32, i64, i32); 4] = [
        (63851, 27260, BIO, 63817),
        (63852, 27261, ELEC, 63818),
        (63853, 27262, POWER, 63819),
        (63854, 27263, MAT, 63820),
    ];
    const SPAWNS: [(i32, i32, &str); 8] = [
        (63828, 63847, "UnionOmega_AsgardArmorT1"),
        (63829, 63848, "UnionOmega_AsgardArmorT2"),
        (63830, 63849, "UnionOmega_AsgardArmorT3"),
        (63831, 63850, "UnionOmega_AsgardArmorT4"),
        (63832, 63851, "UnionOmega_AsgardCraftingBiomed"),
        (63833, 63852, "UnionOmega_AsgardCraftingElec"),
        (63834, 63853, "UnionOmega_AsgardCraftingPowerSys"),
        (63835, 63854, "UnionOmega_AsgardCraftingMat"),
    ];

    #[tokio::test]
    async fn omega_asgard_armor_carries_the_armor_bit() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _) in ARMOR_VENDORS {
            assert_vendor_template(&pool, template, ARMOR, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_asgard_crafting_uses_the_shared_lists() {
        let pool = require_db_or_skip!();
        for (template, moniker, bit, list) in CRAFTING {
            assert_vendor_template(&pool, template, bit, moniker, list).await;
            assert!(
                !load_list(&pool, list).await.is_empty(),
                "template {template} buys from shared list {list}, which has no rows"
            );
        }
    }

    #[tokio::test]
    async fn omega_asgard_vendors_wear_the_asgard_body() {
        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT template_id, body_set, skin_tint FROM resources.entity_templates \
             WHERE template_id BETWEEN 63847 AND 63854 ORDER BY template_id",
        )
        .fetch_all(&pool)
        .await
        .expect("entity_templates body query");
        assert_eq!(
            rows.len(),
            8,
            "expected the 8 Asgard vendor templates 63847-63854, found {}",
            rows.len()
        );
        for row in rows {
            let template: i32 = row.get("template_id");
            let body: Option<String> = row.get("body_set");
            let tint: Option<i64> = row.get("skin_tint");
            assert_eq!(
                (body.as_deref(), tint),
                (Some(ASGARD_BODY), Some(0)),
                "template {template} must wear {ASGARD_BODY} with skin_tint 0"
            );
        }
    }

    #[tokio::test]
    async fn omega_asgard_armor_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for (_, _, list, tier) in ARMOR_VENDORS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Armor, Some(tier));
        }
    }

    #[tokio::test]
    async fn omega_asgard_vendors_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }
}
