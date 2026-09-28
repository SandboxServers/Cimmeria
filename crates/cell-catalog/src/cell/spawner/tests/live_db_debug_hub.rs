//! Live-DB guards for the Castle_CellBlock stasis-room debug hub: templates
//! 300-304, `spawnlist` 400-404, loot table 3 and dialogs
//! 60100-60103 (`docs/content/debug-hub.md`).
//!
//! Every guard here is about seed content that loads without error and is
//! still wrong, the same seam as [`super::live_db_castle_seed`]:
//!
//! * a template that lost its role column (a vendor with no buy list opens an
//!   empty store; a trainer with no list is not a trainer at all);
//! * an NPC placed outside the stasis room, or on top of the spot where every
//!   new character appears;
//! * a loot table with a roll that can come up empty, which leaves the corpse
//!   unclickable, or a list or table naming an item that does not exist;
//! * a crate whose ability set can hurt the new characters who shoot it;
//! * a dialog whose button sits anywhere but its final screen.
//!
//! Each was proven to fail with the hub's seed rows removed.
//!
//! The seed rows for dialogs 60100/60101 stay while their cooked-data
//! overrides are quarantined (client map-load crash, 2026-09-27; see
//! `QUARANTINED_DIALOG_OVERRIDES` in `cimmeria-resources`).
mod live_db {
    use std::collections::HashSet;

    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const VENDOR: i32 = 300;
    const TRAINER: i32 = 301;
    const DIALOG: i32 = 302;
    const LIVEWIRE: i32 = 303;
    const CRATE: i32 = 304;
    /// The Gate Mail Clerk (social-systems SS-U3, spawn 490). Its role
    /// columns are pinned in `live_db_mail_clerk.rs`; here it is placed.
    const MAIL_CLERK: i32 = 390;

    /// `(spawnlist.tag, template_id)` for every hub NPC.
    const HUB: [(&str, i32); 6] = [
        ("DebugHub_Vendor", VENDOR),
        ("DebugHub_Trainer", TRAINER),
        ("DebugHub_DialogNpc", DIALOG),
        ("DebugHub_LivewireTerminal", LIVEWIRE),
        ("DebugHub_LootCrate", CRATE),
        ("DebugHub_MailClerk", MAIL_CLERK),
    ];

    /// The stasis room (point set 2032) and the respawner every new
    /// character appears at (respawner 8, 'Stasis Chamber').
    const ROOM: &str = "Castle_Cellblock.Region1";
    const STASIS_RESPAWNER: i32 = 8;

    /// Minimum XZ distance from the respawner. The nearest seeded NPC is 5.5
    /// units away; anything closer crowds the spot a new character wakes up.
    const MIN_RESPAWNER_CLEARANCE: f32 = 5.0;

    /// Minimum XZ distance between two hub NPCs. `MAX_INTERACT_DISTANCE` is 5,
    /// so this is about bodies not overlapping, not about which NPC a click
    /// reaches (the client names the clicked entity).
    const MIN_NPC_SPACING: f32 = 2.5;

    /// The respawner's floor height, which the NPCs share.
    const FLOOR_Y: f32 = 73.472;

    fn hub_spawns(records: &[SpawnRecord]) -> Vec<&SpawnRecord> {
        HUB.iter()
            .map(|(tag, template)| {
                let matches: Vec<&SpawnRecord> = records
                    .iter()
                    .filter(|r| r.tag.as_deref() == Some(tag))
                    .collect();
                assert_eq!(
                    matches.len(),
                    1,
                    "tag {tag} must resolve to exactly one resources.spawnlist row"
                );
                assert_eq!(
                    matches[0].template_id, *template,
                    "tag {tag} must spawn template {template}"
                );
                matches[0]
            })
            .collect()
    }

    fn xz_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    /// Each template carries the columns its role runs on, and none of the
    /// others. The vendor needs its four lists and a vendor bit (the bit is
    /// what `spawn_npc_from_record_into` derives the store from); the trainer
    /// needs list 1 and no vendor lists, so the two roles never share a
    /// click; the crate is an unkillable container (no faction, no ability
    /// set, no death-time loot table) with the loot cursor, opened by chain
    /// 7020's `open_loot` (Decision (@Cadacious, 2026-09-28)).
    #[tokio::test]
    async fn debug_hub_templates_carry_their_role_fields() {
        let pool = require_db_or_skip!();
        let rows = sqlx::query(
            "SELECT template_id, template_name, class, faction, interaction_type, name_id, \
                    speaker_id, buy_item_list, sell_item_list, repair_item_list, \
                    recharge_item_list, trainer_ability_list_id, loot_table_id, ability_set_id \
             FROM resources.entity_templates WHERE template_id BETWEEN 300 AND 304 \
             ORDER BY template_id",
        )
        .fetch_all(&pool)
        .await
        .expect("entity_templates query must succeed");
        assert_eq!(rows.len(), 5, "templates 300-304 must all be seeded");

        type Row = (
            i32,
            String,
            String,
            Option<i32>,
            i64,
            Option<i32>,
            Option<i32>,
            [Option<i32>; 4],
            Option<i32>,
            Option<i32>,
            Option<i32>,
        );
        let got: Vec<Row> = rows
            .iter()
            .map(|r| {
                (
                    r.get("template_id"),
                    r.get("template_name"),
                    r.get("class"),
                    r.get("faction"),
                    r.get("interaction_type"),
                    r.get("name_id"),
                    r.get("speaker_id"),
                    [
                        r.get("buy_item_list"),
                        r.get("sell_item_list"),
                        r.get("repair_item_list"),
                        r.get("recharge_item_list"),
                    ],
                    r.get("trainer_ability_list_id"),
                    r.get("loot_table_id"),
                    r.get("ability_set_id"),
                )
            })
            .collect();

        let none4 = [None; 4];
        let want: Vec<Row> = vec![
            (
                VENDOR,
                "Debug Hub - Vendor".into(),
                "mob".into(),
                Some(1),
                65536, // INT_VendorGeneral
                Some(8010),
                None,
                [Some(1), Some(2), Some(2), Some(2)],
                None,
                None,
                None,
            ),
            (
                TRAINER,
                "Debug Hub - Trainer".into(),
                "mob".into(),
                Some(1),
                128, // INT_Trainer
                Some(20186),
                None,
                none4,
                Some(1),
                None,
                None,
            ),
            (
                DIALOG,
                "Debug Hub - Dialog".into(),
                "mob".into(),
                Some(1),
                134_217_728, // INT_NonAStoryMissionAvaliable
                Some(7412),
                Some(754),
                none4,
                None,
                None,
                None,
            ),
            (
                LIVEWIRE,
                "Debug Hub - Livewire Terminal".into(),
                "being".into(),
                Some(1),
                256, // INT_MinigameLivewire
                Some(7550),
                None,
                none4,
                None,
                None,
                None,
            ),
            (
                CRATE,
                "Debug Hub - Loot Crate".into(),
                "spawnable".into(),
                None,
                4_611_686_018_427_387_904, // INT_NormalLoot
                Some(7054),
                None,
                none4,
                None,
                None,
                None,
            ),
        ];
        assert_eq!(got, want, "debug-hub template role columns");
    }

    /// Every hub NPC is a world 12 spawn inside the stasis room's polygon, at
    /// the respawner's floor height, clear of the respawner and of every
    /// other spawn in the room (the pet trainer and any later corner
    /// included, not only the hub's own). None of them respawns, holds
    /// position by flag or carries an aggression override: the crate used to
    /// (a killable mob), and is an unkillable container now.
    #[tokio::test]
    async fn debug_hub_spawns_sit_inside_the_stasis_room() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");

        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));
        assert_eq!(room.points.len(), 4, "{ROOM} is a four-corner room");
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == STASIS_RESPAWNER)
            .expect("respawner 8 'Stasis Chamber' must be seeded");
        assert_eq!(respawner.world_name, "Castle_CellBlock");

        let spawns = hub_spawns(&records);
        for s in &spawns {
            let tag = s.tag.as_deref().unwrap_or_default();
            let pos = [s.x, s.y, s.z];
            assert_eq!(
                s.world_name, "Castle_CellBlock",
                "{tag} must be in world 12"
            );
            assert!(
                region_contains_xz(&room.points, s.x, s.z),
                "{tag} at ({}, {}) must stand inside {ROOM}",
                s.x,
                s.z
            );
            assert!(
                (s.y - FLOOR_Y).abs() < 0.5,
                "{tag} must stand on the stasis-room floor (y {FLOOR_Y}), got y {}",
                s.y
            );
            let clearance = xz_distance(pos, respawner.pos);
            assert!(
                clearance >= MIN_RESPAWNER_CLEARANCE,
                "{tag} is {clearance:.2} units from the respawner; a new character \
                 appears there, so keep at least {MIN_RESPAWNER_CLEARANCE}"
            );
        }
        let in_room: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| {
                r.world_name == "Castle_CellBlock" && region_contains_xz(&room.points, r.x, r.z)
            })
            .collect();
        for a in &spawns {
            for b in in_room.iter().filter(|b| b.spawn_id != a.spawn_id) {
                let d = xz_distance([a.x, a.y, a.z], [b.x, b.y, b.z]);
                assert!(
                    d >= MIN_NPC_SPACING,
                    "{:?} and {:?} are {d:.2} units apart",
                    a.tag,
                    b.tag
                );
            }
        }

        for s in &spawns {
            assert_eq!(s.respawn_secs, None, "{:?}: nothing respawns", s.tag);
            assert!(!s.is_stationary, "{:?}: no hold flag", s.tag);
            assert_eq!(
                s.aggression_override, None,
                "{:?}: no aggression override",
                s.tag
            );
        }
    }

    /// The crate is opened by exactly one chain: `interact_tag
    /// DebugHub_LootCrate` → `open_loot` on table 3, repeatable (no once
    /// flag), with no condition, so every click works for every character.
    #[tokio::test]
    async fn debug_hub_crate_opens_table_3_through_chain_7020() {
        let pool = require_db_or_skip!();
        let rows: Vec<(i32, String, Option<i32>, String)> = sqlx::query_as(
            "SELECT a.chain_id, a.action_type, a.target_id, a.params::text \
             FROM resources.content_triggers t \
             JOIN resources.content_actions a ON a.chain_id = t.chain_id \
             WHERE t.event_type = 'interact_tag' AND t.event_key = 'DebugHub_LootCrate'",
        )
        .fetch_all(&pool)
        .await
        .expect("crate chain query must succeed");
        assert_eq!(
            rows,
            vec![(7020, "open_loot".to_string(), Some(3), "{}".to_string())],
            "the crate is chain 7020's open_loot on table 3, repeatable"
        );
        let conditions: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.content_conditions WHERE chain_id = 7020",
        )
        .fetch_one(&pool)
        .await
        .expect("condition count");
        assert_eq!(conditions, 0, "no condition: every click works");
    }

    /// The trainer resolves list 1 through the same loader the cell uses at
    /// startup, and list 1 offers abilities. The respec gate
    /// (`trainer_pin`) reads the same map, so this is also the respec path's
    /// seed half.
    #[tokio::test]
    async fn debug_hub_trainer_resolves_list_1() {
        let pool = require_db_or_skip!();
        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert_eq!(
            lists.get(&TRAINER),
            Some(&1),
            "template 301 must train list 1"
        );
        for other in [VENDOR, DIALOG, LIVEWIRE, CRATE] {
            assert!(
                !lists.contains_key(&other),
                "template {other} must not be a trainer: the trainer check runs \
                 first and would swallow its click"
            );
        }
        let offered = load_trainer_abilities(&pool)
            .await
            .expect("load_trainer_abilities must succeed");
        let list1: usize = offered
            .iter()
            .filter(|((list, _), _)| *list == 1)
            .map(|(_, abilities)| abilities.len())
            .sum();
        assert!(list1 > 0, "trainer list 1 must offer abilities");
    }

    /// Every row of the vendor's four lists names a real item, and each list
    /// has at least one row. A dangling design id would put an item the
    /// client cannot draw in the store.
    #[tokio::test]
    async fn debug_hub_vendor_lists_resolve_to_real_items() {
        let pool = require_db_or_skip!();
        for column in [
            "buy_item_list",
            "sell_item_list",
            "repair_item_list",
            "recharge_item_list",
        ] {
            let sql = format!(
                "SELECT ili.design_id, i.item_id AS resolved \
                 FROM resources.entity_templates t \
                 JOIN resources.item_list_items ili ON ili.item_list_id = t.{column} \
                 LEFT JOIN resources.items i ON i.item_id = ili.design_id \
                 WHERE t.template_id = $1"
            );
            let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(VENDOR)
                .fetch_all(&pool)
                .await
                .unwrap_or_else(|e| panic!("{column} query must succeed: {e}"));
            assert!(!rows.is_empty(), "the vendor's {column} must have rows");
            for r in &rows {
                let design: i32 = r.get("design_id");
                let resolved: Option<i32> = r.get("resolved");
                assert_eq!(
                    resolved,
                    Some(design),
                    "{column} names design {design}, which is not in resources.items"
                );
            }
        }
    }

    /// Loot table 3 through the loader the cell rolls from: every item row
    /// names a real item, there is a naquadah row, and the corpse always has
    /// loot. A corpse with no loot never gets its loot bit, so the test
    /// reads as a broken loot path; the naquadah row and at least one item
    /// row therefore drop for certain, while the crafting knowledge items
    /// ride along at a lower chance.
    #[tokio::test]
    async fn debug_hub_loot_table_resolves_to_real_items() {
        let pool = require_db_or_skip!();
        let tables = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed");
        let entries = tables.get(&3).expect("loot table 3 must have rows");
        assert!(entries.len() >= 2, "loot table 3 must drop items and cash");
        assert!(
            entries.iter().any(|e| e.design_id.is_none()),
            "loot table 3 must carry a naquadah row (design_id NULL)"
        );
        let items: HashSet<i32> =
            sqlx::query_scalar::<_, i32>("SELECT item_id FROM resources.items")
                .fetch_all(&pool)
                .await
                .expect("items query must succeed")
                .into_iter()
                .collect();
        assert!(
            entries
                .iter()
                .any(|e| e.design_id.is_none() && e.probability == 1.0),
            "the naquadah row must always drop"
        );
        assert!(
            entries
                .iter()
                .any(|e| e.design_id.is_some() && e.probability == 1.0),
            "at least one item row must always drop"
        );
        for e in entries {
            assert!(
                e.probability > 0.0 && e.probability <= 1.0,
                "every table-3 row must be able to drop: {e:?}"
            );
            assert!(
                e.min_quantity >= 1 && e.min_quantity <= e.max_quantity,
                "table-3 quantities must be a non-empty range: {e:?}"
            );
            if let Some(design) = e.design_id {
                assert!(items.contains(&design), "loot design {design} must exist");
            }
        }
    }

    /// The crate also drops the five Racial Paradigm Guides and the Steel
    /// Plating blueprint item, each once, at a chance below certain. Each is
    /// a `{17,15}` item that stacks to 1, so the drop must be exactly one
    /// (a larger stack would be written as one over-full row), and each has
    /// a crafting effect, so using it does something.
    #[tokio::test]
    async fn debug_hub_loot_table_drops_crafting_knowledge_items() {
        let pool = require_db_or_skip!();
        let tables = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed");
        let entries = tables.get(&3).expect("loot table 3 must have rows");
        for design in [7805, 7806, 7807, 7808, 7809, 6483] {
            let rows: Vec<_> = entries
                .iter()
                .filter(|e| e.design_id == Some(design))
                .collect();
            assert_eq!(
                rows.len(),
                1,
                "table 3 must drop item {design} once: {rows:?}"
            );
            let e = rows[0];
            assert!(
                e.probability > 0.0 && e.probability < 1.0,
                "item {design} is a chance drop: {e:?}"
            );
            assert_eq!(
                (e.min_quantity, e.max_quantity),
                (1, 1),
                "item {design} stacks to 1"
            );
            let (sets, stack, effects): (Vec<i32>, i32, i64) = sqlx::query_as(
                "SELECT ri.container_sets, ri.max_stack_size, \
                        (SELECT COUNT(*) FROM resources.crafting_item_effects e \
                          WHERE e.item_id = ri.item_id) \
                   FROM resources.items ri WHERE ri.item_id = $1",
            )
            .bind(design)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("item {design} must exist: {e}"));
            assert_eq!(sets, vec![17, 15], "item {design} container_sets");
            assert_eq!(stack, 1, "item {design} max_stack_size");
            assert!(effects > 0, "item {design} must have a crafting effect");
        }
    }

    /// Dialog 60100 has two screens, pages in index order, and carries its
    /// one button on the final screen; 60101 has one screen and no button,
    /// so its close sends -1; the two bark lines exist. Both dialogs are
    /// spoken by the NPC, so neither may be in the monologue set, which
    /// would make `display_dialog` bind the player as the speaker.
    #[tokio::test]
    async fn debug_hub_dialogs_resolve_screens_and_buttons() {
        let pool = require_db_or_skip!();
        let screens = |dialog: i32| {
            let pool = pool.clone();
            async move {
                sqlx::query(
                    "SELECT s.screen_id, s.index, s.speaker_id, \
                            count(b.screen_button_id) AS buttons, \
                            min(b.button_type) AS button_type \
                     FROM resources.dialog_screens s \
                     LEFT JOIN resources.dialog_screen_buttons b ON b.screen_id = s.screen_id \
                     WHERE s.dialog_id = $1 \
                     GROUP BY s.screen_id, s.index, s.speaker_id ORDER BY s.index",
                )
                .bind(dialog)
                .fetch_all(&pool)
                .await
                .expect("dialog screens query must succeed")
                .iter()
                .map(|r| {
                    (
                        r.get::<i32, _>("screen_id"),
                        r.get::<i32, _>("index"),
                        r.get::<Option<i32>, _>("speaker_id"),
                        r.get::<i64, _>("buttons"),
                        r.get::<Option<i32>, _>("button_type"),
                    )
                })
                .collect::<Vec<_>>()
            }
        };

        assert_eq!(
            screens(60100).await,
            vec![
                (200000, 0, Some(754), 0, None),
                (200001, 1, Some(754), 1, Some(4)),
            ],
            "60100: two NPC screens, one Generic 1 button on the final one"
        );
        assert_eq!(
            screens(60101).await,
            vec![(200002, 0, Some(754), 0, None)],
            "60101: one NPC screen, no buttons"
        );

        let text = load_dialog_screen_text(&pool)
            .await
            .expect("load_dialog_screen_text must succeed");
        for screen in [200003, 200004] {
            assert!(
                text.get(&screen).is_some_and(|t| !t.is_empty()),
                "bark screen {screen} must carry text for npc_bark"
            );
        }

        let monologues = load_monologue_dialog_ids(&pool)
            .await
            .expect("load_monologue_dialog_ids must succeed");
        for dialog in [60100, 60101] {
            assert!(
                !monologues.contains(&dialog),
                "dialog {dialog} is spoken by Airman Lance, not the player"
            );
        }
    }
}
