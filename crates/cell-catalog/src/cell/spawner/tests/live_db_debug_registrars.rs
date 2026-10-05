//! Live-DB guards for the stasis-room debug hub's organization registrars
//! (organizations campaign ORG-05): templates 330 (Team) and 331 (Command)
//! and `spawnlist` 430 and 431 (`docs/content/debug-hub.md`).
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a registrar template that lost `INT_Organization` (no organization
//!   cursor, and the registrar arm never matches) or its type's registrar
//!   interaction set (7447 Team, 7448 Command), carries both sets (no type),
//!   picked up a bit or column that answers the click first (a trainer list,
//!   a vendor list or bit, the Banker or DHD bit), or shows no name;
//! * a killable registrar (faction 10);
//! * a registrar placed outside the stasis room, on the respawner, or on top
//!   of another NPC there.
//!
//! Each was proven to fail with the ORG-05 seed rows removed (worknote
//! `docs/analysis/organizations/worknotes/org-05.md`).
mod live_db {
    use sqlx::Row;

    use cimmeria_entity::interaction_flags::INT_ORGANIZATION;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `(template, spawn, tag, registrar interaction set)`. The sets are the
    /// 2009 `INTERACTION_OrganizationRegisterTeam` / `...Command` ids the
    /// cell's `org_registrar::registrar_type` reads.
    const REGISTRARS: [(i32, i32, &str, i32); 2] = [
        (330, 430, "DebugHub_TeamRegistrar", 7447),
        (331, 431, "DebugHub_CommandRegistrar", 7448),
    ];
    const REGISTRAR_SETS: [i32; 2] = [7447, 7448];
    /// `DN_npc_reg_OmegaSite_TeamCommandRegistrar` ('Organization
    /// Registrar'), a moniker the client ships.
    const REGISTRAR_NAME: i32 = 29068;
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

    /// Templates 330 and 331 are registrars and nothing else: exactly the
    /// `INT_Organization` bit, exactly their own registrar set, a shipped
    /// name, and no column that answers a click before the registrar arm or
    /// makes them a pet or a mob that can die.
    #[tokio::test]
    async fn registrar_templates_carry_their_role_fields() {
        let pool = require_db_or_skip!();
        for (template, _, tag, set) in REGISTRARS {
            let row = sqlx::query(
                "SELECT t.class, t.faction, t.interaction_type, t.flags, t.name_id, \
                        t.static_interaction_sets, t.trainer_ability_list_id, \
                        t.buy_item_list, t.sell_item_list, t.repair_item_list, \
                        t.recharge_item_list, t.loot_table_id, t.ability_set_id, x.text \
                   FROM resources.entity_templates t \
                   LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
                  WHERE t.template_id = $1",
            )
            .bind(template)
            .fetch_optional(&pool)
            .await
            .expect("entity_templates query")
            .unwrap_or_else(|| panic!("template {template} ({tag}) must be seeded"));

            let got = (
                row.get::<String, _>("class"),
                row.get::<i64, _>("interaction_type"),
                row.get::<Vec<i32>, _>("static_interaction_sets"),
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
                    INT_ORGANIZATION,
                    vec![set],
                    0,
                    Some(REGISTRAR_NAME),
                    None,
                    [None; 4],
                    None,
                    None,
                ),
                "template {template} ({tag}) role columns"
            );
            let faction: Option<i32> = row.get("faction");
            assert!(
                faction.is_some_and(|f| f != HOSTILE_FACTION),
                "a registrar must not be killable (template {template}, faction {faction:?})"
            );
            let name: Option<String> = row.get("text");
            assert!(
                name.as_deref().is_some_and(|n| !n.trim().is_empty()),
                "name_id {REGISTRAR_NAME} must resolve to non-empty text, got {name:?}"
            );
        }

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        for (template, ..) in REGISTRARS {
            assert_eq!(
                lists.get(&template),
                None,
                "a trainer list would open the trainer before the registrar arm"
            );
        }
    }

    /// No other template is a registrar: a third `INT_Organization` template
    /// with a registrar set would open a founding dialog somewhere nobody
    /// documented.
    #[tokio::test]
    async fn only_the_hub_templates_are_registrars() {
        let pool = require_db_or_skip!();
        let ids: Vec<i32> = sqlx::query_scalar(
            "SELECT template_id FROM resources.entity_templates \
              WHERE interaction_type & $1 <> 0 AND static_interaction_sets && $2 \
              ORDER BY template_id",
        )
        .bind(INT_ORGANIZATION)
        .bind(REGISTRAR_SETS.to_vec())
        .fetch_all(&pool)
        .await
        .expect("registrar template query");
        assert_eq!(ids, vec![330, 331]);
    }

    /// Spawns 430 and 431 are the one placement of each registrar: world 12,
    /// inside the stasis room, on its floor, clear of the respawner and of
    /// every other NPC standing there, loading with the registrar's bit and
    /// set, and not a respawning or aggression-overridden mob; they hold
    /// position by flag (`is_stationary`).
    #[tokio::test]
    async fn registrar_spawns_sit_in_the_stasis_room_hub() {
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

        for (template, spawn, tag, set) in REGISTRARS {
            let placed: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.template_id == template && r.world_name == "Castle_CellBlock")
                .collect();
            // Every placement of the template, in any world: the hub's and
            // the Debug Area plaza's copy (DA-02), nothing else.
            let plaza = if template == 330 { 13013 } else { 13014 };
            let mut everywhere: Vec<(i32, &str)> = records
                .iter()
                .filter(|r| r.template_id == template)
                .map(|r| (r.spawn_id, r.world_name.as_str()))
                .collect();
            everywhere.sort_unstable();
            assert_eq!(
                everywhere,
                vec![(spawn, "Castle_CellBlock"), (plaza, "DebugArea")],
                "template {template} is placed in the hub and the plaza only"
            );
            assert_eq!(
                placed.len(),
                1,
                "template {template} is placed once in the hub"
            );
            let s = placed[0];
            assert_eq!(s.spawn_id, spawn);
            assert_eq!(s.tag.as_deref(), Some(tag));
            assert_eq!(s.world_name, "Castle_CellBlock");
            assert_eq!(s.interaction_type, INT_ORGANIZATION);
            assert_eq!(s.static_interaction_sets, vec![set]);

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

            let neighbours: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.spawn_id != spawn && r.world_name == "Castle_CellBlock")
                .filter(|r| region_contains_xz(&room.points, r.x, r.z))
                .collect();
            assert!(
                neighbours.len() >= 8,
                "the hub's other NPCs must be in the room too: {:?}",
                neighbours.iter().map(|r| r.spawn_id).collect::<Vec<_>>()
            );
            for other in neighbours {
                let d = xz_distance(s, other);
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
