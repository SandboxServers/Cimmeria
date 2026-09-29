//! Live-DB guards for the stasis-room debug hub's Team and Command Bankers
//! (bank-vault BV-10a): templates 371 and 372 and `spawnlist` 471 and 472
//! (`docs/content/debug-hub.md`).
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * an org Banker template that lost `INT_BANKER` (the click derives
//!   nothing), names the wrong vault (`personal`, or Team for Command), picked
//!   up a column that answers the click first (a trainer list, a vendor list),
//!   or shows no name;
//! * a killable Banker (faction 10): death overwrites the interaction with
//!   `Loot` and the respawn tick never restores it;
//! * a Banker placed outside the stasis room, on the respawner, or on top of
//!   another hub NPC, crafting station or the other Bankers.
//!
//! Each was proven to fail with the BV-10a seed rows removed (worknote
//! `docs/analysis/bank-vault/worknotes/bv-10a.md`).
mod live_db {
    use sqlx::Row;

    use cimmeria_entity::cell_entity::VaultScope;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `(template, spawn, tag, vault_scope column, loaded scope)`.
    const ORG_BANKERS: [(i32, i32, &str, &str, VaultScope); 2] = [
        (371, 471, "DebugHub_TeamBanker", "team", VaultScope::Team),
        (
            372,
            472,
            "DebugHub_CommandBanker",
            "command",
            VaultScope::Command,
        ),
    ];
    /// `DN_npc_OmegaSite_Banker` ('Storage Officer'), a moniker the client
    /// ships; no shipped moniker names a Team or Command banker.
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

    /// Templates 371 and 372 are org Bankers and nothing else: exactly the
    /// `INT_BANKER` bit, their own `vault_scope`, a shipped name, and no
    /// column that answers a click before the Banker arm or makes them a pet
    /// or a mob that can die.
    #[tokio::test]
    async fn org_banker_templates_carry_their_role_fields() {
        let pool = require_db_or_skip!();
        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        for (template, _, _, scope, _) in ORG_BANKERS {
            let row = sqlx::query(
                "SELECT t.class, t.faction, t.interaction_type, t.flags, t.name_id, \
                        t.vault_scope, t.trainer_ability_list_id, t.buy_item_list, \
                        t.sell_item_list, t.repair_item_list, t.recharge_item_list, \
                        t.loot_table_id, t.ability_set_id, x.text \
                   FROM resources.entity_templates t \
                   LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
                  WHERE t.template_id = $1",
            )
            .bind(template)
            .fetch_optional(&pool)
            .await
            .expect("entity_templates query")
            .unwrap_or_else(|| panic!("template {template} must be seeded"));

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
                    scope.to_string(),
                    0,
                    Some(BANKER_NAME),
                    None,
                    [None; 4],
                    None,
                    None,
                ),
                "template {template} role columns"
            );
            let faction: Option<i32> = row.get("faction");
            assert!(
                faction.is_some_and(|f| f != HOSTILE_FACTION),
                "template {template}: a Banker must not be killable (faction {faction:?})"
            );
            let name: Option<String> = row.get("text");
            assert!(
                name.as_deref().is_some_and(|n| !n.trim().is_empty()),
                "template {template}: name_id {BANKER_NAME} must resolve to text, got {name:?}"
            );
            assert_eq!(
                lists.get(&template),
                None,
                "template {template}: a trainer list would open the trainer before the Banker arm"
            );
        }
    }

    /// Spawns 471 and 472 are the one placement each of templates 371 and
    /// 372: world 12, inside the stasis room, on its floor, clear of the
    /// respawner and of every other NPC in the room (the hub, the pets and
    /// crafting NPCs, the Storage Officer and each other), loading as a Team
    /// and a Command Banker, and not respawning or
    /// aggression-overridden; they hold position by flag (`is_stationary`).
    #[tokio::test]
    async fn org_banker_spawns_sit_in_the_stasis_room_hub() {
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
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == STASIS_RESPAWNER)
            .expect("respawner 8 must be seeded");
        let in_room: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.world_name == "Castle_CellBlock")
            .filter(|r| region_contains_xz(&room.points, r.x, r.z))
            .collect();

        for (template, spawn, tag, _, scope) in ORG_BANKERS {
            let placed: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.template_id == template)
                .collect();
            assert_eq!(
                placed.len(),
                1,
                "template {template} is placed exactly once"
            );
            let s = placed[0];
            assert_eq!(s.spawn_id, spawn);
            assert_eq!(s.tag.as_deref(), Some(tag));
            assert_eq!(s.world_name, "Castle_CellBlock");
            assert_eq!(
                s.interaction_type,
                cimmeria_entity::interaction_flags::INT_BANKER
            );
            assert_eq!(s.vault_scope, scope, "spawn {spawn} vault scope");

            assert!(
                region_contains_xz(&room.points, s.x, s.z),
                "spawn {spawn} at ({}, {}) must stand inside {ROOM}",
                s.x,
                s.z
            );
            assert!((s.y - FLOOR_Y).abs() < 0.5, "on the floor, got y {}", s.y);
            let clearance =
                ((s.x - respawner.pos[0]).powi(2) + (s.z - respawner.pos[2]).powi(2)).sqrt();
            assert!(
                clearance >= MIN_RESPAWNER_CLEARANCE,
                "spawn {spawn} is {clearance:.2} units from the respawner"
            );

            let neighbours: Vec<&&SpawnRecord> =
                in_room.iter().filter(|r| r.spawn_id != spawn).collect();
            assert!(
                neighbours.iter().any(|r| r.spawn_id == 470),
                "the Storage Officer (470) must be in the room too: {:?}",
                neighbours.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
            );
            for other in neighbours {
                let d = ((s.x - other.x).powi(2) + (s.z - other.z).powi(2)).sqrt();
                assert!(
                    d >= MIN_NPC_SPACING,
                    "spawn {spawn} is {d:.2} units from spawn {} ({:?})",
                    other.spawn_id,
                    other.tag
                );
            }
            assert_eq!(s.respawn_secs, None);
            assert!(s.is_stationary, "every hub spawn holds position");
            assert_eq!(s.aggression_override, None);
        }
    }
}
