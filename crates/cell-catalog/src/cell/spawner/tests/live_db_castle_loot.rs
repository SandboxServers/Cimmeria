//! Live-DB guard for the Castle (World 8) hostile loot tables 4-6, the owner's
//! D-CP09 decision in the Castle population pass
//! (`docs/analysis/castle-population/README.md`): Castle hostiles drop something
//! sometimes, never every time, and only items that do something today.
//!
//! Split from `live_db_castle_population.rs`, which guards placement; this
//! guards what a corpse can hold. The seed loads cleanly whatever the rows say,
//! so none of these mistakes surfaces anywhere but in a playtest:
//!
//! * a template left without a table, so a whole post never drops anything;
//! * a row at probability 1, which is the guaranteed Cellblock drop the owner
//!   ruled out;
//! * an item whose use is an `items_event_sets` event-5 ability, which the
//!   server never executes, so the player clicks it and nothing happens.
mod live_db {
    use std::collections::HashMap;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `(template_id, loot_table_id)` for every World 8 hostile template (D-CP09):
    /// 4 rank-and-file guard, 5 veteran / named / officer, 6 drone salvage.
    const CASTLE_LOOT: [(i32, i32); 12] = [
        (145, 6),
        (146, 4),
        (148, 4),
        (181, 4),
        (182, 4),
        (184, 4),
        (185, 4),
        (183, 5),
        (186, 5),
        (169, 5),
        (170, 5),
        (171, 5),
    ];
    /// `(loot_table_id, lowest, highest)` chance that a corpse drops nothing,
    /// the product of every row's miss chance. Seeded: 36 %, 23 %, 56 %.
    const EMPTY_RATE_BANDS: [(i32, f32, f32); 3] =
        [(4, 0.30, 0.40), (5, 0.20, 0.25), (6, 0.50, 0.60)];
    /// Items a Castle loot row may drop; `None` is naquadah. 2893 Health
    /// Slappack TC1 is the only consumable whose use works today (content chain
    /// 4001 on `item_use` 2893); event-5 "use" abilities are never executed, so
    /// TC5 health, focus heals and Mark III stimpacks would do nothing on click.
    /// 5224, 5188 and 5257 are tier-1 crafting components that blueprints
    /// consume.
    const USABLE_CONSUMABLES: [i32; 1] = [2893];
    const CRAFTING_COMPONENTS: [i32; 3] = [5224, 5188, 5257];

    /// **Population guard (D-CP09)**: every Castle hostile rolls a Castle loot
    /// table, no row is guaranteed, every item does something today, and each
    /// table's chance of an empty corpse stays in its band.
    ///
    /// Rows roll independently (`cell-combat` `abilities/loot_drop.rs`), so the
    /// empty chance is the product of the misses; an all-miss roll leaves the
    /// corpse without a loot cursor, which is the intended "nothing dropped".
    /// A probability of 1 turns "sometimes" into the guaranteed Cellblock drop
    /// the owner ruled out, and an item outside the allowed sets is either a
    /// consumable that does nothing on its first click or clutter no recipe
    /// uses. Both sets are also checked against the data: the consumable still
    /// has its `item_use` chain and every component is still consumed by at
    /// least one blueprint.
    #[tokio::test]
    async fn castle_live_db_hostile_loot_drops_sometimes_and_only_useful_items() {
        let pool = require_db_or_skip!();
        let templates: HashMap<i32, SpawnRecord> = load_spawn_templates(&pool)
            .await
            .expect("load_spawn_templates must succeed");
        for (id, table) in CASTLE_LOOT {
            let t = templates
                .get(&id)
                .unwrap_or_else(|| panic!("template {id} must load"));
            assert_eq!(
                t.loot_table_id,
                Some(table),
                "Castle hostile template {id} must roll loot table {table}"
            );
        }
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        for h in spawns
            .iter()
            .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
        {
            assert!(
                CASTLE_LOOT.iter().any(|(id, _)| *id == h.template_id),
                "World 8 hostile {:?} (spawn {}) uses template {}, which has no \
                 Castle loot assignment",
                h.tag,
                h.spawn_id,
                h.template_id
            );
        }

        let rows: Vec<(i32, i32, Option<i32>, f32, i32, i32)> = sqlx::query_as(
            "SELECT loot_id, loot_table_id, design_id, probability::real, \
                    min_quantity, max_quantity \
             FROM resources.loot WHERE loot_table_id IN (4, 5, 6) ORDER BY loot_id",
        )
        .fetch_all(&pool)
        .await
        .expect("loot rows query must succeed");
        for (loot_id, table, design, p, min, max) in &rows {
            assert!(
                *p > 0.0 && *p < 1.0,
                "loot row {loot_id} (table {table}) has probability {p}; Castle drops \
                 are 'sometimes', never guaranteed or impossible"
            );
            assert!(
                0 < *min && min <= max,
                "loot row {loot_id} quantity {min}..{max}"
            );
            if let Some(item) = design {
                assert!(
                    USABLE_CONSUMABLES.contains(item) || CRAFTING_COMPONENTS.contains(item),
                    "loot row {loot_id} (table {table}) drops item {item}, which is \
                     neither a consumable with a working use path nor a crafting \
                     component"
                );
            }
            if *table == 6 {
                assert!(
                    design.is_some_and(|i| CRAFTING_COMPONENTS.contains(&i)),
                    "PRU salvage row {loot_id} must be a crafting component, not \
                     naquadah or a heal"
                );
            }
        }
        for (table, lo, hi) in EMPTY_RATE_BANDS {
            let table_rows: Vec<_> = rows.iter().filter(|r| r.1 == table).collect();
            assert!(!table_rows.is_empty(), "loot table {table} has no rows");
            let empty: f32 = table_rows.iter().map(|r| 1.0 - r.3).product();
            assert!(
                (lo..=hi).contains(&empty),
                "loot table {table} leaves {:.1} % of corpses empty; the band is \
                 {:.0}-{:.0} %",
                empty * 100.0,
                lo * 100.0,
                hi * 100.0
            );
        }

        for item in USABLE_CONSUMABLES {
            let chains: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM resources.content_triggers t \
                 JOIN resources.content_chains c ON c.chain_id = t.chain_id \
                 WHERE t.event_type = 'item_use' AND t.event_key = $1 AND c.enabled",
            )
            .bind(item.to_string())
            .fetch_one(&pool)
            .await
            .expect("item_use chain query must succeed");
            assert!(
                chains > 0,
                "consumable {item} has no enabled item_use chain; it would do nothing on click"
            );
        }
        for item in CRAFTING_COMPONENTS {
            let uses: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM resources.blueprints_components WHERE item_id = $1",
            )
            .bind(item)
            .fetch_one(&pool)
            .await
            .expect("blueprints_components query must succeed");
            assert!(uses > 0, "component {item} is consumed by no blueprint");
        }
    }
}
