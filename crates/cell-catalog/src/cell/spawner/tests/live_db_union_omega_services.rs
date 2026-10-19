//! Live-DB guards for Union Omega's hub banker and organization registrars
//! (UO-10, D-UO14): templates 63810 (Banker), 63811 (Team) and 63830
//! (Command), and `spawnlist` 63820, 63821 and 63840 in world 18
//! `Omega_Site`. They are clones of the debug hub's rows 370, 330 and 331.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a banker that lost `INT_BANKER`, a registrar that lost `INT_Organization`
//!   or its own registrar set (7447 Team, 7448 Command; a template carrying
//!   both has no type), picked up a bit or column that answers the click
//!   first (a trainer list, a vendor list or bit), or shows no name;
//! * a killable service (faction 10), or a banker that is not personal;
//! * a service placed twice, outside the hub, on the respawner, or on top of
//!   another NPC there.
mod live_db {
    use sqlx::Row;

    use cimmeria_entity::cell_entity::VaultScope;
    use cimmeria_entity::interaction_flags::{INT_BANKER, INT_ORGANIZATION};

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `(template, spawn, tag)` for the banker and both registrars.
    const SERVICES: [(i32, i32, &str); 3] = [
        (63810, 63820, "UnionOmega_Banker"),
        (63811, 63821, "UnionOmega_TeamRegistrar"),
        (63830, 63840, "UnionOmega_CommandRegistrar"),
    ];
    /// The registrar templates and their registrar interaction set (7447
    /// Team, 7448 Command), as `org_registrar::registrar_type` reads them.
    const REGISTRARS: [(i32, i32); 2] = [(63811, 7447), (63830, 7448)];
    /// The Omega hub's respawner (UO-02 seed).
    const OMEGA_HUB_RESPAWNER: i32 = 180;
    const OMEGA_WORLD: &str = "Omega_Site";
    /// `DN_npc_OmegaSite_Banker` ('Storage Officer'), a moniker the client
    /// ships.
    const BANKER_NAME: i32 = 29462;
    /// `DN_npc_reg_OmegaSite_TeamCommandRegistrar` ('Organization
    /// Registrar'), a moniker the client ships.
    const REGISTRAR_NAME: i32 = 29068;
    /// `ENTITYFLAG_Pet`.
    const ENTITYFLAG_PET: i64 = 1024;
    /// The only faction a player can damage.
    const HOSTILE_FACTION: i32 = 10;
    const MIN_RESPAWNER_CLEARANCE: f32 = 3.0;
    const MIN_NPC_SPACING: f32 = 2.5;

    fn xz_distance(a: &SpawnRecord, b: &SpawnRecord) -> f32 {
        ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
    }

    /// Template 63810 is a personal Banker and nothing else: exactly the
    /// `INT_BANKER` bit, `vault_scope = 'personal'`, a shipped name, and no
    /// column that answers a click before the Banker arm or makes it a pet or
    /// a mob that can die.
    #[tokio::test]
    async fn omega_banker_carries_its_role_fields() {
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
        .bind(63810)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .expect("template 63810 must be seeded");

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
                INT_BANKER,
                "personal".to_string(),
                0,
                Some(BANKER_NAME),
                None,
                [None; 4],
                None,
                None,
            ),
            "template 63810 role columns"
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
            lists.get(&63810),
            None,
            "a trainer list would open the trainer before the Banker arm"
        );
    }

    /// Templates 63811 and 63830 are registrars and nothing else: exactly the
    /// `INT_Organization` bit, exactly their own registrar set, a shipped
    /// name, and no column that answers a click before the registrar arm or
    /// makes them a pet or a mob that can die.
    #[tokio::test]
    async fn omega_registrars_carry_their_role_fields() {
        let pool = require_db_or_skip!();
        for (template, set) in REGISTRARS {
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
            .unwrap_or_else(|| panic!("template {template} must be seeded"));

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
                "template {template} role columns"
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
        for (template, _) in REGISTRARS {
            assert_eq!(
                lists.get(&template),
                None,
                "a trainer list would open the trainer before the registrar arm"
            );
        }
    }

    /// Each service stands once in Omega's hub: the one placement of its
    /// template, in `Omega_Site`, under its named spawn and tag, holding
    /// position, not respawning and not aggression-overridden. The banker
    /// loads personal. It stands clear of the hub respawner and of every
    /// other Omega NPC.
    #[tokio::test]
    async fn omega_services_stand_once_in_the_hub() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == OMEGA_HUB_RESPAWNER)
            .unwrap_or_else(|| panic!("respawner {OMEGA_HUB_RESPAWNER} must be seeded"));

        for (template, spawn, tag) in SERVICES {
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
            assert_eq!(s.spawn_id, spawn, "template {template} spawn id");
            assert_eq!(s.world_name, OMEGA_WORLD, "spawn {spawn} world");
            assert_eq!(s.tag.as_deref(), Some(tag), "spawn {spawn} tag");
            assert!(s.is_stationary, "spawn {spawn} holds position");
            assert_eq!(s.respawn_secs, None, "spawn {spawn} does not respawn");
            assert_eq!(
                s.aggression_override, None,
                "spawn {spawn} is not aggression-overridden"
            );
            if template == 63810 {
                assert_eq!(s.vault_scope, VaultScope::Personal);
            }

            let clearance =
                ((s.x - respawner.pos[0]).powi(2) + (s.z - respawner.pos[2]).powi(2)).sqrt();
            assert!(
                clearance >= MIN_RESPAWNER_CLEARANCE,
                "spawn {spawn} is {clearance:.2} units from respawner {OMEGA_HUB_RESPAWNER}"
            );
            for other in records
                .iter()
                .filter(|r| r.spawn_id != spawn && r.world_name == OMEGA_WORLD)
            {
                let d = xz_distance(s, other);
                assert!(
                    d >= MIN_NPC_SPACING,
                    "spawn {spawn} is {d:.2} units from spawn {} ({:?})",
                    other.spawn_id,
                    other.tag
                );
            }
        }
    }
}
