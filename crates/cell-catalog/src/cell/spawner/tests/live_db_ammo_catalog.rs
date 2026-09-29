//! Live-DB guards for the ammo campaign's foundation seed (AM-F, issue
//! #1026, `docs/analysis/ammo/`):
//!
//! * the `EAmmoType` ordinals `cimmeria_entity::ammo_type` hard-codes match
//!   `pg_enum`, label by label (audit A-14: every loader computes the ordinal
//!   inline and nothing pinned it before);
//! * `resources.ammo_item_types` maps exactly the fifteen bullet and dart
//!   specials, each to an existing stackable reserve item that stays out of
//!   the `WeaponDef` cache;
//! * the Standard Pistol (27) and Standard SMG (25) families accept all five
//!   bullet specials (D-AM10);
//! * the catalog loader reads both tables.
mod live_db {
    use std::collections::BTreeMap;

    use cimmeria_entity::ammo_type::{self, is_special, LABELS};

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const BULLET_SPECIALS: [&str; 5] = [
        "Bullet_Armor_Piercing",
        "Bullet_Hollow_Point",
        "Bullet_Incendiary",
        "Bullet_EMP",
        "Bullet_Explosive",
    ];

    #[tokio::test]
    async fn ammo_type_ordinals_match_pg_enum() {
        let pool = require_db_or_skip!();
        let rows: Vec<(String, f32)> = sqlx::query_as(
            "SELECT enumlabel::text, enumsortorder FROM pg_enum \
             WHERE enumtypid = 'resources.\"EAmmoType\"'::regtype \
             ORDER BY enumsortorder",
        )
        .fetch_all(&pool)
        .await
        .expect("read pg_enum");
        let labels: Vec<&str> = rows.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            labels,
            LABELS.to_vec(),
            "resources.\"EAmmoType\" no longer matches cimmeria_entity::ammo_type"
        );
        // The ordinal the loaders compute is the same index.
        let hp: i32 = sqlx::query_scalar(
            "SELECT (array_position(enum_range(NULL::resources.\"EAmmoType\"), \
                     'Bullet_Hollow_Point'::resources.\"EAmmoType\") - 1)::integer",
        )
        .fetch_one(&pool)
        .await
        .expect("ordinal of Bullet_Hollow_Point");
        assert_eq!(hp, ammo_type::BULLET_HOLLOW_POINT);
    }

    #[tokio::test]
    async fn ammo_item_types_maps_every_bullet_and_dart_special_once() {
        let pool = require_db_or_skip!();
        let rows: Vec<(i32, i32, i32, i32, i32, Vec<i32>)> = sqlx::query_as(
            "SELECT (array_position(enum_range(NULL::resources.\"EAmmoType\"), t.ammo_type) - 1)::integer, \
                    t.item_id, i.max_stack_size, i.clip_size, i.flags, i.container_sets \
             FROM resources.ammo_item_types t \
             JOIN resources.items i ON i.item_id = t.item_id \
             ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("read ammo_item_types");
        let unjoined: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.ammo_item_types t \
             WHERE NOT EXISTS (SELECT 1 FROM resources.items i WHERE i.item_id = t.item_id)",
        )
        .fetch_one(&pool)
        .await
        .expect("count orphan rows");
        assert_eq!(unjoined, 0, "an ammo_item_types row names a missing item");

        // Exactly the bullet and dart specials; no default, no dagger.
        let expected: Vec<i32> = (ammo_type::BULLET_ARMOR_PIERCING..=ammo_type::BULLET_EXPLOSIVE)
            .chain(ammo_type::DART_POISON..=ammo_type::DART_ADRENALINE)
            .collect();
        let types: Vec<i32> = rows.iter().map(|r| r.0).collect();
        assert_eq!(types, expected);
        assert_eq!(rows.len(), 15);

        let mut by_item = BTreeMap::new();
        for (t, item_id, max_stack, clip, flags, sets) in &rows {
            assert!(is_special(*t), "type {t} is not special");
            assert!(
                (9000..=9014).contains(item_id),
                "type {t} maps to {item_id}, outside the reserved 9000-9014 block"
            );
            assert!(*max_stack > 1, "item {item_id} must stack");
            assert_eq!(*clip, 0, "item {item_id} is a reserve stack, not a weapon");
            assert_eq!(flags & 4, 0, "item {item_id} must not bind on acquire");
            assert_eq!(
                sets.first(),
                Some(&1),
                "item {item_id} lands in the main bag"
            );
            assert!(
                by_item.insert(*item_id, *t).is_none(),
                "item {item_id} twice"
            );
        }

        // The WeaponDef cache must not pick the reserve items up.
        let defs = load_item_defs(&pool).await.expect("load_item_defs");
        for item_id in by_item.keys() {
            assert!(!defs.contains_key(item_id), "{item_id} in WeaponDef cache");
        }
    }

    #[tokio::test]
    async fn standard_pistol_and_smg_accept_every_bullet_special() {
        let pool = require_db_or_skip!();
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT description, count(*), \
                    count(*) FILTER (WHERE ammo_types @> $1::text[]::resources.\"EAmmoType\"[] \
                                       AND 'Bullet_Default' = ANY (ammo_types) \
                                       AND array_length(ammo_types, 1) = 6) \
             FROM resources.items \
             WHERE description IN ('Standard Pistol', 'Standard SMG') \
             GROUP BY description ORDER BY description",
        )
        .bind(BULLET_SPECIALS.map(String::from).to_vec())
        .fetch_all(&pool)
        .await
        .expect("read widened families");
        assert_eq!(
            rows,
            vec![
                ("Standard Pistol".to_string(), 27, 27),
                ("Standard SMG".to_string(), 25, 25),
            ],
            "(family, ids, ids listing Bullet_Default plus the five specials once)"
        );

        // The WeaponDef cache the whitelist reads carries the widening.
        let defs = load_item_defs(&pool).await.expect("load_item_defs");
        let pistol = defs.get(&3241).expect("SI 3 9mm Pistol in WeaponDef cache");
        for t in ammo_type::BULLET_ARMOR_PIERCING..=ammo_type::BULLET_EXPLOSIVE {
            assert!(pistol.allowed_ammo_types.contains(&t), "3241 lacks {t}");
        }
    }

    #[tokio::test]
    async fn load_ammo_catalog_reads_both_tables() {
        let pool = require_db_or_skip!();
        let catalog = load_ammo_catalog(&pool).await.expect("load_ammo_catalog");
        assert_eq!(catalog.item_type_count(), 15);
        assert_eq!(
            catalog.item_id_for(ammo_type::BULLET_ARMOR_PIERCING),
            Some(9000)
        );
        assert_eq!(
            catalog.item_id_for(ammo_type::BULLET_HOLLOW_POINT),
            Some(9001)
        );
        assert_eq!(catalog.item_id_for(ammo_type::DART_ADRENALINE), Some(9014));
        assert_eq!(
            catalog.ammo_type_for_item(9001),
            Some(ammo_type::BULLET_HOLLOW_POINT)
        );
        assert_eq!(catalog.item_id_for(ammo_type::BULLET_DEFAULT), None);
        // Modifier rows arrive with the family packets; each must name a
        // special type.
        let modifiers = load_ammo_modifiers(&pool)
            .await
            .expect("load_ammo_modifiers");
        for (t, m) in &modifiers {
            assert!(is_special(*t), "modifier for non-special type {t}");
            assert_eq!(*t, m.ammo_type);
        }
    }
}
