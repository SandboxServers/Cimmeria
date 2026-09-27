//! Pets PT-11: each roster summon, cast through the real `useAbility` path
//! with the seeded data.
//!
//! The PT-03 unit tests model 2826 with a hand-built fixture. These load the
//! ability defs, `pet_summons`, the template cache and the sequence map from
//! the seed, exactly as cell startup does, then cast 1643 Summon Jaffa, 1645
//! Summon Prime and 1644 Summon Lo'taur with `handle_use_ability` and run
//! the warmup tick. Each must spawn its own template, owned by the caster,
//! at the caster's level, with its kit on the pet bar and every stance
//! offered, after playing the Goa'uld summon cast effect.

use std::time::{Duration, Instant};

use cimmeria_cell_world::test_fixtures::add_pet_owner;
use cimmeria_entity::cell_entity::ALL_STANCES_MASK;

use super::warmup::sequences;
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::cell::spawner::{
    load_ability_defs, load_event_set_sequences, load_pet_summons, load_spawn_templates,
};
use crate::mercury::SGWPET_CLASS_ID;
use crate::test_support::{require_db_or_skip, NoContentEvents};

const OWNER: u32 = 1;
/// The owner's level: every roster pet must take it (D-PT02).
const OWNER_LEVEL: u32 = 23;
/// 2826 Summon Straegis, the Servant Lord L50 capstone, and its template.
const SUMMON_STRAEGIS: i32 = 2826;
const STRAEGIS_PET: i32 = 350;
/// Ability_End of event set 1121 "Goauld summon source".
const SEQ_SUMMON_CAST: i32 = 2292;

/// (summon, template, name_id, kit) for each roster pet.
const ROSTER: [(i32, i32, i32, &[i32]); 3] = [
    (1643, 351, 8087, &[584, 710, 1652]),
    (1645, 352, 28892, &[584, 710, 1654]),
    (1644, 353, 28891, &[1653, 3326, 3327, 3328, 3329]),
];

/// A Castle space with the owner in it at `owner_level`, knowing every
/// roster summon and 2826, and the four caches the summon path reads loaded
/// from the seed.
async fn seeded_mgr(pool: &sqlx::PgPool, owner_level: u32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    add_pet_owner(&mut mgr, OWNER, "Castle", [0.0, 0.0, 0.0], owner_level);
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .add_ability(SUMMON_STRAEGIS);
    for (summon, ..) in ROSTER {
        mgr.get_entity_mut(OWNER)
            .unwrap()
            .abilities
            .add_ability(summon);
    }
    mgr.ability_defs = load_ability_defs(pool).await.expect("ability defs load");
    mgr.pet_summons = load_pet_summons(pool).await.expect("pet summons load");
    mgr.spawn_templates = load_spawn_templates(pool).await.expect("templates load");
    mgr.sequence_map = load_event_set_sequences(pool)
        .await
        .expect("sequence map loads");
    mgr
}

/// Cast `summon` and run the warmup tick past its seeded warmup.
async fn summon_through_the_cast_path(
    mgr: &mut SpaceManager,
    summon: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> Vec<CellToBaseMsg> {
    let warmup = mgr.ability_defs[&summon].warmup;
    assert!(
        handle_use_ability(OWNER, summon, OWNER as i32, tx, mgr).await,
        "{summon}: the summon must commit"
    );
    let at = Instant::now() + Duration::from_secs_f32(warmup) + Duration::from_millis(50);
    assert_eq!(
        resolve_warmups(at, tx, mgr, &NoContentEvents).await,
        1,
        "{summon}: the warmup must resolve"
    );
    drain(rx)
}

/// **Acceptance: each roster summon spawns its pet** through
/// `handle_use_ability` with the seeded rows. Cast in turn, each replaces
/// the one before (D-PT04), so the test also sees one pet at a time.
#[tokio::test]
async fn each_roster_summon_spawns_its_pet_through_the_cast_path() {
    let pool = require_db_or_skip!();
    let mut mgr = seeded_mgr(&pool, OWNER_LEVEL).await;
    let (tx, mut rx) = mpsc::channel(512);

    let mut previous: Option<u32> = None;
    for (summon, template, name_id, kit) in ROSTER {
        let sent = summon_through_the_cast_path(&mut mgr, summon, &tx, &mut rx).await;

        let pets = mgr.pets.pets_of(OWNER);
        assert_eq!(pets.len(), 1, "{summon}: one active pet");
        let pet_id = pets[0];
        if let Some(old) = previous {
            assert_ne!(pet_id, old, "{summon}: replaced the previous pet");
            assert!(mgr.get_entity(old).is_none(), "{summon}: old pet despawned");
        }
        previous = Some(pet_id);

        let pet = mgr.get_entity(pet_id).expect("pet entity exists");
        assert_eq!(pet.template_id, Some(template), "{summon}: template");
        assert_eq!(pet.class_id, SGWPET_CLASS_ID, "{summon}: SGWPet class");
        assert_eq!(pet.name_id, Some(name_id), "{summon}: nameplate");
        assert_eq!(
            pet.level, OWNER_LEVEL,
            "{summon}: the owner's level (D-PT02)"
        );
        let state = pet.pet.as_deref().expect("PetState");
        assert_eq!(state.owner_id, OWNER);
        assert_eq!(state.summon_ability_id, summon);
        assert_eq!(state.ability_list, kit, "{summon}: the pet bar is the kit");
        assert_eq!(
            state.stance_mask, ALL_STANCES_MASK,
            "{summon}: every stance is offered"
        );
        assert!(
            sequences(&sent, OWNER).contains(&SEQ_SUMMON_CAST),
            "{summon}: the cast plays 2292 on the caster"
        );
    }
}

/// **A Straegis summoned by a level-50 owner is level 50** (D-PT02). Template
/// 350 used to carry `ENTITYFLAG_NoPetLeveling`, which makes the spawn keep
/// the template's level: the L50 capstone pet came out at level 1 with
/// 250 HP. Fails with bit 8 back on 350's flags.
#[tokio::test]
async fn a_straegis_summoned_by_a_level_50_owner_is_level_50() {
    let pool = require_db_or_skip!();
    let mut mgr = seeded_mgr(&pool, 50).await;
    let (tx, mut rx) = mpsc::channel(512);

    summon_through_the_cast_path(&mut mgr, SUMMON_STRAEGIS, &tx, &mut rx).await;

    let pets = mgr.pets.pets_of(OWNER);
    assert_eq!(pets.len(), 1, "one pet");
    let pet = mgr.get_entity(pets[0]).expect("pet entity exists");
    assert_eq!(pet.template_id, Some(STRAEGIS_PET));
    assert_eq!(pet.level, 50, "the Straegis takes its owner's level");
}
