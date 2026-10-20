//! UO-31: the five class trainers (templates and spawns 63804-63808, D-UO17 c,
//! D-UO30). Trainer list 1 offers each player the tree of their own archetype,
//! so the five trainers differ only in body, moniker and place.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a trainer template that lost its trainer bit or its list 1 link, or that
//!   picked up a shop list (the click then opens a vendor or nothing);
//! * list 1 offering no tree to one trainer's archetype (the window opens bare
//!   and the server WARNs `trainer_empty_offering`);
//! * a trainer spawn that stands on another trainer or off its wing.

mod live_db {
    use sqlx::Row;

    use super::super::live_db_union_omega_vendor_common::*;
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `INT_Trainer`.
    const INT_TRAINER: i64 = 128;
    const LIST: i32 = 1;
    /// (template = spawn, moniker, tag, archetype label the trainer serves)
    const TRAINERS: [(i32, i32, &str, &str); 5] = [
        (
            63804,
            7988,
            "UnionOmega_SoldierTrainer",
            "ARCHETYPE_Soldier",
        ),
        (
            63805,
            7989,
            "UnionOmega_ScientistTrainer",
            "ARCHETYPE_Scientist",
        ),
        (
            63806,
            7992,
            "UnionOmega_CommandoTrainer",
            "ARCHETYPE_Commando",
        ),
        (
            63807,
            7993,
            "UnionOmega_ArchaeologistTrainer",
            "ARCHETYPE_Archeologist",
        ),
        (63808, 7990, "UnionOmega_JaffaTrainer", "ARCHETYPE_Sholva"),
    ];

    async fn archetype_index(pool: &sqlx::PgPool, label: &str) -> i32 {
        sqlx::query_scalar(
            "SELECT array_position(enum_range(NULL::resources.\"EArchetype\"), \
                    $1::resources.\"EArchetype\") - 1",
        )
        .bind(label)
        .fetch_one(pool)
        .await
        .expect("EArchetype index query")
    }

    /// Each class trainer is a placed trainer NPC: trainer bit, list 1, a
    /// basic-skills moniker that resolves to text, and no shop, loot or
    /// ability set.
    #[tokio::test]
    async fn omega_class_trainers_carry_trainer_columns() {
        let pool = require_db_or_skip!();
        for (template, moniker, _, _) in TRAINERS {
            let row = sqlx::query(
                "SELECT t.class, t.faction, t.interaction_type, t.name_id, \
                        t.trainer_ability_list_id, t.buy_item_list, t.sell_item_list, \
                        t.repair_item_list, t.recharge_item_list, t.loot_table_id, \
                        t.ability_set_id, x.text \
                   FROM resources.entity_templates t \
                   LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
                  WHERE t.template_id = $1",
            )
            .bind(template)
            .fetch_optional(&pool)
            .await
            .expect("entity_templates query")
            .unwrap_or_else(|| panic!("template {template} must be seeded"));

            let got = (
                row.get::<String, _>("class"),
                row.get::<Option<i32>, _>("faction"),
                row.get::<i64, _>("interaction_type"),
                row.get::<Option<i32>, _>("name_id"),
                row.get::<Option<i32>, _>("trainer_ability_list_id"),
                row.get::<Option<i32>, _>("buy_item_list"),
                row.get::<Option<i32>, _>("sell_item_list"),
                row.get::<Option<i32>, _>("repair_item_list"),
                row.get::<Option<i32>, _>("recharge_item_list"),
                row.get::<Option<i32>, _>("loot_table_id"),
                row.get::<Option<i32>, _>("ability_set_id"),
            );
            assert_eq!(
                got,
                (
                    "mob".to_string(),
                    Some(1),
                    INT_TRAINER,
                    Some(moniker),
                    Some(LIST),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                "template {template} trainer columns"
            );
            let name: Option<String> = row.get("text");
            assert!(
                name.as_deref().is_some_and(|n| !n.trim().is_empty()),
                "template {template}: name_id {moniker} must resolve to non-empty text, got {name:?}"
            );
        }

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        for (template, _, _, _) in TRAINERS {
            assert_eq!(
                lists.get(&template),
                Some(&LIST),
                "the startup loader must map template {template} to list {LIST}"
            );
        }
    }

    /// Trainer list 1 holds a non-empty tree for the archetype of every class
    /// trainer. An empty offer WARNs `trainer_empty_offering` and the window
    /// opens bare.
    #[tokio::test]
    async fn omega_trainer_list_offers_each_class_a_tree() {
        let pool = require_db_or_skip!();
        let offered = load_trainer_abilities(&pool)
            .await
            .expect("load_trainer_abilities must succeed");
        for (template, _, _, label) in TRAINERS {
            let archetype = archetype_index(&pool, label).await;
            let abilities = offered.get(&(LIST, archetype));
            assert!(
                abilities.is_some_and(|a| !a.is_empty()),
                "template {template}: list {LIST} must offer a tree to {label} (index {archetype}), got {abilities:?}"
            );
        }
    }

    /// Each class trainer stands alone in its wing: stationary, clear of every
    /// other Omega spawn, and clear of every Omega respawner.
    #[tokio::test]
    async fn omega_class_trainers_stand_in_their_wings() {
        let pool = require_db_or_skip!();
        assert_spawns(&pool, &TRAINERS.map(|(t, _, tag, _)| (t, t, tag))).await;
    }
}
