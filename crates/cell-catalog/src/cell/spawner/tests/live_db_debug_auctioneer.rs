//! Live-DB guards for the stasis-room debug hub's Black Market auctioneer
//! (Black Market BM-07): template 305 and `spawnlist` 405
//! (`docs/content/debug-hub.md`).
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * an auctioneer template that lost `INT_AUCTION` (the NPC derives no
//!   `Auctioneer` at spawn, so `open_black_market` refuses every click),
//!   picked up a bit or column that answers the click before its chain (a
//!   trainer list, a vendor list or bit, a Banker bit), or shows no name;
//! * a killable auctioneer (faction 10): death overwrites the interaction
//!   with `Loot` and the respawn tick never restores it;
//! * another template carrying `INT_AUCTION`, which would make that NPC a
//!   Black Market terminal (the BM-07 authority rule is "only a seeded
//!   auctioneer");
//! * the auctioneer placed outside the stasis room, on the respawner, or on
//!   top of another hub NPC or crafting station, or under another tag than
//!   chain 5030's.
mod live_db {
    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const AUCTIONEER: i32 = 305;
    const AUCTIONEER_SPAWN: i32 = 405;
    /// The `interact_tag` chain 5030 fires on.
    const AUCTIONEER_TAG: &str = "BlackMarket_Auctioneer";
    /// `DN_npc_MgVen_Machra_Tollana` ('Machra'), a moniker the client ships.
    const AUCTIONEER_NAME: i32 = 7133;
    /// `ENTITYFLAG_Pet`.
    const ENTITYFLAG_PET: i64 = 1024;
    /// The only faction a player can damage.
    const HOSTILE_FACTION: i32 = 10;

    const ROOM: &str = "Castle_Cellblock.Region1";
    const STASIS_RESPAWNER: i32 = 8;
    const MIN_RESPAWNER_CLEARANCE: f32 = 5.0;
    const MIN_NPC_SPACING: f32 = 2.5;
    const FLOOR_Y: f32 = 73.472;

    fn xz_distance(a: &SpawnRecord, b: &SpawnRecord) -> f32 {
        ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
    }

    /// Template 305 is an auctioneer and nothing else: exactly the
    /// `INT_AUCTION` bit, a shipped name, and no column that answers a click
    /// before its chain or makes it a pet or a mob that can die.
    #[tokio::test]
    async fn auctioneer_template_carries_its_role_fields() {
        let pool = require_db_or_skip!();
        let row = sqlx::query(
            "SELECT t.class, t.faction, t.interaction_type, t.flags, t.name_id, \
                    t.trainer_ability_list_id, t.buy_item_list, t.sell_item_list, \
                    t.repair_item_list, t.recharge_item_list, t.loot_table_id, \
                    t.ability_set_id, t.components, x.text \
               FROM resources.entity_templates t \
               LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
              WHERE t.template_id = $1",
        )
        .bind(AUCTIONEER)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .expect("template 305 must be seeded");

        let got = (
            row.get::<String, _>("class"),
            row.get::<i64, _>("interaction_type"),
            row.get::<i64, _>("flags") & ENTITYFLAG_PET,
            row.get::<Option<i32>, _>("name_id"),
            row.get::<Option<i32>, _>("trainer_ability_list_id"),
            [
                row.get::<Option<i32>, _>("buy_item_list"),
                row.get::<Option<i32>, _>("sell_item_list"),
                row.get::<Option<i32>, _>("repair_item_list"),
                row.get::<Option<i32>, _>("recharge_item_list"),
            ],
            row.get::<Option<i32>, _>("loot_table_id"),
            row.get::<Option<i32>, _>("ability_set_id"),
        );
        assert_eq!(
            got,
            (
                "mob".to_string(),
                cimmeria_entity::interaction_flags::INT_AUCTION,
                0,
                Some(AUCTIONEER_NAME),
                None,
                [None; 4],
                None,
                None,
            ),
            "template 305 role columns"
        );
        let faction: Option<i32> = row.get("faction");
        assert!(
            faction.is_some_and(|f| f != HOSTILE_FACTION),
            "an auctioneer must not be killable (faction {faction:?})"
        );
        let name: Option<String> = row.get("text");
        assert!(
            name.as_deref().is_some_and(|n| !n.trim().is_empty()),
            "name_id {AUCTIONEER_NAME} must resolve to non-empty text, got {name:?}"
        );
        let components: Option<Vec<String>> = row.get("components");
        assert!(
            components.is_some_and(|c| !c.is_empty()),
            "a body with no components is invisible"
        );

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert_eq!(
            lists.get(&AUCTIONEER),
            None,
            "a trainer list would open the trainer before the chain"
        );
    }

    /// Template 305 is the only template with `INT_AUCTION`: the bit is the
    /// Black Market's authority marker, so any other carrier would be a
    /// Black Market terminal the moment a chain opened it.
    #[tokio::test]
    async fn only_the_auctioneer_template_carries_the_auction_bit() {
        let pool = require_db_or_skip!();
        let carriers: Vec<i32> = sqlx::query_scalar(
            "SELECT template_id FROM resources.entity_templates \
              WHERE interaction_type & $1 <> 0 ORDER BY template_id",
        )
        .bind(cimmeria_entity::interaction_flags::INT_AUCTION)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(carriers, vec![AUCTIONEER]);
    }

    /// Spawn 405 is the one placement of template 305: world 12, tagged for
    /// chain 5030, inside the stasis room, on its floor, clear of the
    /// respawner and of every other NPC in the room, and not a respawning,
    /// holding or aggression-overridden mob.
    #[tokio::test]
    async fn auctioneer_spawn_sits_in_the_stasis_room_hub() {
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

        let placed: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.template_id == AUCTIONEER)
            .collect();
        assert_eq!(placed.len(), 1, "template 305 is placed exactly once");
        let s = placed[0];
        assert_eq!(s.spawn_id, AUCTIONEER_SPAWN);
        assert_eq!(s.tag.as_deref(), Some(AUCTIONEER_TAG));
        assert_eq!(s.world_name, "Castle_CellBlock");
        assert_eq!(
            s.interaction_type,
            cimmeria_entity::interaction_flags::INT_AUCTION
        );
        let tagged: Vec<i32> = records
            .iter()
            .filter(|r| r.tag.as_deref() == Some(AUCTIONEER_TAG))
            .map(|r| r.spawn_id)
            .collect();
        assert_eq!(
            tagged,
            vec![AUCTIONEER_SPAWN],
            "chain 5030's tag names the auctioneer and nothing else"
        );

        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));
        assert!(
            region_contains_xz(&room.points, s.x, s.z),
            "spawn 405 at ({}, {}) must stand inside {ROOM}",
            s.x,
            s.z
        );
        assert!((s.y - FLOOR_Y).abs() < 0.5, "on the floor, got y {}", s.y);
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == STASIS_RESPAWNER)
            .expect("respawner 8 must be seeded");
        let clearance =
            ((s.x - respawner.pos[0]).powi(2) + (s.z - respawner.pos[2]).powi(2)).sqrt();
        assert!(
            clearance >= MIN_RESPAWNER_CLEARANCE,
            "spawn 405 is {clearance:.2} units from the respawner"
        );

        let neighbours: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.spawn_id != AUCTIONEER_SPAWN && r.world_name == "Castle_CellBlock")
            .filter(|r| region_contains_xz(&room.points, r.x, r.z))
            .collect();
        assert!(
            neighbours.len() >= 12,
            "the hub's NPCs must be in the room too: {:?}",
            neighbours.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
        );
        for other in neighbours {
            let d = xz_distance(s, other);
            assert!(
                d >= MIN_NPC_SPACING,
                "spawn 405 is {d:.2} units from spawn {} ({:?})",
                other.spawn_id,
                other.tag
            );
        }
        assert_eq!(s.respawn_secs, None);
        assert!(!s.is_stationary);
        assert_eq!(s.aggression_override, None);
    }
}
