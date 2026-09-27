//! No pet to act on: every owner-pet ability is refused at the press with
//! `onErrorCode` plus a `CHAN_FEEDBACK` line, before the cooldown is charged.

use cimmeria_cell_world::test_fixtures::{add_pet_owner, make_pet_world, PET_FIXTURE_ABILITIES};
use cimmeria_entity::cell_entity::{PetState, PlayerIdentity};
use cimmeria_entity::stats::ACCURACY;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::*;
use crate::cell::abilities::use_ability::handle_use_ability;

/// An owner with no pet, knowing every ability under test.
fn world_without_pet() -> SpaceManager {
    let mut mgr = make_pet_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    seed_rows(&mut mgr);
    let owner = mgr.get_entity_mut(OWNER).unwrap();
    for id in [
        HOLY_WARRIOR,
        TO_THE_DEATH,
        LORDS_CONCENTRATION,
        REPAIR_PERCENT,
    ] {
        owner.abilities.add_ability(id);
    }
    mgr
}

async fn press(mgr: &mut SpaceManager, ability: i32) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let ok = handle_use_ability(OWNER, ability, 0, &tx, mgr).await;
    (ok, drain(&mut rx))
}

/// **Regression guard.** No pet: each ability is refused with
/// `EntityDoesNotHavePet` (190) and the line, and no cooldown is charged.
/// Fails when the launch refusal is removed (the press commits silently).
#[tokio::test]
async fn no_pet_is_refused_with_feedback_and_no_cooldown() {
    let mut mgr = world_without_pet();
    for ability in [
        HOLY_WARRIOR,
        TO_THE_DEATH,
        LORDS_CONCENTRATION,
        REPAIR_PERCENT,
    ] {
        let (ok, sent) = press(&mut mgr, ability).await;
        assert!(!ok, "{ability}");
        assert_eq!(error_codes(&sent, OWNER), vec![(ability, 190)], "{ability}");
        assert!(
            got_line(&sent, OWNER, "You have no pet to use that on."),
            "{ability}"
        );
        assert!(!on_cooldown(&mgr, ability), "{ability}: no cooldown");
    }
}

/// A dead pet: `NotLiving` (14) and "Your pet is dead.".
#[tokio::test]
async fn a_dead_pet_is_refused() {
    let (mut mgr, pet) = world();
    mgr.get_entity_mut(pet).unwrap().state_field |= BSF_DEAD;
    let (ok, sent) = press(&mut mgr, HOLY_WARRIOR).await;
    assert!(!ok);
    assert_eq!(error_codes(&sent, OWNER), vec![(HOLY_WARRIOR, 14)]);
    assert!(got_line(&sent, OWNER, "Your pet is dead."));
    assert_eq!(stat(&mgr, pet, ACCURACY), 0);
}

/// The only pet is in another space (the sweep has not run yet): refused
/// with "Your pet is not here.", and the stray pet is not buffed.
#[tokio::test]
async fn a_pet_in_another_space_is_refused() {
    let mut mgr = world_without_pet();
    const STRAY: u32 = 0x7000_0801;
    mgr.spawn_npc(STRAY, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(STRAY).unwrap().pet = Some(Box::new(PetState::new(
        OWNER,
        PET_FIXTURE_ABILITIES.to_vec(),
        0b111,
        0,
    )));
    let summoner = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, STRAY, summoner);

    let (ok, sent) = press(&mut mgr, HOLY_WARRIOR).await;
    assert!(!ok);
    assert_eq!(error_codes(&sent, OWNER), vec![(HOLY_WARRIOR, 190)]);
    assert!(got_line(&sent, OWNER, "Your pet is not here."));
    assert_eq!(stat(&mgr, STRAY, ACCURACY), 0);
}

/// **Regression guard (never trust a bare owner id).** The registry lists a
/// pet under the caster's entity id, but the caster is not the player who
/// summoned it (the id was reused): refused, and the pet is not buffed.
/// Fails when the summoner check is dropped from `owner_pet_targets`.
#[tokio::test]
async fn a_pet_summoned_by_an_earlier_holder_of_the_id_is_not_touched() {
    let (mut mgr, pet) = world();
    mgr.pets
        .register(OWNER, pet, PlayerIdentity::new(Some(9_999), Some(9_999)));

    let (ok, sent) = press(&mut mgr, HOLY_WARRIOR).await;
    assert!(!ok);
    assert_eq!(error_codes(&sent, OWNER), vec![(HOLY_WARRIOR, 190)]);
    assert_eq!(stat(&mgr, pet, ACCURACY), 0);
}
