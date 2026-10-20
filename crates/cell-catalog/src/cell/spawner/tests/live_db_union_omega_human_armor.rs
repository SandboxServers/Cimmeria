//! UO-27: Human Armor T1-T4 (templates 63816-63819, lists 63804-63807, spawns 63847-63852).
//! D-UO8 B (every tier), D-UO16 B (extra Common Area spawns), D-UO29 (price rule), D-UO35 (one bit).

mod live_db {
    use super::super::live_db_union_omega_vendor_common::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_VENDOR_ARMOR`.
    const BIT: i64 = 8192;
    /// (template, moniker, list, tier)
    const TIERS: [(i32, i32, i32, i32); 4] = [
        (63816, 27244, 63804, 1),
        (63817, 27245, 63805, 2),
        (63818, 27246, 63806, 3),
        (63819, 27247, 63807, 4),
    ];
    const SPAWNS: [(i32, i32, &str); 6] = [
        (63847, 63816, "UnionOmega_HumanArmorT1"),
        (63848, 63817, "UnionOmega_HumanArmorT2"),
        (63849, 63818, "UnionOmega_HumanArmorT3"),
        (63850, 63819, "UnionOmega_HumanArmorT4"),
        (63851, 63816, "UnionOmega_HumanArmorT1_Common"),
        (63852, 63819, "UnionOmega_HumanArmorT4_Common"),
    ];

    #[tokio::test]
    async fn omega_human_armor_templates_sell_one_tier_each() {
        let pool = require_db_or_skip!();
        for (template, moniker, list, _tier) in TIERS {
            assert_vendor_template(&pool, template, BIT, moniker, list).await;
        }
    }

    #[tokio::test]
    async fn omega_human_armor_lists_follow_d_uo29() {
        let pool = require_db_or_skip!();
        for (_template, _moniker, list, tier) in TIERS {
            let rows = load_list(&pool, list).await;
            assert_list(list, &rows, Price::Armor, Some(tier));
        }
    }

    #[tokio::test]
    async fn omega_human_armor_stand_clear_in_the_hub() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &SPAWNS).await;
    }

    #[tokio::test]
    async fn omega_human_armor_vendors_are_unarmed() {
        let pool = require_db_or_skip!();
        for (template, _moniker, _list, _tier) in TIERS {
            let components: String = sqlx::query_scalar(
                "SELECT array_to_string(components, ',') FROM resources.entity_templates \
                  WHERE template_id = $1",
            )
            .bind(template)
            .fetch_one(&pool)
            .await
            .expect("entity_templates query must succeed");
            assert!(
                !components.contains("WP-Human."),
                "template {template} must hold no weapon (D-UO37): {components}"
            );
        }
    }
}
