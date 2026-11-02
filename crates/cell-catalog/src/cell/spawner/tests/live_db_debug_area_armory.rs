//! Live-DB guards for the Debug Area Armory (DR-03b, DR-04,
//! `docs/analysis/debug-armory`): world 1300, templates 1610-1659, spawns
//! 14110-14299, buy lists 1310-1348 and the shared sell list 1349.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * an Armory template the seed defines that is not spawned exactly once in
//!   world 1300 inside the Armory's spawn block;
//! * a buy list with no rows or more than the store window's row cap, a row
//!   selling a design that does not exist, or a price that is not
//!   max(1, the highest sell-list unit price for that design);
//! * an archetype tree ability that needs a weapon moniker and has no Armory
//!   row carrying one, other than the four weapon-requirement audit FAILs;
//! * an equipment slot (containers 4-14) and tier in `resources.items` with
//!   no Armory row.
//! * a sell-list row whose design the sell path refuses (items.flags without
//!   1024, or with 4).
//!
//! Revert proofs (the seed edits stay local; reload after each):
//! * `item_lists_debug_area_armory.sql`: row 131000 (list 1310, design 3128)
//!   at `naquadah` 0; `live_db_armory_lists_stocked_and_priced` fails naming
//!   that row.
//! * the five Armory rows carrying ability 612 (moniker 1115110575; list 1311,
//!   items 131032-131036), deleted; `live_db_armory_covers_tree_weapons` fails
//!   naming 612. No covered ability has exactly one carrier (the minimum is 5),
//!   so the proof removes all of them.
//! * a third row, `INSERT INTO item_list_items (item_id, item_list_id, design_id,
//!   quantity, naquadah) VALUES (131300, 1349, 2699, 1, 0);`, added to the seed and
//!   reloaded: `live_db_armory_sell_list_is_sellable` fails naming (1349, 2699).
mod live_db {
    use crate::test_support::require_db_or_skip;

    const WORLD_ID: i32 = 1300;
    const FIRST_SPAWN: i32 = 14110;
    const LAST_SPAWN: i32 = 14299;

    /// The store window's row cap per vendor: 19, the largest buy list proven
    /// to open (list 1300). `evidence/client-limits.md` (DR-02) found no client
    /// cap, so this is a generator choice (`debug_armory.py`'s `DEFAULT_CAP`).
    const ARMORY_ROW_CAP: i64 = 19;

    /// Tree abilities the weapon-requirement audit lists as FAIL (no shipped
    /// weapon fits; weapon-requirement-audit.md:327-339). None has an Armory
    /// carrier, so the test expects exactly these misses.
    // 997 Full Mag: Dart Pistol, needs ITEM_Dart_Rifle (3257416555).
    // 1246 Stealthed Strike, needs ITEM_Melee (3756430796).
    // 1250 Escape, needs ITEM_Stealth_{Chest,Feet,Hands,Head,Legs} (armor).
    // 1355 Lethal Strike, needs ITEM_Melee (3756430796).
    const KNOWN_UNCOVERED: [i32; 4] = [997, 1246, 1250, 1355];

    /// Every Armory template the seed defines is spawned exactly once in
    /// world 1300, inside spawns 14110-14299. At least one exists (control).
    #[tokio::test]
    async fn live_db_armory_vendors_spawned() {
        let pool = require_db_or_skip!();
        let counts: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT t.template_id, COUNT(s.spawn_id) \
               FROM resources.entity_templates t \
               LEFT JOIN resources.spawnlist s ON s.template_id = t.template_id \
                    AND s.world_id = $1 AND s.spawn_id BETWEEN $2 AND $3 \
              WHERE t.template_id BETWEEN 1610 AND 1659 \
              GROUP BY t.template_id ORDER BY t.template_id",
        )
        .bind(WORLD_ID)
        .bind(FIRST_SPAWN)
        .bind(LAST_SPAWN)
        .fetch_all(&pool)
        .await
        .expect("armory spawn query");
        assert!(
            !counts.is_empty(),
            "the seed must define Armory templates 1610-1659 (control)"
        );
        for (template, spawned) in &counts {
            assert_eq!(
                *spawned, 1,
                "Armory template {template} must spawn exactly once in world \
                 {WORLD_ID} within spawns {FIRST_SPAWN}-{LAST_SPAWN}"
            );
        }
    }

    /// Each Armory buy list holds 1 to `ARMORY_ROW_CAP` rows, every design
    /// exists in `resources.items`, and every price equals max(1, the highest
    /// sell-list unit price for that design), computed in SQL.
    #[tokio::test]
    async fn live_db_armory_lists_stocked_and_priced() {
        let pool = require_db_or_skip!();

        let stock: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT l.item_list_id, COUNT(b.item_id) \
               FROM resources.item_lists l \
               LEFT JOIN resources.item_list_items b ON b.item_list_id = l.item_list_id \
              WHERE l.item_list_id IN (SELECT buy_item_list FROM resources.entity_templates \
                                        WHERE template_id BETWEEN 1610 AND 1659 \
                                          AND buy_item_list IS NOT NULL) \
              GROUP BY l.item_list_id ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("armory stock query");
        assert!(
            !stock.is_empty(),
            "the Armory must have buy lists (control)"
        );
        for (list, rows) in &stock {
            assert!(
                (1..=ARMORY_ROW_CAP).contains(rows),
                "Armory buy list {list} holds {rows} rows, must be 1 to {ARMORY_ROW_CAP}"
            );
        }

        let orphans: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT b.item_list_id, b.design_id \
               FROM resources.item_list_items b \
              WHERE b.item_list_id IN (SELECT buy_item_list FROM resources.entity_templates \
                                        WHERE template_id BETWEEN 1610 AND 1659 \
                                          AND buy_item_list IS NOT NULL) \
                AND NOT EXISTS (SELECT 1 FROM resources.items i WHERE i.item_id = b.design_id)",
        )
        .fetch_all(&pool)
        .await
        .expect("armory design query");
        assert!(
            orphans.is_empty(),
            "Armory rows selling a design that does not exist, as (list, design): {orphans:?}"
        );

        let mispriced: Vec<(i32, i32, i32, i32)> = sqlx::query_as(
            "SELECT x.item_list_id, x.design_id, x.naquadah, x.expected \
               FROM (SELECT b.item_list_id, b.design_id, b.naquadah, \
                            GREATEST(1, COALESCE((SELECT MAX(s.naquadah) \
                                                    FROM resources.item_list_items s \
                                                   WHERE s.design_id = b.design_id \
                                                     AND s.item_list_id IN \
                                                         (SELECT sell_item_list FROM resources.entity_templates \
                                                           WHERE sell_item_list IS NOT NULL)), 0)) \
                                        AS expected \
                       FROM resources.item_list_items b \
                      WHERE b.item_list_id IN (SELECT buy_item_list FROM resources.entity_templates \
                                                WHERE template_id BETWEEN 1610 AND 1659 \
                                                  AND buy_item_list IS NOT NULL)) x \
              WHERE x.naquadah <> x.expected \
              ORDER BY 1, 2",
        )
        .fetch_all(&pool)
        .await
        .expect("armory price query");
        assert!(
            mispriced.is_empty(),
            "Armory rows priced other than max(1, highest sell-list price), as \
             (list, design, price, expected): {mispriced:?}"
        );
    }

    /// Every archetype tree ability with a non-empty `item_monikers` has an
    /// Armory row whose design carries one of them, except `KNOWN_UNCOVERED`,
    /// which is exactly the set of misses.
    #[tokio::test]
    async fn live_db_armory_covers_tree_weapons() {
        let pool = require_db_or_skip!();
        let gated: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT t.ability_id) \
               FROM resources.archetype_ability_tree t \
               JOIN resources.abilities a ON a.ability_id = t.ability_id \
              WHERE cardinality(a.item_monikers) > 0",
        )
        .fetch_one(&pool)
        .await
        .expect("gated ability count");
        assert!(
            gated > 0,
            "some tree abilities need a weapon moniker (control)"
        );

        let misses: Vec<(i32,)> = sqlx::query_as(
            "SELECT g.ability_id \
               FROM (SELECT DISTINCT t.ability_id, a.item_monikers \
                       FROM resources.archetype_ability_tree t \
                       JOIN resources.abilities a ON a.ability_id = t.ability_id \
                      WHERE cardinality(a.item_monikers) > 0) g \
              WHERE NOT EXISTS (SELECT 1 FROM resources.item_list_items b \
                                  JOIN resources.items i ON i.item_id = b.design_id \
                                 WHERE b.item_list_id IN \
                                       (SELECT buy_item_list FROM resources.entity_templates \
                                         WHERE template_id BETWEEN 1610 AND 1659 \
                                           AND buy_item_list IS NOT NULL) \
                                   AND i.moniker_ids && g.item_monikers) \
              ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("armory coverage query");
        let mut missing: Vec<i32> = misses.into_iter().map(|(id,)| id).collect();
        missing.sort_unstable();
        let mut expected = KNOWN_UNCOVERED.to_vec();
        expected.sort_unstable();
        assert_eq!(
            missing, expected,
            "tree abilities needing a weapon with no Armory carrier must be exactly KNOWN_UNCOVERED"
        );
    }

    /// Every (equipment slot 4-14, tier) present in `resources.items` has an
    /// Armory row whose design sits in that slot at that tier.
    #[tokio::test]
    async fn live_db_armory_covers_equipment_slots() {
        let pool = require_db_or_skip!();
        let pairs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM (SELECT DISTINCT s.slot, i.tier \
               FROM resources.items i \
               CROSS JOIN LATERAL unnest(i.container_sets) AS s(slot) \
              WHERE s.slot BETWEEN 4 AND 14) p",
        )
        .fetch_one(&pool)
        .await
        .expect("slot and tier count");
        assert!(pairs > 0, "the items must have equipment slots (control)");

        let uncovered: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT DISTINCT s.slot, i.tier \
               FROM resources.items i \
               CROSS JOIN LATERAL unnest(i.container_sets) AS s(slot) \
              WHERE s.slot BETWEEN 4 AND 14 \
                AND NOT EXISTS (SELECT 1 FROM resources.item_list_items b \
                                  JOIN resources.items d ON d.item_id = b.design_id \
                                 WHERE b.item_list_id IN \
                                       (SELECT buy_item_list FROM resources.entity_templates \
                                         WHERE template_id BETWEEN 1610 AND 1659 \
                                           AND buy_item_list IS NOT NULL) \
                                   AND d.tier = i.tier \
                                   AND s.slot = ANY(d.container_sets)) \
              ORDER BY 1, 2",
        )
        .fetch_all(&pool)
        .await
        .expect("armory slot query");
        assert!(
            uncovered.is_empty(),
            "equipment (slot, tier) with no Armory row: {uncovered:?}"
        );
    }

    /// Every Armory sell-list row names a design the sell path accepts:
    /// `items.flags` has CanBeSold (1024) and not BindOnAcquire (4), the two
    /// bits `vendor/sell/mod.rs` joins on. A design it refuses stays in the
    /// bag (README F7), so the sell list must hold only sellable designs.
    #[tokio::test]
    async fn live_db_armory_sell_list_is_sellable() {
        let pool = require_db_or_skip!();
        let rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.item_list_items WHERE item_list_id IN \
               (SELECT sell_item_list FROM resources.entity_templates \
                 WHERE template_id BETWEEN 1610 AND 1659 AND sell_item_list IS NOT NULL)",
        )
        .fetch_one(&pool)
        .await
        .expect("armory sell-list count");
        assert!(rows > 0, "the Armory sell list must have rows (control)");

        let bad: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT b.item_list_id, b.design_id \
               FROM resources.item_list_items b \
               JOIN resources.items i ON i.item_id = b.design_id \
              WHERE b.item_list_id IN \
                    (SELECT sell_item_list FROM resources.entity_templates \
                      WHERE template_id BETWEEN 1610 AND 1659 AND sell_item_list IS NOT NULL) \
                AND ((COALESCE(i.flags, 0) & 1024) = 0 OR (COALESCE(i.flags, 0) & 4) <> 0) \
              ORDER BY 1, 2",
        )
        .fetch_all(&pool)
        .await
        .expect("armory sell-list sellable query");
        assert!(
            bad.is_empty(),
            "Armory sell-list rows the sell path refuses (no CanBeSold, or BindOnAcquire), as (list, design): {bad:?}"
        );
    }
}
