//! Live-DB guards for the stasis-room debug hub's pet trainer (pets
//! campaign PT-07): template 360, `spawnlist` 450 and trainer list 350
//! (`docs/content/debug-hub.md`).
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a trainer template that lost its list or its trainer bit (the click
//!   falls through to nothing), picked up a pet flag, or shows no name;
//! * the trainer placed outside the stasis room, on the respawner, or on top
//!   of another hub NPC;
//! * list 350 offering an ability that is not a Goa'uld tree node (the
//!   trainer shows it greyed forever and WARNs `trainer_offered_unbound`),
//!   or losing Summon Straegis.
//!
//! Each was proven to fail with the PT-07 seed rows removed (worknote
//! `docs/analysis/pets/worknotes/pt-07.md`).
mod live_db {
    use std::collections::BTreeSet;

    use sqlx::Row;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const PET_TRAINER: i32 = 360;
    const PET_TRAINER_TAG: &str = "DebugHub_PetTrainer";
    const PET_TRAINER_LIST: i32 = 350;
    /// `DN_npc_trn_TBD_Harset_Goa'uldAdvancedSkills` ('Goa'uld Advanced
    /// Skills'): a moniker the client ships. No moniker says "Pet Trainer".
    const PET_TRAINER_NAME: i32 = 8000;
    /// 2826 Summon Straegis (the first pet, D-PT13), then the later pets'
    /// Servant Lord nodes.
    const OFFERED: [i32; 6] = [2826, 1643, 1644, 1645, 1652, 1654];
    /// `INT_Trainer`.
    const INT_TRAINER: i64 = 128;
    /// `ENTITYFLAG_Pet`.
    const ENTITYFLAG_PET: i64 = 1024;

    /// The other hub NPCs, for spacing.
    const HUB_TAGS: [&str; 5] = [
        "DebugHub_Vendor",
        "DebugHub_Trainer",
        "DebugHub_DialogNpc",
        "DebugHub_LivewireTerminal",
        "DebugHub_LootCrate",
    ];
    const ROOM: &str = "Castle_Cellblock.Region1";
    const STASIS_RESPAWNER: i32 = 8;
    const MIN_RESPAWNER_CLEARANCE: f32 = 5.0;
    const MIN_NPC_SPACING: f32 = 2.5;
    const FLOOR_Y: f32 = 73.472;

    fn xz_distance(a: &SpawnRecord, b: [f32; 3]) -> f32 {
        ((a.x - b[0]).powi(2) + (a.z - b[2]).powi(2)).sqrt()
    }

    async fn goauld_archetype_index(pool: &sqlx::PgPool) -> i32 {
        sqlx::query_scalar(
            "SELECT array_position(enum_range(NULL::resources.\"EArchetype\"), \
                    'ARCHETYPE_Goauld'::resources.\"EArchetype\") - 1",
        )
        .fetch_one(pool)
        .await
        .expect("EArchetype index query")
    }

    /// Template 360 is a placed trainer NPC: trainer bit, list 350, a
    /// shipped name, and nothing that makes it a pet, a vendor or a mob that
    /// fights.
    #[tokio::test]
    async fn pet_trainer_template_carries_its_role_fields() {
        let pool = require_db_or_skip!();
        let row = sqlx::query(
            "SELECT t.class, t.faction, t.interaction_type, t.flags, t.name_id, \
                    t.trainer_ability_list_id, t.buy_item_list, t.loot_table_id, \
                    t.ability_set_id, x.text \
               FROM resources.entity_templates t \
               LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
              WHERE t.template_id = $1",
        )
        .bind(PET_TRAINER)
        .fetch_optional(&pool)
        .await
        .expect("entity_templates query")
        .expect("template 360 must be seeded");

        let got = (
            row.get::<String, _>("class"),
            row.get::<Option<i32>, _>("faction"),
            row.get::<i64, _>("interaction_type"),
            row.get::<i64, _>("flags") & ENTITYFLAG_PET,
            row.get::<Option<i32>, _>("name_id"),
            row.get::<Option<i32>, _>("trainer_ability_list_id"),
            row.get::<Option<i32>, _>("buy_item_list"),
            row.get::<Option<i32>, _>("loot_table_id"),
            row.get::<Option<i32>, _>("ability_set_id"),
        );
        assert_eq!(
            got,
            (
                "mob".to_string(),
                Some(1),
                INT_TRAINER,
                0,
                Some(PET_TRAINER_NAME),
                Some(PET_TRAINER_LIST),
                None,
                None,
                None,
            ),
            "template 360 role columns"
        );
        let name: Option<String> = row.get("text");
        assert!(
            name.as_deref().is_some_and(|n| !n.trim().is_empty()),
            "name_id {PET_TRAINER_NAME} must resolve to non-empty text, got {name:?}"
        );

        let lists = load_template_trainer_lists(&pool)
            .await
            .expect("load_template_trainer_lists must succeed");
        assert_eq!(
            lists.get(&PET_TRAINER),
            Some(&PET_TRAINER_LIST),
            "the startup loader must map template 360 to list 350"
        );
    }

    /// Spawn 450 is the one placement of template 360: world 12, inside the
    /// stasis room, on its floor, clear of the respawner and of every other
    /// hub NPC, and not a respawning or aggression-overridden mob; it holds
    /// position by flag (`is_stationary`).
    #[tokio::test]
    async fn pet_trainer_spawn_sits_in_the_stasis_room_hub() {
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
            .filter(|r| r.template_id == PET_TRAINER)
            .collect();
        assert_eq!(placed.len(), 1, "template 360 is placed exactly once");
        let s = placed[0];
        assert_eq!(s.spawn_id, 450);
        assert_eq!(s.tag.as_deref(), Some(PET_TRAINER_TAG));
        assert_eq!(s.world_name, "Castle_CellBlock");
        let room = regions
            .iter()
            .find(|r| r.name == ROOM)
            .unwrap_or_else(|| panic!("point set {ROOM} must be loaded"));
        assert!(
            region_contains_xz(&room.points, s.x, s.z),
            "spawn 450 at ({}, {}) must stand inside {ROOM}",
            s.x,
            s.z
        );
        assert!((s.y - FLOOR_Y).abs() < 0.5, "on the floor, got y {}", s.y);
        let respawner = respawners
            .iter()
            .find(|r| r.respawner_id == STASIS_RESPAWNER)
            .expect("respawner 8 must be seeded");
        let clearance = xz_distance(s, respawner.pos);
        assert!(
            clearance >= MIN_RESPAWNER_CLEARANCE,
            "spawn 450 is {clearance:.2} units from the respawner"
        );
        for tag in HUB_TAGS {
            let other = records
                .iter()
                .find(|r| r.tag.as_deref() == Some(tag))
                .unwrap_or_else(|| panic!("hub spawn {tag} must exist"));
            let d = xz_distance(s, [other.x, other.y, other.z]);
            assert!(d >= MIN_NPC_SPACING, "spawn 450 is {d:.2} units from {tag}");
        }
        assert_eq!(s.respawn_secs, None);
        assert!(s.is_stationary, "every hub spawn holds position");
        assert_eq!(s.aggression_override, None);
    }

    /// List 350 offers exactly the pet nodes, to the Goa'uld only, and every
    /// one is a Goa'uld tree node (so the trainer can ever mark it
    /// trainable). Summon Straegis is also the ability `pet_summons` maps.
    #[tokio::test]
    async fn pet_trainer_list_offers_the_goauld_pet_nodes() {
        let pool = require_db_or_skip!();
        let goauld = goauld_archetype_index(&pool).await;
        let offered = load_trainer_abilities(&pool)
            .await
            .expect("load_trainer_abilities must succeed");

        let keys: Vec<(i32, i32)> = offered
            .keys()
            .filter(|(list, _)| *list == PET_TRAINER_LIST)
            .copied()
            .collect();
        assert_eq!(
            keys,
            vec![(PET_TRAINER_LIST, goauld)],
            "list 350 must be keyed for the Goa'uld archetype only"
        );
        let got: BTreeSet<i32> = offered[&(PET_TRAINER_LIST, goauld)]
            .iter()
            .copied()
            .collect();
        assert_eq!(got, OFFERED.into_iter().collect::<BTreeSet<i32>>());

        let tree: Vec<i32> = sqlx::query_scalar(
            "SELECT ability_id FROM resources.archetype_ability_tree \
              WHERE archetype = 'ARCHETYPE_Goauld' AND ability_id = ANY($1)",
        )
        .bind(OFFERED.as_slice())
        .fetch_all(&pool)
        .await
        .expect("archetype_ability_tree query");
        let tree: BTreeSet<i32> = tree.into_iter().collect();
        assert_eq!(
            tree,
            OFFERED.into_iter().collect::<BTreeSet<i32>>(),
            "every offered ability must be a Goa'uld tree node"
        );

        let summons = load_pet_summons(&pool)
            .await
            .expect("load_pet_summons must succeed");
        assert!(
            summons.pet_summon_for(OFFERED[0]).is_some(),
            "the trainer's first offer must be a summon the pet seed maps"
        );
    }
}
