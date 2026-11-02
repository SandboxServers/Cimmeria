//! Live-DB guard: no starter-world vendor or trainer opens onto an empty or
//! dangling list (SS-02).
//!
//! A vendor template whose buy, sell, repair or recharge list names no rows
//! opens an empty store, and nothing says so. A list row that names a design
//! missing from `resources.items`, or a trainer list row that names an ability
//! missing from `resources.abilities`, puts something the client cannot draw
//! in the window.
//!
//! Scoped to the templates spawned in the starter worlds, so a vendor added to
//! one of them is covered without a new list here. Fail-closed: no allow-list
//! and no skip.
//!
//! Revert proof: add an `item_lists` row 999999 with no items (the foreign key
//! rejects a missing list), point template 415's `buy_item_list`
//! (entity_templates.sql line 1099) at it, and test 1 fails naming template 415
//! as an empty list.
mod live_db {
    use crate::test_support::require_db_or_skip;

    /// The starter worlds: 12 Castle_CellBlock, 8 Castle, 57 and 68-70 Harset,
    /// 19 Tollana, 58 SGC_W1, 61 and 62 Dakara_E1, 18 and 80 Omega, 23 Beta
    /// Site E1, 1300 the Debug Area.
    const STARTER_WORLDS: [i32; 14] = [12, 8, 57, 68, 69, 70, 19, 58, 61, 62, 18, 80, 23, 1300];

    /// The vendor lists of every template spawned in a starter world, one
    /// row per (template, column, list). Bound as `$1`: the world ids.
    const VENDOR_LISTS: &str = "WITH spawned AS ( \
            SELECT DISTINCT t.template_id, t.buy_item_list, t.sell_item_list, \
                   t.repair_item_list, t.recharge_item_list \
              FROM resources.spawnlist s \
              JOIN resources.entity_templates t ON t.template_id = s.template_id \
             WHERE s.world_id = ANY($1) \
        ), vendor_lists AS ( \
            SELECT template_id, 'buy_item_list'::text AS column_name, buy_item_list AS list_id \
              FROM spawned WHERE buy_item_list IS NOT NULL \
            UNION ALL \
            SELECT template_id, 'sell_item_list'::text, sell_item_list \
              FROM spawned WHERE sell_item_list IS NOT NULL \
            UNION ALL \
            SELECT template_id, 'repair_item_list'::text, repair_item_list \
              FROM spawned WHERE repair_item_list IS NOT NULL \
            UNION ALL \
            SELECT template_id, 'recharge_item_list'::text, recharge_item_list \
              FROM spawned WHERE recharge_item_list IS NOT NULL \
        )";

    /// The trainer lists of every template spawned in a starter world, one
    /// row per (template, list). Bound as `$1`: the world ids.
    const TRAINER_LISTS: &str = "WITH trainer_lists AS ( \
            SELECT DISTINCT t.template_id, t.trainer_ability_list_id AS list_id \
              FROM resources.spawnlist s \
              JOIN resources.entity_templates t ON t.template_id = s.template_id \
             WHERE s.world_id = ANY($1) AND t.trainer_ability_list_id IS NOT NULL \
        )";

    /// Test 1: every vendor list of a starter-world template has rows, and
    /// every row names a real item.
    #[tokio::test]
    async fn live_db_starter_vendor_lists_are_stocked() {
        let pool = require_db_or_skip!();
        let misses: Vec<(i32, String, i32, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{VENDOR_LISTS} \
             SELECT vl.template_id, vl.column_name, vl.list_id, 'empty list'::text \
               FROM vendor_lists vl \
              WHERE NOT EXISTS (SELECT 1 FROM resources.item_list_items ili \
                                 WHERE ili.item_list_id = vl.list_id) \
             UNION ALL \
             SELECT DISTINCT vl.template_id, vl.column_name, vl.list_id, \
                    format('dangling design %s', ili.design_id) \
               FROM vendor_lists vl \
               JOIN resources.item_list_items ili ON ili.item_list_id = vl.list_id \
              WHERE NOT EXISTS (SELECT 1 FROM resources.items i \
                                 WHERE i.item_id = ili.design_id) \
              ORDER BY 1, 2, 3, 4"
        )))
        .bind(STARTER_WORLDS.to_vec())
        .fetch_all(&pool)
        .await
        .expect("vendor list query");
        assert!(
            misses.is_empty(),
            "starter-world vendor lists that are empty or dangling, as \
             (template_id, column, list_id, problem): {misses:?}"
        );

        // Control: the query sees the vendors the starter worlds spawn, so an
        // empty answer is not an empty join.
        let vendors: Vec<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "{VENDOR_LISTS} SELECT DISTINCT template_id FROM vendor_lists ORDER BY 1"
        )))
        .bind(STARTER_WORLDS.to_vec())
        .fetch_all(&pool)
        .await
        .expect("vendor control query");
        for template in [300, 314, 415, 1302] {
            assert!(
                vendors.contains(&template),
                "template {template} must be a spawned starter-world vendor, found {vendors:?}"
            );
        }
    }

    /// Test 2: every trainer list of a starter-world template has abilities,
    /// and every ability names a real ability.
    #[tokio::test]
    async fn live_db_starter_trainer_lists_are_stocked() {
        let pool = require_db_or_skip!();
        let misses: Vec<(i32, String, i32, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{TRAINER_LISTS} \
             SELECT tl.template_id, 'trainer_ability_list_id'::text, tl.list_id, 'empty list'::text \
               FROM trainer_lists tl \
              WHERE NOT EXISTS (SELECT 1 FROM resources.trainer_abilities ta \
                                 WHERE ta.list_id = tl.list_id) \
             UNION ALL \
             SELECT DISTINCT tl.template_id, 'trainer_ability_list_id'::text, tl.list_id, \
                    format('dangling ability %s', ta.ability_id) \
               FROM trainer_lists tl \
               JOIN resources.trainer_abilities ta ON ta.list_id = tl.list_id \
              WHERE NOT EXISTS (SELECT 1 FROM resources.abilities a \
                                 WHERE a.ability_id = ta.ability_id) \
              ORDER BY 1, 3, 4"
        )))
        .bind(STARTER_WORLDS.to_vec())
        .fetch_all(&pool)
        .await
        .expect("trainer list query");
        assert!(
            misses.is_empty(),
            "starter-world trainer lists that are empty or dangling, as \
             (template_id, column, list_id, problem): {misses:?}"
        );

        // Control: the Archetype Skills Trainer (301) is spawned in the hub.
        let trainers: Vec<i32> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "{TRAINER_LISTS} SELECT DISTINCT template_id FROM trainer_lists ORDER BY 1"
        )))
        .bind(STARTER_WORLDS.to_vec())
        .fetch_all(&pool)
        .await
        .expect("trainer control query");
        assert!(
            trainers.contains(&301),
            "template 301 must be a spawned starter-world trainer, found {trainers:?}"
        );
    }
}
