//! Live-DB sanity tests for the spawner loader functions.
//!
//! Split out of the monolithic `spawner/tests.rs` (issue #529) — every
//! test body and assertion is byte-identical to the original.
//!
//! Each loader is a thin sqlx query that maps `resources.*` rows into a
//! runtime cache. The tests below run each loader against the seeded DB
//! and assert (a) it doesn't error, (b) the cache is non-empty (the
//! seeded resources schema has rows for every loaded table), (c) sample
//! rows have plausible shape. These are byte-cheap regression guards
//! for column renames, type drift, and JOIN breakage that the rest of
//! the test suite wouldn't catch — sqlx surfaces those as `Err` from
//! the loader.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn load_loot_tables_returns_seeded_data_with_non_empty_entries() {
        let pool = require_db_or_skip!();
        let map = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed against seeded DB");
        assert!(!map.is_empty(), "seeded resources.loot has rows");
        for (id, entries) in &map {
            assert!(
                !entries.is_empty(),
                "loot_table {id} present in map but has no entries",
            );
        }
    }

    /// Cellblock guard table (loot_table_id = 2) must include a guaranteed
    /// Health Slappack TC1 drop (item 2893). Without this, back-to-back
    /// Castle CellBlock guard fights leave players with no burst-recovery
    /// option even with out-of-combat regen. The seed file is the source
    /// of truth — pinning probability = 1.0 here means a future nerf has
    /// to update both the seed and this test consciously.
    #[tokio::test]
    async fn load_loot_tables_includes_guaranteed_slappack_for_cellblock_guards() {
        const CELLBLOCK_GUARD_LOOT_TABLE: i32 = 2;
        const HEALTH_SLAPPACK_TC1: i32 = 2893;

        let pool = require_db_or_skip!();
        let map = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed against seeded DB");

        let entries = map
            .get(&CELLBLOCK_GUARD_LOOT_TABLE)
            .expect("loot_table_id=2 (Cellblock NID guard default) must exist");
        let slappack = entries
            .iter()
            .find(|e| e.design_id == Some(HEALTH_SLAPPACK_TC1))
            .expect("loot_table 2 must include Health Slappack TC1 (item 2893)");
        assert_eq!(
            slappack.probability, 1.0,
            "slappack must be a guaranteed drop"
        );
        assert_eq!(slappack.min_quantity, 1);
        assert_eq!(slappack.max_quantity, 1);
    }

    #[tokio::test]
    async fn load_item_defs_returns_seeded_weapons_with_clip_size_columns() {
        let pool = require_db_or_skip!();
        let map = load_item_defs(&pool)
            .await
            .expect("load_item_defs must succeed against seeded DB");
        // The loader filters `WHERE clip_size > 0` — actual ammo-bearing
        // weapons. Every cached entry must have a positive clip_size; the
        // cache must not be empty (the seed has weapons).
        assert!(
            !map.is_empty(),
            "seeded resources.items has weapons with clip_size > 0"
        );
        for (item_id, def) in &map {
            assert!(
                def.clip_size > 0,
                "item {item_id} surfaced from load_item_defs with non-positive \
                 clip_size {} — the WHERE clip_size > 0 filter has regressed and \
                 non-weapons (clip_size = 0 in the seed) are leaking into the \
                 WeaponDef cache",
                def.clip_size
            );
        }
    }

    /// The slappack (type 2893, `clip_size = 0` in the seed) MUST NOT be
    /// in the WeaponDef cache. Companion to the `clip_size > 0` filter
    /// assertion above — that test catches "any zero-clip leak"; this
    /// test catches "the specific slappack leak that motivated the fix"
    /// so a regression names the right culprit.
    ///
    /// Bug shape this guards against: reverting `WHERE clip_size > 0`
    /// to `WHERE clip_size IS NOT NULL` would silently put every
    /// non-weapon back into the cache as a zero-clip `WeaponDef` —
    /// confusing the equipment-grant code, which keys off the cache for
    /// ammo seeding (see `cell/content/executor/inventory.rs::weapon_stats`).
    #[tokio::test]
    async fn load_item_defs_excludes_zero_clip_consumables_like_slappack() {
        let pool = require_db_or_skip!();
        let map = load_item_defs(&pool)
            .await
            .expect("load_item_defs must succeed against seeded DB");

        // 2893 = Health Slappack TC1 (db/resources/Items/Seed/items.sql).
        // Its seed row has clip_size = 0 — the canonical non-weapon shape.
        const SLAPPACK_TYPE_ID: i32 = 2893;
        assert!(
            !map.contains_key(&SLAPPACK_TYPE_ID),
            "slappack (type {SLAPPACK_TYPE_ID}, clip_size=0 in seed) leaked into \
             the WeaponDef cache — the `WHERE clip_size > 0` filter regressed",
        );
    }

    /// Health Slappack TC1 (item 2893) + its TC-18 craftable duplicate
    /// (item 4735) must seed with `max_stack_size = 10` and the
    /// `set:ItemIcon001 image:Medkit` icon. The original 2009 seed
    /// shipped `max_stack_size = 1` + `set:CoreWidgets image:IconMissing`
    /// (a placeholder), so each looted slappack ate its own bag slot
    /// and rendered as a generic broken-icon square. This guard pins
    /// the corrected values across both rows so a revert that drops
    /// the migration (or the seed change) is caught at CI before
    /// players see broken icons + 5 slappacks taking 5 slots again.
    ///
    /// Both rows are checked because the inventory UI displays by
    /// item-id, not by name — having 4735 still at the placeholder
    /// would surface as "the slappack I crafted looks different
    /// from the one I looted" even though both are named identically.
    #[tokio::test]
    async fn health_slappack_seeds_with_stack_10_and_medkit_icon() {
        let pool = require_db_or_skip!();
        const SLAPPACK_ITEM_IDS: [i32; 2] = [2893, 4735];

        for item_id in SLAPPACK_ITEM_IDS {
            let row: (i32, String) = sqlx::query_as(
                "SELECT max_stack_size, icon_location \
                 FROM resources.items \
                 WHERE item_id = $1",
            )
            .bind(item_id)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| {
                panic!("slappack item {item_id} must exist in seeded resources.items: {e}")
            });

            let (max_stack_size, icon_location) = row;
            assert_eq!(
                max_stack_size, 10,
                "Health Slappack TC1 (item {item_id}) must seed with \
                 max_stack_size = 10 so consumables stack instead of each \
                 eating a bag slot; pre-fix value was 1 — a regression to \
                 the placeholder would surface as 5 looted slappacks \
                 occupying 5 different inventory rows"
            );
            assert_eq!(
                icon_location, "set:ItemIcon001 image:Medkit",
                "Health Slappack TC1 (item {item_id}) must seed with the \
                 Medkit icon; pre-fix value was the 'set:CoreWidgets \
                 image:IconMissing' placeholder which renders as a broken \
                 square in the inventory UI"
            );
        }
    }

    #[tokio::test]
    async fn load_item_containers_projects_first_element_of_container_sets() {
        let pool = require_db_or_skip!();
        // Pick a seeded item with a non-empty `container_sets` and remember
        // its first element. The loader's `container_sets[1]` projection
        // (PostgreSQL is 1-indexed) must round-trip that exact value into
        // the cached HashMap. A regression that swaps to `container_sets[2]`
        // or aggregates the array would fail this assertion.
        let row: Option<(i32, i32)> = sqlx::query_as(
            "SELECT item_id, container_sets[1] AS first_container \
             FROM resources.items \
             WHERE array_length(container_sets, 1) > 0 \
             ORDER BY item_id \
             LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("seed query must succeed");
        let (probe_item_id, expected_first) =
            row.expect("seed must have at least one row with non-empty container_sets");

        let map = load_item_containers(&pool)
            .await
            .expect("load_item_containers must succeed against seeded DB");
        assert!(!map.is_empty(), "non-empty seed → non-empty cache");
        assert_eq!(
            map.get(&probe_item_id).copied(),
            Some(expected_first),
            "loader must project the FIRST element of container_sets"
        );
    }

    #[tokio::test]
    async fn load_respawners_returns_seeded_rows_with_world_names() {
        let pool = require_db_or_skip!();
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");
        assert!(!respawners.is_empty());
        for r in &respawners {
            assert!(
                !r.world_name.is_empty(),
                "respawner {} has empty world_name — JOIN to resources.worlds broke",
                r.respawner_id
            );
        }
    }

    #[tokio::test]
    async fn load_spawns_returns_records_with_resolved_world_names() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        assert!(!records.is_empty(), "seeded resources.spawnlist has rows");
        for r in &records {
            assert!(
                !r.world_name.is_empty(),
                "spawn {} has empty world_name — JOIN to resources.worlds broke",
                r.spawn_id
            );
            assert!(
                !r.template_name.is_empty(),
                "spawn {} has empty template_name — JOIN to entity_templates broke",
                r.spawn_id
            );
        }
    }

    #[tokio::test]
    async fn load_mission_defs_only_includes_missions_with_a_step() {
        let pool = require_db_or_skip!();
        let map = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        assert!(!map.is_empty(), "seeded mission_steps has rows");
        for (mission_id, entry) in &map {
            assert!(
                entry.step_id > 0,
                "mission {mission_id} has non-positive step_id {}",
                entry.step_id
            );
        }
    }

    #[tokio::test]
    async fn load_step_objectives_groups_by_step_id() {
        let pool = require_db_or_skip!();
        let map = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        assert!(!map.is_empty());
        for (step_id, objs) in &map {
            assert!(
                !objs.is_empty(),
                "step {step_id} present in map with no objectives — should have been filtered out"
            );
        }
    }

    #[tokio::test]
    async fn load_dialog_set_maps_drops_rows_with_null_dialog_id() {
        let pool = require_db_or_skip!();
        let map = load_dialog_set_maps(&pool)
            .await
            .expect("load_dialog_set_maps must succeed");
        assert!(
            !map.is_empty(),
            "seeded dialog_set_maps has rows with non-null dialog_id"
        );
        // The loader explicitly drops rows where `dialog_id IS NULL` —
        // every cached entry must have a positive dialog_id by construction.
        for (set_map_id, entry) in &map {
            assert!(
                entry.dialog_id > 0,
                "dialog_set_map_id {set_map_id} surfaced with non-positive dialog_id {}",
                entry.dialog_id
            );
        }
        // Direct invariant pin: any row with NULL dialog_id in the seeded
        // table must be absent from the loaded cache. Catches a regression
        // that flips the `if let Some(dialog_id)` to a default-on-None.
        let null_set_map_id: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_set_map_id FROM resources.dialog_set_maps WHERE dialog_id IS NULL LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(id) = null_set_map_id {
            assert!(
                !map.contains_key(&id),
                "dialog_set_map_id {id} has NULL dialog_id in DB but surfaced in the cache"
            );
        }
    }

    /// **Regression guard for the monologue dialog cache:** dialog 2982
    /// is the Castle Cellblock wake-up monologue, two screens both
    /// `speaker_id = 0`. The cache must include it OR the
    /// `DisplayDialog` monologue fallback never triggers and the
    /// cellblock opening narration silently never shows.
    ///
    /// Also asserts the predicate is correctly exclusive — a dialog
    /// with any non-zero speaker_id row must NOT surface in the
    /// monologue set (else the executor would bind the player as the
    /// NPC for that dialog and blank the NPC portrait — the bug the
    /// original abort gate was designed to prevent).
    #[tokio::test]
    async fn load_monologue_dialog_ids_includes_cellblock_wakeup() {
        let pool = require_db_or_skip!();
        let ids = load_monologue_dialog_ids(&pool)
            .await
            .expect("load_monologue_dialog_ids must succeed");

        // Dialog 2982 = Castle Cellblock wake-up monologue.
        assert!(
            ids.contains(&2982),
            "dialog 2982 (cellblock wake-up monologue) must be in the monologue set; \
             without it the chain-1001 DisplayDialog continues to silently never fire"
        );

        // Pick any dialog id from the seed that has at least one
        // non-zero speaker_id screen — it must NOT be in the cache.
        // Fetch a representative dialog and assert.
        let mixed_dialog: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_id FROM resources.dialog_screens \
             WHERE speaker_id <> 0 \
             GROUP BY dialog_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(d) = mixed_dialog {
            assert!(
                !ids.contains(&d),
                "dialog {d} has at least one non-zero speaker_id screen — it must NOT be \
                 in the monologue set, or the executor's monologue fallback would \
                 incorrectly bind the player for an NPC dialog"
            );
        }

        // A dialog with at least one NULL `speaker_id` screen must NOT
        // be classified as a monologue. The original predicate
        // (`COUNT(*) FILTER (WHERE speaker_id <> 0) = 0`) treated NULL
        // as "not non-zero" and wrongly admitted such dialogs — the
        // exact mis-binding the cache exists to prevent.
        let null_speaker_dialog: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_id FROM resources.dialog_screens \
             WHERE speaker_id IS NULL \
             GROUP BY dialog_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(d) = null_speaker_dialog {
            assert!(
                !ids.contains(&d),
                "dialog {d} has at least one NULL speaker_id screen — it must NOT be \
                 in the monologue set; treating NULL as monologue would mis-bind the \
                 player as the dialog context entity for unknown-speaker dialogs"
            );
        }
    }

    #[tokio::test]
    async fn load_stargates_resolves_world_join() {
        let pool = require_db_or_skip!();
        let map = load_stargates(&pool)
            .await
            .expect("load_stargates must succeed");
        assert!(!map.is_empty());
        for (id, entry) in &map {
            assert!(
                !entry.world_name.is_empty(),
                "stargate {id} has empty world_name — JOIN to resources.worlds broke"
            );
        }
    }

    #[tokio::test]
    async fn load_regions_applies_single_point_cylinder_workaround() {
        let pool = require_db_or_skip!();
        // Find a seeded AreaSet that the workaround SHOULD expand:
        // type='AreaSet', radius > 0, exactly one row in point_set_points.
        // Without this query, a seed with no qualifying region would let
        // a workaround-removed regression slip through (the conditional
        // `if let Some(r) = expanded` would be skipped entirely).
        let probe: Option<(i32, f32)> = sqlx::query_as(
            "SELECT ps.set_id, ps.radius FROM resources.point_sets ps \
             JOIN ( \
               SELECT set_id, COUNT(*) AS pts FROM resources.point_set_points GROUP BY set_id \
             ) c ON c.set_id = ps.set_id \
             WHERE ps.type = 'AreaSet' AND ps.radius > 0 AND c.pts = 1 \
             ORDER BY ps.set_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("seed probe query must succeed");
        let (probe_set_id, probe_radius) = probe.expect(
            "seed must contain at least one type='AreaSet' single-point cylinder \
             (radius > 0, exactly one point) so the workaround test isn't vacuous",
        );

        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");
        assert!(!regions.is_empty());

        let expanded = regions
            .iter()
            .find(|r| r.set_id == probe_set_id)
            .expect("probe region must surface from load_regions_from_db");
        // GenericRegion.workaround(): single-point input + radius > 0 →
        // 4-point bounding box. If a refactor drops the workaround,
        // expanded.points.len() stays at 1 and this assertion fails.
        assert_eq!(
            expanded.points.len(),
            4,
            "workaround must expand single-point cylinder set_id={probe_set_id} to 4 points"
        );
        // 4-point box: opposing-corner x distance == 2*radius.
        let dx = (expanded.points[2][0] - expanded.points[0][0]).abs();
        let dz = (expanded.points[2][2] - expanded.points[0][2]).abs();
        assert!(
            (dx - 2.0 * probe_radius).abs() < 1e-3,
            "expanded region {probe_set_id} x-extent {dx} should be 2*radius {}",
            2.0 * probe_radius
        );
        assert!(
            (dz - 2.0 * probe_radius).abs() < 1e-3,
            "expanded region {probe_set_id} z-extent {dz} should be 2*radius {}",
            2.0 * probe_radius
        );
    }

    /// **Regression guard for the effect-def loader PG ENUM decode bug.**
    ///
    /// `resources.effects.target_collection_method` is a PG ENUM
    /// (`resources."ETargetCollectionMethod"`), not TEXT. sqlx-postgres
    /// won't auto-coerce the ENUM into `Option<String>` (the
    /// `EffectRow` field type), so the whole `fetch_all` returns a
    /// decode error and `load_effect_defs` returns `Err`. The startup
    /// path swallows that as a WARN, leaving `effect_defs` EMPTY for
    /// the entire process lifetime — every combat ability that
    /// resolves through an effect silently no-ops.
    ///
    /// The fix is `target_collection_method::TEXT` in the SELECT.
    /// Reverting the cast must fail this test with a decode error along
    /// the lines of:
    ///
    /// ```text
    /// error occurred while decoding column
    /// "target_collection_method": mismatched types; Rust type
    /// core::option::Option<alloc::string::String> ... is not
    /// compatible with SQL type
    /// resources."ETargetCollectionMethod"
    /// ```
    #[tokio::test]
    async fn load_effect_defs_succeeds_against_seeded_db() {
        let pool = require_db_or_skip!();
        let map = load_effect_defs(&pool)
            .await
            .expect("load_effect_defs must succeed — PG ENUM cast regression");
        assert!(
            !map.is_empty(),
            "seeded resources.effects has rows; an empty map means \
             the loader silently failed or the seed didn't load"
        );
        // Pin that target_collection_method actually came through —
        // not blank, not all-default. The default-fallback in the loader
        // (`unwrap_or_else(|| TCM_SINGLE.to_string())`) means a column
        // that's NULL falls back to TCM_Single; but every row mapping
        // to TCM_Single would suggest the SELECT silently lost the
        // column or every NULL got the fallback. Check at least one
        // row carries a non-default value (the seed has TCM_AECone +
        // TCM_AERadius rows).
        let has_non_single_tcm = map
            .values()
            .any(|e| e.target_collection_method != cimmeria_entity::abilities::TCM_SINGLE);
        assert!(
            has_non_single_tcm,
            "seeded effects include TCM_AECone and TCM_AERadius rows; \
             every effect being TCM_Single suggests the cast lost data \
             or the column isn't actually being read"
        );
        // Companion to the assertion above: the seed also has rows with
        // `target_collection_method IS NULL`, and the loader is
        // documented to fall back to `TCM_SINGLE` for those. If the
        // fallback path broke (e.g., a refactor that returned an empty
        // string or panicked on NULL) the `unwrap_or_else` would never
        // fire and these rows would surface with something other than
        // `TCM_SINGLE`. Pin that at least one row in the loaded map
        // carries the fallback value — which is true today because of
        // the canonical seed's NULL rows.
        let has_single_tcm = map
            .values()
            .any(|e| e.target_collection_method == cimmeria_entity::abilities::TCM_SINGLE);
        assert!(
            has_single_tcm,
            "loader's `unwrap_or_else(TCM_SINGLE)` fallback for NULL \
             target_collection_method must surface as TCM_SINGLE on at \
             least one row (the seed has NULL rows that exercise this \
             path)"
        );
    }
}
