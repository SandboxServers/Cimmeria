//! Live-DB guards for the ammo campaign's loot seed (AM-05, issue #1026,
//! `db/resources/Loot/Seed/ammo_loot.sql`), acceptance criterion 8:
//!
//! * the debug hub crate (table 3) hands out, at probability 1, a full stack
//!   of every bullet special and a pistol and an SMG that accept all five
//!   (D-AM06);
//! * the Castle pre-Romney chest (tables 8 and 9) carries Hollow Point
//!   (D-AM03);
//! * the NPC drop rows are exactly the decided ones, at modest rates, and no
//!   other table drops ammo.
//!
//! Ammo items are named through `resources.ammo_item_types`, never by id, as
//! the seed does. The seed loads cleanly whatever its rows say, so a missing
//! type, a short stack or a wrong weapon only shows up here or in a UAT.
mod live_db {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The five bullet specials, as `EAmmoType` labels.
    const BULLET_SPECIALS: [&str; 5] = [
        "Bullet_Armor_Piercing",
        "Bullet_Hollow_Point",
        "Bullet_Incendiary",
        "Bullet_EMP",
        "Bullet_Explosive",
    ];

    /// The debug crate's table (D-AM06) and the chest's two tables (D-AM03).
    const DEBUG_CRATE: i32 = 3;
    const CHEST_TABLES: [i32; 2] = [8, 9];

    /// `(loot_table_id, ammo_type, min, max, probability)` for every NPC drop
    /// row AM-05 seeds. Modest by design: nothing above 10 %.
    const NPC_DROPS: [(i32, &str, i32, i32, f32); 4] = [
        (4, "Bullet_Hollow_Point", 10, 25, 0.05),
        (5, "Bullet_Hollow_Point", 15, 30, 0.06),
        (5, "Bullet_Armor_Piercing", 10, 20, 0.03),
        (7, "Bullet_Hollow_Point", 10, 25, 0.10),
    ];
    const MAX_NPC_DROP_PROBABILITY: f32 = 0.10;

    /// `item_id -> (ammo_type label, max_stack_size)` for every mapped ammo item.
    async fn ammo_items(pool: &sqlx::PgPool) -> BTreeMap<i32, (String, i32)> {
        let rows: Vec<(i32, String, i32)> = sqlx::query_as(
            "SELECT a.item_id, a.ammo_type::text, i.max_stack_size \
               FROM resources.ammo_item_types a \
               JOIN resources.items i ON i.item_id = a.item_id",
        )
        .fetch_all(pool)
        .await
        .expect("ammo_item_types query must succeed");
        assert!(!rows.is_empty(), "AM-F seeds ammo_item_types");
        rows.into_iter().map(|(id, t, s)| (id, (t, s))).collect()
    }

    fn item_for(ammo: &BTreeMap<i32, (String, i32)>, label: &str) -> (i32, i32) {
        ammo.iter()
            .find(|(_, (t, _))| t == label)
            .map(|(id, (_, stack))| (*id, *stack))
            .unwrap_or_else(|| panic!("{label} must map to an ammo item"))
    }

    /// D-AM06 through the loader the cell rolls from: one full-stack row per
    /// bullet special at probability 1, and at least one Standard Pistol and
    /// one Standard SMG row at probability 1 whose `ammo_types` list all five.
    #[tokio::test]
    async fn ammo_loot_live_db_debug_crate_hands_out_every_bullet_special_and_two_weapons() {
        let pool = require_db_or_skip!();
        let tables = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed");
        let entries = tables
            .get(&DEBUG_CRATE)
            .expect("loot table 3 must have rows");
        let ammo = ammo_items(&pool).await;

        for label in BULLET_SPECIALS {
            let (item, stack) = item_for(&ammo, label);
            let rows: Vec<_> = entries
                .iter()
                .filter(|e| e.design_id == Some(item))
                .collect();
            assert_eq!(rows.len(), 1, "table 3 must drop {label} ({item}) once");
            let e = rows[0];
            assert_eq!(e.probability, 1.0, "{label} is certain on the debug crate");
            assert_eq!(
                (e.min_quantity, e.max_quantity),
                (stack, stack),
                "{label} drops a full stack of {stack} rounds"
            );
        }

        // Weapons on table 3 at probability 1, with their family and ammo list.
        let weapons: Vec<(i32, String, Vec<String>)> = sqlx::query_as(
            "SELECT i.item_id, i.description, i.ammo_types::text[] \
               FROM resources.loot l JOIN resources.items i ON i.item_id = l.design_id \
              WHERE l.loot_table_id = $1 AND l.probability = 1 AND i.clip_size > 0",
        )
        .bind(DEBUG_CRATE)
        .fetch_all(&pool)
        .await
        .expect("table 3 weapon query must succeed");
        for family in ["Standard Pistol", "Standard SMG"] {
            let fit: Vec<_> = weapons
                .iter()
                .filter(|(_, d, types)| {
                    d == family && BULLET_SPECIALS.iter().all(|t| types.iter().any(|x| x == t))
                })
                .collect();
            assert!(
                !fit.is_empty(),
                "table 3 must drop a {family} that accepts all five bullet specials \
                 for certain; weapons on the table: {weapons:?}"
            );
        }
        // Every certain weapon is in the one-per-row loader list too.
        for (id, _, _) in &weapons {
            assert!(
                entries
                    .iter()
                    .any(|e| e.design_id == Some(*id) && (e.min_quantity, e.max_quantity) == (1, 1)),
                "weapon {id} drops exactly one"
            );
        }
    }

    /// D-AM03: each archetype's pre-Romney chest carries Hollow Point, certain,
    /// once, within one stack, and no other ammo type.
    #[tokio::test]
    async fn ammo_loot_live_db_castle_chests_carry_hollow_point() {
        let pool = require_db_or_skip!();
        let tables = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed");
        let ammo = ammo_items(&pool).await;
        let (hp, stack) = item_for(&ammo, "Bullet_Hollow_Point");
        for table in CHEST_TABLES {
            let entries = tables
                .get(&table)
                .unwrap_or_else(|| panic!("loot table {table} must have rows"));
            let rows: Vec<_> = entries
                .iter()
                .filter(|e| e.design_id.is_some_and(|d| ammo.contains_key(&d)))
                .collect();
            assert_eq!(
                rows.len(),
                1,
                "chest table {table} carries exactly one ammo row: {rows:?}"
            );
            let e = rows[0];
            assert_eq!(
                e.design_id,
                Some(hp),
                "chest table {table} ammo is Hollow Point"
            );
            assert_eq!(e.probability, 1.0, "chest rows are certain");
            assert_eq!(
                (e.min_quantity, e.max_quantity),
                (50, 75),
                "chest table {table} gives 50-75 Hollow Point rounds"
            );
            assert!(e.max_quantity <= stack, "within one stack of {stack}");
        }
    }

    /// Every ammo loot row, on any table, stays within one stack; the NPC drop
    /// rows are exactly [`NPC_DROPS`], none above
    /// [`MAX_NPC_DROP_PROBABILITY`]; and no
    /// table outside the debug crate, the chests and those NPC tables drops
    /// ammo at all (a stray row on the Cellblock tutorial guard, the PRU
    /// drones or a mission container would be a live-content change nobody
    /// decided).
    #[tokio::test]
    async fn ammo_loot_live_db_npc_drop_rows_are_the_decided_modest_ones() {
        let pool = require_db_or_skip!();
        let ammo = ammo_items(&pool).await;
        let rows: Vec<(i32, i32, i32, i32, f32)> = sqlx::query_as(
            "SELECT l.loot_table_id, l.design_id, l.min_quantity, l.max_quantity, \
                    l.probability::real \
               FROM resources.loot l \
               JOIN resources.ammo_item_types a ON a.item_id = l.design_id \
              ORDER BY l.loot_id",
        )
        .fetch_all(&pool)
        .await
        .expect("ammo loot rows query must succeed");

        // `GrantItem` inserts the looted count as one stack, uncapped, so a
        // row above the item's max stack would mint an over-full stack. Every
        // ammo row, on any table, stays within one stack.
        for &(t, d, lo, hi, _) in &rows {
            let (label, stack) = &ammo[&d];
            assert!(
                0 < lo && lo <= hi && hi <= *stack,
                "table {t} drops {label} {lo}..{hi}, above its max stack of {stack}"
            );
        }

        let mut decided: BTreeSet<i32> = BTreeSet::from([DEBUG_CRATE]);
        decided.extend(CHEST_TABLES);
        decided.extend(NPC_DROPS.iter().map(|d| d.0));
        let strays: Vec<_> = rows.iter().filter(|r| !decided.contains(&r.0)).collect();
        assert!(
            strays.is_empty(),
            "ammo drops from a table AM-05 did not decide: {strays:?}"
        );

        let npc_tables: BTreeSet<i32> = NPC_DROPS.iter().map(|d| d.0).collect();
        let npc: Vec<(i32, String, i32, i32, f32)> = rows
            .iter()
            .filter(|r| npc_tables.contains(&r.0))
            .map(|&(t, d, lo, hi, p)| (t, ammo[&d].0.clone(), lo, hi, p))
            .collect();
        let want: Vec<(i32, String, i32, i32, f32)> = NPC_DROPS
            .iter()
            .map(|&(t, a, lo, hi, p)| (t, a.to_string(), lo, hi, p))
            .collect();
        assert_eq!(npc, want, "the NPC ammo drop rows");
        for (t, label, lo, hi, p) in &npc {
            assert!(
                *p > 0.0 && *p <= MAX_NPC_DROP_PROBABILITY,
                "table {t} drops {label} at {p}; NPC ammo drops stay modest"
            );
            let (_, stack) = item_for(&ammo, label);
            assert!(
                0 < *lo && lo <= hi && *hi <= stack,
                "table {t} {label} {lo}..{hi}"
            );
        }
    }
}
