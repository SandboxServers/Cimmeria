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

    /// Seeded respawners must carry authored coordinates, not the world
    /// origin.
    ///
    /// Bug shape (Castle audit defect B1): all four World 8 / `Castle` rows
    /// shipped as `(0, 0, 0)` — the recovered data kept the checkpoint names
    /// and lost the positions. That is worse than having no row at all,
    /// because `resolve_respawn_target` finds the row by id (the Defeat
    /// Window offers it by name) or by world, returns its zeros, and never
    /// reaches the in-place / Castle-default fallbacks below it. Every death
    /// in Castle teleported the player to the world origin.
    ///
    /// `respawn.rs`'s `is_unauthored` guard now skips origin rows at
    /// runtime, but the guard only downgrades the failure to "respawn where
    /// you died" — the coordinates still have to exist for a checkpoint to
    /// work. This is the seed-side half of that pair.
    ///
    /// Scope: every row EXCEPT the two World 23 `Beta_Site_Evo_1` rows,
    /// which are a documented, evidence-less gap (see the KNOWN GAP comment
    /// in `db/resources/Worlds/Seed/respawners.sql`). Listing them rather
    /// than weakening the assertion to "world 8 only" means a new
    /// unauthored row in any world trips this test.
    ///
    /// The exemption is a **composite** pin — id, world and still-at-origin
    /// — not a bare id allowlist. A bare id list silently exempts whatever
    /// row happens to hold that id later: renumber the seed, or delete row
    /// 6 and reuse the id for a real checkpoint, and the exemption follows
    /// the number rather than the gap it was written for. Pinning the world
    /// and the zeros means any of those edits fails here and forces the
    /// author to re-read the KNOWN GAP comment.
    ///
    /// The Castle assertions are load-bearing too: without the name set,
    /// deleting the four rows instead of authoring them would pass the
    /// origin check vacuously, and the distinctness check catches the
    /// copy-paste collapse (four rows, one position) that the origin check
    /// cannot see. Positions themselves are deliberately not pinned — they
    /// move after the in-client UAT (D-CA11).
    #[tokio::test]
    async fn seeded_respawners_are_not_at_the_world_origin() {
        /// World 23 `Beta_Site_Evo_1`: names survived, positions did not,
        /// and nothing in the seed or the recovered scripts says where the
        /// zone's respawn points were. Left at the origin deliberately.
        const UNAUTHORED_BY_DESIGN: [(i32, &str); 2] =
            [(6, "Beta_Site_Evo_1"), (7, "Beta_Site_Evo_1")];
        const CASTLE_WORLD: &str = "Castle";
        const CASTLE_CHECKPOINTS: [&str; 4] = [
            "Armory Respawn",
            "Checkpoint Alpha Respawn",
            "Op-Core Triangle Respawn",
            "Throne Checkpoint Respawn",
        ];

        let pool = require_db_or_skip!();
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");

        // The exemption must still describe the rows it was written for.
        for (id, world) in UNAUTHORED_BY_DESIGN {
            let row = respawners
                .iter()
                .find(|r| r.respawner_id == id)
                .unwrap_or_else(|| {
                    panic!(
                        "respawner {id} is exempted as an unauthored world-23 row but no \
                         longer exists — drop it from UNAUTHORED_BY_DESIGN (and from the \
                         KNOWN GAP comment in db/resources/Worlds/Seed/respawners.sql) \
                         rather than leaving a stale exemption that a future row can \
                         inherit"
                    )
                });
            assert_eq!(
                row.world_name, world,
                "respawner {id} is exempted as a world-23 ({world}) row but now belongs \
                 to '{}' — the exemption is following the id, not the documented gap",
                row.world_name
            );
            assert_eq!(
                row.pos,
                [0.0, 0.0, 0.0],
                "respawner {id} '{}' has been authored ({:?}) — remove it from \
                 UNAUTHORED_BY_DESIGN so it is covered by the origin check like every \
                 other row",
                row.name,
                row.pos
            );
        }

        let at_origin: Vec<String> = respawners
            .iter()
            .filter(|r| {
                !UNAUTHORED_BY_DESIGN
                    .iter()
                    .any(|(id, _)| *id == r.respawner_id)
            })
            .filter(|r| r.pos == [0.0, 0.0, 0.0])
            .map(|r| format!("{} '{}' (world {})", r.respawner_id, r.name, r.world_name))
            .collect();
        assert!(
            at_origin.is_empty(),
            "respawner rows sitting at the world origin: {at_origin:?} — (0,0,0) means \
             the row was never authored, so the checkpoint does not work: the Defeat \
             Window still offers it by name, `resolve_respawn_target` skips it, and the \
             player is quietly put wherever the fallback lands (in place, or the Castle \
             default) instead of at the checkpoint they picked. Author the coordinates \
             in db/resources/Worlds/Seed/respawners.sql, or add the row to \
             UNAUTHORED_BY_DESIGN here with a seed comment saying why it cannot be \
             authored"
        );

        let castle: Vec<&RespawnerDef> = respawners
            .iter()
            .filter(|r| r.world_name == CASTLE_WORLD)
            .collect();

        let mut names: Vec<&str> = castle.iter().map(|r| r.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names, CASTLE_CHECKPOINTS,
            "world '{CASTLE_WORLD}' must ship exactly these four authored checkpoints"
        );

        for (i, a) in castle.iter().enumerate() {
            for b in castle.iter().skip(i + 1) {
                assert_ne!(
                    a.pos, b.pos,
                    "'{}' and '{}' share a position {:?} — four checkpoints that all \
                     land on the same spot is the copy-paste failure the origin check \
                     cannot see",
                    a.name, b.name, a.pos
                );
            }
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

    /// The gate-event pipeline's only DB-shaped dependency (CA10):
    /// `load_stargates` must carry `stargates.event_set_id`, and that
    /// event set must resolve `Stargate_MakeGate` (6100) and
    /// `Stargate_CrossGate` (6113). Every `cell::gate_travel` unit test
    /// seeds its own `sequence_map`, so dropping the column from the
    /// SELECT, renaming it, or clearing Castle gate 2's event set leaves
    /// the suite green while the live server plays no gate animation.
    ///
    /// Castle gate 2 is the anchor: `stargates.sql` gives it
    /// `event_set_id = 10011`, mapped to 10145 (6100) and 10158 (6113).
    #[tokio::test]
    async fn load_stargates_carries_the_event_set_that_resolves_gate_sequences() {
        const GATE: i32 = 2;
        const EVENT_SET: i32 = 10011;

        let pool = require_db_or_skip!();
        let gates = load_stargates(&pool)
            .await
            .expect("load_stargates must succeed");
        let castle = gates.get(&GATE).expect("seeded stargates has gate 2");
        assert_eq!(
            castle.world_name, "Castle",
            "gate 2 is Castle's — if this moved, re-anchor the test"
        );
        assert_eq!(
            castle.event_set_id,
            Some(EVENT_SET),
            "load_stargates must select stargates.event_set_id; None here \
             means the column left the SELECT or the seed cleared it, and \
             the gate would open with no animation"
        );

        let sequences = load_event_set_sequences(&pool)
            .await
            .expect("load_event_set_sequences must succeed");
        for (event_id, label) in [(6100, "Stargate_MakeGate"), (6113, "Stargate_CrossGate")] {
            assert!(
                sequences.contains_key(&(EVENT_SET, event_id)),
                "event set {EVENT_SET} must resolve {label} ({event_id}) — \
                 without it `send_gate_sequence` warns and emits nothing"
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

    /// **CA05 regression guard** (docs/analysis/castle-rebuild/work-packets.md CA05
    /// acceptance): every new Castle story-actor tag must resolve to exactly one
    /// `resources.spawnlist` row in World 8 ("Castle") whose `entity_templates` join
    /// succeeds (non-empty `template_name`), and every new Castle point set must have
    /// at least one point. Deleting a tag's spawnlist row, its template row, or a point
    /// set's points must fail this test — that is the guard's whole job.
    #[tokio::test]
    async fn castle_ca05_story_actor_tags_resolve_to_exactly_one_spawn_row() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        const CASTLE_TAGS: [&str; 9] = [
            "Castle_Zuritska_Cell",
            "Castle_Zuritska_Comms",
            "Castle_Romney",
            "Castle_Muelbach",
            "Castle_BravoOfficer1",
            "Castle_BravoOfficer2",
            "Castle_BravoOfficer3",
            "Castle_SurrenderGuard",
            "Castle_CommsTerminal",
        ];

        for tag in CASTLE_TAGS {
            let matches: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.tag.as_deref() == Some(tag))
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "tag {tag} must resolve to exactly one resources.spawnlist row in \
                 World 8 (Castle); found {} — a deleted or duplicated CA05 spawn row \
                 would fail this assertion",
                matches.len()
            );
            let record = matches[0];
            assert_eq!(
                record.world_name, "Castle",
                "tag {tag} must be a World 8 (Castle) spawn, got world {}",
                record.world_name
            );
            assert!(
                !record.template_name.is_empty(),
                "tag {tag} spawn row's entity_templates join produced an empty \
                 template_name — the referenced template_id was deleted or renamed"
            );
        }
    }

    /// **CA05 regression guard**: every new Castle point set (Interrogation Block,
    /// Comms Room, Checkpoint Bravo, Checkpoint Alpha) must have at least one point.
    /// Deleting a point set's `point_set_points` rows must fail this test.
    #[tokio::test]
    async fn castle_ca05_point_sets_have_at_least_one_point() {
        let pool = require_db_or_skip!();
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");

        const CASTLE_POINT_SETS: [&str; 4] = [
            "Castle.InterrogationBlock",
            "Castle.CommsRoom",
            "Castle.CheckpointBravo",
            "Castle.CheckpointAlpha",
        ];

        for name in CASTLE_POINT_SETS {
            let region = regions.iter().find(|r| r.name == name).unwrap_or_else(|| {
                panic!("point set {name} must be loaded by load_regions_from_db")
            });
            assert!(
                !region.points.is_empty(),
                "point set {name} (set_id={}) has no points — a deleted \
                 point_set_points row would fail this assertion",
                region.set_id
            );
        }
    }

    /// **CA05 regression guard** (coordinator scope addition, worknotes/ca05.md "Zone-wide
    /// hostile respawn timers"): every hostile (faction 10) World 8 spawn row must have a
    /// non-NULL resolved `respawn_secs`, or the first player to kill it locks that mob out
    /// of the shared world for everyone else (no shipped Castle spawn row set a respawn
    /// timer before this packet). A regression that drops `respawn_secs` from a hostile
    /// row — new or pre-existing — must fail this test.
    #[tokio::test]
    async fn castle_hostile_world8_spawns_all_have_a_respawn_timer() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        let hostile_castle_spawns: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
            .collect();
        assert!(
            !hostile_castle_spawns.is_empty(),
            "expected at least one hostile (faction 10) World 8 spawn row \
             (NID Guard / Prisoner Retrieval Unit templates 145/146/148, plus \
             Castle_Romney/Castle_Muelbach/Castle_BravoOfficer*) — none found, \
             the probe query itself may be broken"
        );
        for r in &hostile_castle_spawns {
            assert!(
                r.respawn_secs.is_some(),
                "hostile World 8 spawn {} (tag={:?}, template_id={}) has no \
                 resolved respawn_secs — it would permanently disappear from the \
                 shared world after the first kill",
                r.spawn_id,
                r.tag,
                r.template_id
            );
        }
    }

    /// **CA05 regression guard** (worknotes/ca05.md "Recovered display names"): every named
    /// CA05 story actor must resolve a non-zero `name_id`.
    ///
    /// `name_id` is written raw onto the AoI create packet and only when it is `Some(n)` with
    /// `n != 0` (`mercury/aoi/create.rs:211-218`), so a NULL or 0 column silently ships a
    /// nameless NPC to the client — there is no server-side fallback and no error. The ids
    /// themselves are recovered originals from `texts.sql` (7066 'Dr. Zuritska', 6962 'NID
    /// Interrogator Romney', 6965 'Warden Muelbach', 6966 'NID Officer', 7720
    /// 'Communications Terminal'), and new ids cannot be minted because the client resolves
    /// them against its own PAK string table. Reverting any of those columns to NULL — which
    /// is what this packet's first draft shipped — must fail this test.
    #[tokio::test]
    async fn castle_ca05_story_actors_all_have_a_client_resolvable_name_id() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        // Castle_SurrenderGuard is deliberately included: objective 2794 names him only
        // "a guard", so he keeps template 148's generic 7417 'NID Guard' rather than a
        // recovered unique name — but he still must have *a* name.
        const NAMED_CASTLE_TAGS: [&str; 9] = [
            "Castle_Zuritska_Cell",
            "Castle_Zuritska_Comms",
            "Castle_Romney",
            "Castle_Muelbach",
            "Castle_BravoOfficer1",
            "Castle_BravoOfficer2",
            "Castle_BravoOfficer3",
            "Castle_SurrenderGuard",
            "Castle_CommsTerminal",
        ];

        for tag in NAMED_CASTLE_TAGS {
            let record = records
                .iter()
                .find(|r| r.tag.as_deref() == Some(tag))
                .unwrap_or_else(|| panic!("tag {tag} must have a spawnlist row"));
            let name_id = record.name_id.unwrap_or(0);
            assert!(
                name_id != 0,
                "tag {tag} (template_id={}) resolves name_id={:?} — the AoI create packet \
                 omits the name property for NULL/0, so this actor would appear unnamed \
                 in-client",
                record.template_id,
                record.name_id
            );
        }
    }
}
