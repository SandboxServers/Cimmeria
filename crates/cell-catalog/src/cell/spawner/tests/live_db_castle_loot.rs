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
//! * a consumable with no working use path (the Stealth, Energy and Disguise
//!   boosts, the antidotes), which the player loots, clicks, and gets "This
//!   item has no effect yet.";
//! * a stimpack stronger than Mark III in a level 3-4 zone.
mod live_db {
    use std::collections::{HashMap, HashSet};

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
    /// the product of every row's miss chance. Seeded: 30.5 %, 17.5 %, 56 %.
    const EMPTY_RATE_BANDS: [(i32, f32, f32); 3] =
        [(4, 0.25, 0.35), (5, 0.15, 0.20), (6, 0.50, 0.60)];

    /// The item-use ability id the seed binds to 158 mission items as a
    /// filler. Mirrors `PLACEHOLDER_ITEM_USE_ABILITY` in cell-content's
    /// `content/consumable_use.rs`, which this crate cannot depend on.
    const PLACEHOLDER_ITEM_USE_ABILITY: i32 = 597;
    /// The effect scripts the native consumable path applies (same file).
    const NATIVE_CONSUMABLE_SCRIPTS: [&str; 3] = ["HealHealth", "HealFocus", "StatBuff"];
    /// A bag consumable with no working use path today: the native path
    /// refuses it with "This item has no effect yet." and no chain owns it.
    /// The predicate below must reject it, or it would not reject anything.
    const UNWIRED_CONSUMABLE: i32 = 6206; // Stealth Boost Consumable

    /// Seed data the working-use predicate reads, from the loaders the cell
    /// uses at startup.
    struct UsePaths {
        bindings: HashMap<(i32, i32), i32>,
        abilities: HashMap<i32, cimmeria_entity::abilities::AbilityDef>,
        effects: HashMap<i32, cimmeria_entity::abilities::EffectDef>,
        chain_items: HashSet<String>,
    }

    impl UsePaths {
        async fn load(pool: &sqlx::PgPool) -> Self {
            let chain_items: Vec<(String,)> = sqlx::query_as(
                "SELECT t.event_key FROM resources.content_triggers t \
                 JOIN resources.content_chains c ON c.chain_id = t.chain_id \
                 WHERE t.event_type = 'item_use' AND c.enabled",
            )
            .fetch_all(pool)
            .await
            .expect("item_use chain query must succeed");
            Self {
                bindings: load_item_event_set_abilities(pool)
                    .await
                    .expect("load_item_event_set_abilities"),
                abilities: load_ability_defs(pool).await.expect("load_ability_defs"),
                effects: load_effect_defs(pool).await.expect("load_effect_defs"),
                chain_items: chain_items.into_iter().map(|(k,)| k).collect(),
            }
        }

        /// Whether `item` is a consumable: it has an event-5 use binding other
        /// than the 597 filler. Several consumables (the Stealth Boost among
        /// them) are also blueprint inputs, so a consumable must pass
        /// [`Self::works`] even when a recipe uses it.
        fn is_consumable(&self, item: i32) -> bool {
            self.bindings
                .get(&(item, EVENT_ITEM_USE_ABILITY))
                .is_some_and(|&a| a != PLACEHOLDER_ITEM_USE_ABILITY)
        }

        /// Whether using `item` does something today: an enabled `item_use`
        /// chain owns it, or `consumable_use::classify` would call it
        /// `Native` -- an event-5 binding to an ability other than the 597
        /// filler, with at least one effect, every effect running a script
        /// the native path applies.
        fn works(&self, item: i32) -> bool {
            if self.chain_items.contains(&item.to_string()) {
                return true;
            }
            let Some(&ability) = self.bindings.get(&(item, EVENT_ITEM_USE_ABILITY)) else {
                return false;
            };
            if ability == PLACEHOLDER_ITEM_USE_ABILITY {
                return false;
            }
            let Some(def) = self.abilities.get(&ability) else {
                return false;
            };
            !def.effect_ids.is_empty()
                && def.effect_ids.iter().all(|e| {
                    self.effects
                        .get(e)
                        .and_then(|d| d.script_name.as_deref())
                        .is_some_and(|s| NATIVE_CONSUMABLE_SCRIPTS.contains(&s))
                })
        }
    }

    /// **Population guard (D-CP09)**: every Castle hostile rolls a Castle loot
    /// table, no row is guaranteed, every item does something today, and each
    /// table's chance of an empty corpse stays in its band.
    ///
    /// Rows roll independently (`cell-combat` `abilities/loot_drop.rs`), so the
    /// empty chance is the product of the misses; an all-miss roll leaves the
    /// corpse without a loot cursor, which is the intended "nothing dropped".
    /// An item is allowed when it is naquadah, a consumable with a working use
    /// path (see [`UsePaths::works`]), or a non-consumable crafting component a
    /// blueprint uses; the drone table takes components only. Stimpacks
    /// stop at Mark III: Mark V and up are too strong for a level 3-4 zone.
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

        let uses = UsePaths::load(&pool).await;
        assert!(
            uses.is_consumable(UNWIRED_CONSUMABLE) && !uses.works(UNWIRED_CONSUMABLE),
            "item {UNWIRED_CONSUMABLE} (Stealth Boost) now reads as working; if it \
             really gained a use path, pick another unwired consumable here"
        );
        let components: HashSet<i32> =
            sqlx::query_scalar("SELECT DISTINCT item_id FROM resources.blueprints_components")
                .fetch_all(&pool)
                .await
                .expect("blueprints_components query must succeed")
                .into_iter()
                .collect();

        let rows: Vec<(i32, i32, Option<i32>, f32, i32, i32, Option<String>)> = sqlx::query_as(
            "SELECT l.loot_id, l.loot_table_id, l.design_id, l.probability::real, \
                        l.min_quantity, l.max_quantity, i.name \
                 FROM resources.loot l LEFT JOIN resources.items i ON i.item_id = l.design_id \
                 WHERE l.loot_table_id IN (4, 5, 6) ORDER BY l.loot_id",
        )
        .fetch_all(&pool)
        .await
        .expect("loot rows query must succeed");
        for (loot_id, table, design, p, min, max, name) in &rows {
            assert!(
                *p > 0.0 && *p < 1.0,
                "loot row {loot_id} (table {table}) has probability {p}; Castle drops \
                 are 'sometimes', never guaranteed or impossible"
            );
            assert!(
                0 < *min && min <= max,
                "loot row {loot_id} quantity {min}..{max}"
            );
            let Some(item) = *design else {
                assert!(
                    *table != 6,
                    "PRU salvage row {loot_id} must not be naquadah"
                );
                continue;
            };
            let consumable = uses.is_consumable(item);
            let is_component = !consumable && components.contains(&item);
            if consumable {
                assert!(
                    uses.works(item),
                    "loot row {loot_id} (table {table}) drops consumable {item} ({name:?}) \
                     with no working use path; a player would click it and get \
                     'This item has no effect yet.'"
                );
            } else {
                assert!(
                    is_component,
                    "loot row {loot_id} (table {table}) drops item {item} ({name:?}), \
                     which is neither a crafting component a blueprint uses nor a \
                     consumable"
                );
            }
            if *table == 6 {
                assert!(
                    is_component,
                    "PRU salvage row {loot_id} must be a crafting component, not a \
                     consumable ({name:?})"
                );
            }
            let name = name.as_deref().unwrap_or("");
            assert!(
                !name.contains("Stimpack") || name.starts_with("Mark III "),
                "loot row {loot_id} drops {name:?}; Castle stimpacks stop at Mark III"
            );
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
    }
}
