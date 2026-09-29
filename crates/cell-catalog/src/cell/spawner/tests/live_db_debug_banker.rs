//! Live-DB guards for the stasis-room debug hub's Banker (bank-vault BV-04):
//! template 370 and `spawnlist` 470 (`docs/content/debug-hub.md`).
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a Banker template that lost `INT_BANKER` (the click derives nothing and
//!   dead-ends), picked up a bit or column that answers the click first (a
//!   trainer list, a vendor list or bit), names an org vault, or shows no
//!   name;
//! * a killable Banker (faction 10): death overwrites the interaction with
//!   `Loot` and the respawn tick never restores it;
//! * the Banker placed outside the stasis room, on the respawner, or on top
//!   of another hub NPC or crafting station.
//!
//! Each was proven to fail with the BV-04 seed rows removed (worknote
//! `docs/analysis/bank-vault/worknotes/bv-04.md`).
mod live_db {
    use sqlx::Row;

    use cimmeria_entity::cell_entity::VaultScope;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const BANKER: i32 = 370;
    const BANKER_SPAWN: i32 = 470;
    const BANKER_TAG: &str = "DebugHub_Banker";
    /// `DN_npc_OmegaSite_Banker` ('Storage Officer'), a moniker the client
    /// ships.
    const BANKER_NAME: i32 = 29462;
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

    /// Template 370 is a personal Banker and nothing else: exactly the
    /// `INT_BANKER` bit, `vault_scope = 'personal'`, a shipped name, and no
    /// column that answers a click before the Banker arm or makes it a pet or
    /// a mob that can die.
    #[tokio::test]
    async fn banker_template_carries_its_role_fields() {
        let pool = require_db_or_skip!();
        let row = sqlx::query(
            "SELECT t.class, t.faction, t.interaction_type, t.flags, t.name_id, \
                    t.vault_scope, t.trainer_ability_list_id, t.buy_item_list, \
                    t.sell_item_list, t.repair_item_list, t.recharge_item_list, \
                    t.loot_table_id, t.ability_set_id, x.text \
               FROM resources.entity_templates t \
               LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
              WHERE t.template_id = $1",
        )
        .bind(BANKER)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .expect("template 370 must be seeded");

        let got = (
            row.get::<String, _>("class"),
            row.get::<i64, _>("interaction_type"),
            row.get::<String, _>("vault_scope"),
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
                cimmeria_entity::interaction_flags::INT_BANKER,
                "personal".to_string(),
                0,
                Some(BANKER_NAME),
                None,
                [None; 4],
                None,
                None,
            ),
            "template 370 role columns"
        );
        let faction: Option<i32> = row.get("faction");
        assert!(
            faction.is_some_and(|f| f != HOSTILE_FACTION),
            "a Banker must not be killable (faction {faction:?})"
        );
        let name: Option<String> = row.get("text");
        assert!(
            name.as_deref().is_some_and(|n| !n.trim().is_empty()),
            "name_id {BANKER_NAME} must resolve to non-empty text, got {name:?}"
        );

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert_eq!(
            lists.get(&BANKER),
            None,
            "a trainer list would open the trainer before the Banker arm"
        );
    }

    /// Spawn 470 is the one placement of template 370: world 12, inside the
    /// stasis room, on its floor, clear of the respawner and of every other
    /// hub NPC and crafting station, loading as a personal Banker, and not a
    /// respawning or aggression-overridden mob; it holds position by flag
    /// (`is_stationary`).
    #[tokio::test]
    async fn banker_spawn_sits_in_the_stasis_room_hub() {
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

        let placed: Vec<&SpawnRecord> =
            records.iter().filter(|r| r.template_id == BANKER).collect();
        assert_eq!(placed.len(), 1, "template 370 is placed exactly once");
        let s = placed[0];
        assert_eq!(s.spawn_id, BANKER_SPAWN);
        assert_eq!(s.tag.as_deref(), Some(BANKER_TAG));
        assert_eq!(s.world_name, "Castle_CellBlock");
        assert_eq!(
            s.interaction_type,
            cimmeria_entity::interaction_flags::INT_BANKER
        );
        assert_eq!(s.vault_scope, VaultScope::Personal);

        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));
        assert!(
            region_contains_xz(&room.points, s.x, s.z),
            "spawn 470 at ({}, {}) must stand inside {ROOM}",
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
            "spawn 470 is {clearance:.2} units from the respawner"
        );

        // Every other NPC standing in the room: the hub line, the pet
        // trainer and, once crafting's CR-11 lands, its stations.
        let neighbours: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.spawn_id != BANKER_SPAWN && r.world_name == "Castle_CellBlock")
            .filter(|r| region_contains_xz(&room.points, r.x, r.z))
            .collect();
        assert!(
            neighbours.len() >= 6,
            "the hub's six NPCs must be in the room too: {:?}",
            neighbours.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
        );
        for other in neighbours {
            let d = xz_distance(s, other);
            assert!(
                d >= MIN_NPC_SPACING,
                "spawn 470 is {d:.2} units from spawn {} ({:?})",
                other.spawn_id,
                other.tag
            );
        }
        assert_eq!(s.respawn_secs, None);
        assert!(s.is_stationary, "every hub spawn holds position");
        assert_eq!(s.aggression_override, None);
    }
}
