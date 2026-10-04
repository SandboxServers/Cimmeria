//! Live-DB guards for the Debug Area services plaza (Z2) and dummies range
//! (Z3), packet DA-02 (`docs/content/debug-area.md`): spawns 13000-13199,
//! templates 1300-1329, chains 13000-13099.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a DA-02 spawn in another world, not stationary (it would path and warn
//!   off-mesh), outside the plaza ring or the dummies line, on top of another
//!   NPC, carrying a `DebugHub_*` tag (the hub's tests count that prefix),
//!   or using a template that is missing or shows no name;
//! * a dummy that is not a training dummy (it would fire back, D-DA7), a
//!   hostile dummy that cannot be shot, a friendly one that can, or a plaza
//!   NPC that became a dummy;
//! * the loader dropping `training_dummy` (every dummy would spawn as a
//!   mob that fights back);
//! * a chain-driven plaza NPC with no `interact_tag` chain, or with a chain
//!   that does something else (the granter must run `gm_ability_bulk`).
//!
//! Revert proof: drop `spawnlist_debug_area_plaza.sql` from
//! `db/database.sql` and every test here fails on its count; set a dummy's
//! `training_dummy` to false and `dummies_are_training_dummies_that_never_fight`
//! fails.
mod live_db {
    use std::collections::BTreeMap;

    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const WORLD: &str = "DebugArea";
    const FIRST_SPAWN: i32 = 13000;
    const LAST_SPAWN: i32 = 13199;
    /// Spawns 13100 and up are the dummies range.
    const FIRST_DUMMY_SPAWN: i32 = 13100;

    const PLAZA_CENTRE: (f32, f32) = (252.0, -923.0);
    /// The ring stands between these radii; the centre stays clear.
    const RING_MIN: f32 = 9.0;
    const RING_MAX: f32 = 12.0;
    const MIN_NPC_SPACING: f32 = 2.5;
    /// The plaza floor (navmesh) is 6.99 to 7.10.
    const PLAZA_Y: (f32, f32) = (6.5, 7.6);

    const DUMMY_LINE_Z: f32 = -872.0;
    const DUMMY_LINE_X: (f32, f32) = (240.0, 264.0);
    const DUMMY_Y: (f32, f32) = (6.0, 7.6);

    const HOSTILE_FACTION: i32 = 10;
    /// Hostile dummies: (template, level).
    const HOSTILE_DUMMIES: [(i32, i32); 4] = [(1310, 1), (1311, 10), (1312, 25), (1313, 50)];
    const FRIENDLY_DUMMY: i32 = 1314;

    /// The plaza NPCs a chain answers, and the action each chain runs.
    const CHAIN_DRIVEN: [(&str, &str); 7] = [
        ("DebugArea_AbilityGranter", "gm_ability_bulk"),
        ("DebugArea_AbilityReset", "gm_ability_bulk"),
        ("DebugArea_DialogNpc", "display_dialog"),
        ("DebugArea_LivewireTerminal", "start_minigame"),
        ("DebugArea_LootCrate", "open_loot"),
        ("DebugArea_MailClerk", "send_system_mail"),
        ("DebugArea_Auctioneer", "open_black_market"),
    ];

    async fn da02_spawns(pool: &sqlx::PgPool) -> Vec<SpawnRecord> {
        load_spawns_from_db(pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| (FIRST_SPAWN..=LAST_SPAWN).contains(&r.spawn_id))
            .collect()
    }

    fn xz(a: &SpawnRecord, b: &SpawnRecord) -> f32 {
        ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
    }

    /// Every DA-02 spawn is in world 1300, stationary, `DebugArea_*`-tagged,
    /// and uses a template that exists and shows a name. The loader's join
    /// drops a spawn whose template or world is missing, so the raw row count
    /// is checked against the loaded one.
    #[tokio::test]
    async fn every_da02_spawn_is_a_stationary_debug_area_npc() {
        let pool = require_db_or_skip!();
        let raw: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.spawnlist WHERE spawn_id BETWEEN $1 AND $2",
        )
        .bind(FIRST_SPAWN)
        .bind(LAST_SPAWN)
        .fetch_one(&pool)
        .await
        .unwrap();
        let spawns = da02_spawns(&pool).await;
        assert_eq!(
            spawns.len() as i64,
            raw,
            "every DA-02 row joins a template and world 1300"
        );
        assert_eq!(spawns.len(), 26, "21 plaza NPCs and 5 dummies");

        let world_id: i32 =
            sqlx::query_scalar("SELECT world_id FROM resources.worlds WHERE world = $1")
                .bind(WORLD)
                .fetch_one(&pool)
                .await
                .expect("world DebugArea is seeded (DA-01)");
        assert_eq!(world_id, 1300);

        for s in &spawns {
            assert_eq!(s.world_name, WORLD, "spawn {} world", s.spawn_id);
            assert!(s.is_stationary, "spawn {} must be stationary", s.spawn_id);
            let tag = s.tag.as_deref().unwrap_or("");
            assert!(
                tag.starts_with("DebugArea_"),
                "spawn {} tag {tag:?} must start DebugArea_ (D-DA6)",
                s.spawn_id
            );
            assert!(s.name_id.is_some(), "spawn {} shows no name", s.spawn_id);
        }
        let named: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources.spawnlist s \
               JOIN resources.entity_templates t USING (template_id) \
               JOIN resources.texts x ON x.moniker_id = t.name_id \
              WHERE s.spawn_id BETWEEN $1 AND $2 AND x.text <> ''",
        )
        .bind(FIRST_SPAWN)
        .bind(LAST_SPAWN)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(named, raw, "every name is a shipped moniker with text");

        let mut tags: BTreeMap<&str, i32> = BTreeMap::new();
        for s in &spawns {
            let prev = tags.insert(s.tag.as_deref().unwrap(), s.spawn_id);
            assert!(prev.is_none(), "tag {:?} used twice", s.tag);
        }
    }

    /// The plaza NPCs stand on the ring around the plaza centre, on its
    /// floor, at least 2.5 apart; the dummies stand on their line.
    #[tokio::test]
    async fn plaza_and_dummies_stand_inside_their_zones() {
        let pool = require_db_or_skip!();
        let spawns = da02_spawns(&pool).await;
        let (plaza, dummies): (Vec<&SpawnRecord>, Vec<&SpawnRecord>) =
            spawns.iter().partition(|s| s.spawn_id < FIRST_DUMMY_SPAWN);
        assert_eq!(plaza.len(), 21);
        assert_eq!(dummies.len(), 5);

        for s in &plaza {
            let r = ((s.x - PLAZA_CENTRE.0).powi(2) + (s.z - PLAZA_CENTRE.1).powi(2)).sqrt();
            assert!(
                (RING_MIN..=RING_MAX).contains(&r),
                "spawn {} is {r:.2} from the plaza centre",
                s.spawn_id
            );
            assert!(
                (PLAZA_Y.0..=PLAZA_Y.1).contains(&s.y),
                "spawn {} y {} is off the plaza floor",
                s.spawn_id,
                s.y
            );
        }
        for s in &dummies {
            assert!((s.z - DUMMY_LINE_Z).abs() < 0.5, "dummy {} z", s.spawn_id);
            assert!(
                (DUMMY_LINE_X.0..=DUMMY_LINE_X.1).contains(&s.x),
                "dummy {} x {}",
                s.spawn_id,
                s.x
            );
            assert!(
                (DUMMY_Y.0..=DUMMY_Y.1).contains(&s.y),
                "dummy {} y {}",
                s.spawn_id,
                s.y
            );
        }
        for (i, a) in spawns.iter().enumerate() {
            for b in &spawns[i + 1..] {
                assert!(
                    xz(a, b) >= MIN_NPC_SPACING,
                    "spawns {} and {} are {:.2} apart",
                    a.spawn_id,
                    b.spawn_id,
                    xz(a, b)
                );
            }
        }
    }

    /// D-DA7: the dummies load as training dummies (the loader selects the
    /// column), hostile ones at levels 1, 10, 25 and 50 that players can
    /// shoot, and one friendly heal target they cannot. Nothing else in the
    /// seed is a training dummy.
    #[tokio::test]
    async fn dummies_are_training_dummies_that_never_fight() {
        let pool = require_db_or_skip!();
        let spawns = da02_spawns(&pool).await;
        let dummies: BTreeMap<i32, &SpawnRecord> = spawns
            .iter()
            .filter(|s| s.spawn_id >= FIRST_DUMMY_SPAWN)
            .map(|s| (s.template_id, s))
            .collect();
        for (template, level) in HOSTILE_DUMMIES {
            let d = dummies
                .get(&template)
                .unwrap_or_else(|| panic!("template {template} is placed on the line"));
            assert!(d.training_dummy, "template {template} is a training dummy");
            assert_eq!(d.faction, Some(HOSTILE_FACTION), "template {template}");
            assert_eq!(d.level, Some(level), "template {template}");
            assert_eq!(d.interaction_type, 0, "template {template}");
            assert!(
                d.loot_table_id.is_none(),
                "template {template} drops nothing"
            );
        }
        let f = dummies[&FRIENDLY_DUMMY];
        assert!(f.training_dummy);
        assert_ne!(f.faction, Some(HOSTILE_FACTION), "players cannot shoot it");
        assert!(f.faction.is_some());

        let marked: Vec<i32> = sqlx::query_scalar(
            "SELECT template_id FROM resources.entity_templates \
              WHERE training_dummy ORDER BY template_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(marked, vec![1310, 1311, 1312, 1313, 1314]);
        for s in spawns.iter().filter(|s| s.spawn_id < FIRST_DUMMY_SPAWN) {
            assert!(
                !s.training_dummy,
                "plaza spawn {} is not a dummy",
                s.spawn_id
            );
        }

        let templates = load_spawn_templates(&pool).await.expect("templates load");
        assert!(
            templates[&1310].training_dummy,
            "the template prototype (GM .spawn) carries the flag too"
        );
    }

    /// Each chain-driven plaza NPC has exactly one `interact_tag` chain, and
    /// that chain runs the action that NPC exists for. The granter and the
    /// reset NPC run `gm_ability_bulk` with their own change.
    #[tokio::test]
    async fn chain_driven_plaza_npcs_answer_through_their_own_chain() {
        let pool = require_db_or_skip!();
        for (tag, action) in CHAIN_DRIVEN {
            let rows = sqlx::query(
                "SELECT a.chain_id, a.action_type, a.params::text AS params \
                   FROM resources.content_triggers t \
                   JOIN resources.content_actions a USING (chain_id) \
                  WHERE t.event_type = 'interact_tag' AND t.event_key = $1",
            )
            .bind(tag)
            .fetch_all(&pool)
            .await
            .unwrap();
            assert_eq!(rows.len(), 1, "{tag}: one chain, one action");
            let chain: i32 = rows[0].get("chain_id");
            assert!((13000..13100).contains(&chain), "{tag}: chain {chain}");
            assert_eq!(rows[0].get::<String, _>("action_type"), action, "{tag}");
            let params: String = rows[0].get("params");
            match tag {
                "DebugArea_AbilityGranter" => {
                    assert!(params.contains("grant_all"), "{tag}: {params}")
                }
                "DebugArea_AbilityReset" => assert!(params.contains("reset"), "{tag}: {params}"),
                _ => {}
            }
        }
    }

    /// The munitions vendor sells all fifteen special-ammo reserve stacks
    /// and a weapon for each family, everything at 1 naquadah.
    #[tokio::test]
    async fn munitions_vendor_sells_every_special_ammo_stack() {
        let pool = require_db_or_skip!();
        let list: Option<i32> = sqlx::query_scalar(
            "SELECT buy_item_list FROM resources.entity_templates WHERE template_id = 1302",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(list, Some(1300));
        let rows: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT li.design_id, li.naquadah FROM resources.item_list_items li \
               JOIN resources.items i ON i.item_id = li.design_id \
              WHERE li.item_list_id = 1300 ORDER BY li.item_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 19, "every row names a real item");
        assert!(rows.iter().all(|&(_, price)| price == 1));
        let ammo: Vec<i32> =
            sqlx::query_scalar("SELECT item_id FROM resources.ammo_item_types ORDER BY item_id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(ammo.len(), 15);
        for id in ammo {
            assert!(
                rows.iter().any(|&(design, _)| design == id),
                "ammo reserve item {id} is on sale"
            );
        }
    }
}
