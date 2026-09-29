//! PT-08: "the owner's pet" resolution and the pet buff ledger. The pet
//! effect scripts' and the passive pass's tests moved with the scripts to
//! `cimmeria-cell-effect-scripts` (`cell::effects::pet_scripts::tests`).

use std::time::{Duration, Instant};

use cimmeria_entity::stats::{ACCURACY, DEFENSE, INTERRUPT_RES};
use cimmeria_wire::state_field::BSF_DEAD;

use super::*;
use crate::cell::pets::{BuffRemoval, OwnerPetRefusal};

fn stat(mgr: &SpaceManager, e: u32, id: i32) -> i32 {
    mgr.get_entity(e).unwrap().stats.get(id).unwrap().cur
}

// ── owner_pet_targets ─────────────────────────────────────────────────────

#[test]
fn the_owners_live_pet_resolves() {
    let (mgr, pet) = world_with_pet();
    assert_eq!(mgr.owner_pet_targets(OWNER), Ok(vec![pet]));
}

#[test]
fn no_pet_dead_pet_and_elsewhere_are_refused_with_their_reason() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    assert_eq!(mgr.owner_pet_targets(OWNER), Err(OwnerPetRefusal::NoPet));

    let (mut mgr, pet) = world_with_pet();
    mgr.get_entity_mut(pet).unwrap().state_field |= BSF_DEAD;
    assert_eq!(mgr.owner_pet_targets(OWNER), Err(OwnerPetRefusal::PetDead));
    assert_eq!(OwnerPetRefusal::PetDead.error_code(), 14);

    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    let stray = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .unwrap();
    // Move the owner out; the pet stays in Agnos.
    mgr.destroy_entity(OWNER);
    add_pet_owner(&mut mgr, OWNER, "Castle", [10.0, 0.0, 10.0], 12);
    assert!(mgr.get_entity(stray).is_some());
    assert_eq!(
        mgr.owner_pet_targets(OWNER),
        Err(OwnerPetRefusal::PetOtherSpace)
    );
}

/// **Regression guard.** The owner's entity id now belongs to another
/// player: the pet the earlier holder summoned is not theirs to act on.
/// Fails when the summoner check is removed.
#[test]
fn a_reused_owner_id_does_not_resolve_the_earlier_pet() {
    let (mut mgr, _pet) = world_with_pet();
    reuse_owner_id_by_another_player(&mut mgr);
    assert_eq!(
        mgr.owner_pet_targets(OWNER),
        Err(OwnerPetRefusal::OwnerIdentityMismatch)
    );
}

/// The id's new holder summons its own pet: that one resolves, the earlier
/// holder's does not.
#[test]
fn after_reuse_only_the_new_holders_pet_resolves() {
    let (mut mgr, old_pet) = world_with_pet();
    let new_pet = reuse_owner_id_then_resummon(&mut mgr);
    let resolved = mgr.owner_pet_targets(OWNER).unwrap();
    assert_eq!(resolved, vec![new_pet]);
    assert!(!resolved.contains(&old_pet));
}

// ── The buff ledger ───────────────────────────────────────────────────────

#[test]
fn a_buff_widens_a_zero_bound_and_its_removal_restores_the_value() {
    let (mut mgr, pet) = world_with_pet();
    let applied = mgr
        .apply_pet_buff(pet, 1, 2, &[(DEFENSE, -100), (INTERRUPT_RES, 50)], None)
        .unwrap();
    assert_eq!(applied, vec![(DEFENSE, -100), (INTERRUPT_RES, 50)]);
    assert_eq!(stat(&mgr, pet, DEFENSE), -100);
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 50);
    assert!(mgr.has_pet_buff(pet, 1));

    let removed = mgr.remove_pet_buff(pet, 1, BuffRemoval::Expired).unwrap();
    assert_eq!(removed.stat_deltas, applied);
    assert_eq!(stat(&mgr, pet, DEFENSE), 0);
    assert_eq!(stat(&mgr, pet, INTERRUPT_RES), 0);
    assert!(!mgr.has_pet_buff(pet, 1));
    assert!(mgr.remove_pet_buff(pet, 1, BuffRemoval::Expired).is_none());
}

#[test]
fn re_applying_the_same_effect_refreshes_instead_of_stacking() {
    let (mut mgr, pet) = world_with_pet();
    let now = Instant::now();
    mgr.apply_pet_buff(pet, 1, 2, &[(ACCURACY, 400)], Some(now));
    mgr.apply_pet_buff(
        pet,
        1,
        2,
        &[(ACCURACY, 400)],
        Some(now + Duration::from_secs(60)),
    );
    assert_eq!(stat(&mgr, pet, ACCURACY), 400);
    assert_eq!(mgr.expired_pet_buffs(now + Duration::from_secs(1)), vec![]);
    assert_eq!(
        mgr.expired_pet_buffs(now + Duration::from_secs(61)),
        vec![(pet, 1)]
    );
}

#[test]
fn a_non_pet_takes_no_buff() {
    let (mut mgr, _pet) = world_with_pet();
    assert!(mgr
        .apply_pet_buff(OWNER, 1, 2, &[(ACCURACY, 400)], None)
        .is_none());
    assert_eq!(stat(&mgr, OWNER, ACCURACY), 0);
}
