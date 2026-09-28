//! The ownership guard (CAT-C-11 / #462): a command naming another player's
//! pet, an NPC, nothing, or a pet an earlier holder of the caller's entity
//! id summoned is refused with `onErrorCode` to the caller.
//! `SpaceManager::owned_pet` logs the refusal once (DEBUG `pets.command`,
//! `event = "ownership_rejected"`, `reason`); the handler adds no second row.
//! Each spoof test also checks the named entity was left alone. Removing the
//! `owned_pet` call from `owned_pet_or_refuse` fails every test here
//! (worknote PT-04, M1).

use cimmeria_entity::cell_entity::PetState;
use tracing::Level;

use super::*;
use crate::cell::cell_methods::player::pet::{
    FEEDBACK_DOES_NOT_HAVE_PET, FEEDBACK_IS_NOT_PET_OWNER, FEEDBACK_NOT_LIVING,
};
use crate::test_support::LogCapture;
use cimmeria_entity::cell_entity::PetStance;

/// The one `ownership_rejected` row with `reason`, or a panic listing what
/// was captured. Also pins that it is the only `pets.command` row about the
/// refusal: DEBUG, and no WARN (a client can name any id at will).
fn guard_row(
    capture: &crate::test_support::LogCaptureGuard,
    reason: &str,
) -> crate::test_support::Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.command" && c.has_field("reason", reason))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "expected one pets.command row with reason={reason}; captured: {:#?}",
        capture.all()
    );
    let row = rows.into_iter().next().unwrap();
    assert_eq!(row.level, Level::DEBUG, "{row:?}");
    assert!(row.has_field("event", "ownership_rejected"), "{row:?}");
    assert!(row.has_field("caller_id", &OWNER.to_string()), "{row:?}");
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.target == "pets.command" && c.level == Level::WARN),
        "an ownership refusal is not a WARN: {:#?}",
        capture.all()
    );
    row
}

/// All three commands against the pet of `OTHER`: refused as `not_owner`,
/// naming the real owner, and the other pet is untouched.
#[tokio::test]
async fn another_players_pet_is_refused_for_every_command() {
    let World {
        mut mgr, other_pet, ..
    } = world();

    for (label, method, args) in [
        (
            "invoke",
            PET_INVOKE_ABILITY,
            invoke_args(other_pet, PET_ABILITY, MOB),
        ),
        (
            "toggle",
            PET_ABILITY_TOGGLE,
            toggle_args(other_pet, PET_ABILITY, 0),
        ),
        ("stance", PET_CHANGE_STANCE, stance_args(other_pet, 2)),
    ] {
        let capture = LogCapture::install();
        let sent = call(&mut mgr, OWNER, method, &args).await;

        let warn = guard_row(&capture, "not_owner");
        assert!(
            warn.has_field("account_id", &OWNER_ACCOUNT_ID.to_string()),
            "{label}"
        );
        assert!(
            warn.has_field("player_id", &OWNER_PLAYER_ID.to_string()),
            "{label}"
        );
        assert!(warn.has_field("pet_id", &other_pet.to_string()), "{label}");
        assert!(
            warn.has_field("owner_id", &OTHER.to_string()),
            "{label}: owner_id names the real owner"
        );
        let instance = if method == PET_CHANGE_STANCE {
            other_pet as i32
        } else {
            PET_ABILITY
        };
        assert_eq!(
            sent.error_codes_to(OWNER),
            vec![(instance, FEEDBACK_IS_NOT_PET_OWNER)],
            "{label}: the caller gets IsNotPetOwner"
        );
        assert_eq!(
            sent.feedback_lines_to(OWNER),
            1,
            "{label}: and a visible CHAN_FEEDBACK line"
        );
        assert!(
            sent.witness_calls().is_empty(),
            "{label}: nothing about the pet is sent to anyone"
        );
        assert_eq!(sent.error_codes_to(OTHER), vec![], "{label}");
    }

    let other = mgr.get_entity(other_pet).unwrap();
    let state = other.extensions.get::<PetState>().unwrap();
    assert_eq!(state.stance, PetStance::Defensive, "stance unchanged");
    assert!(state.toggled_off.is_empty(), "nothing toggled off");
    assert!(
        !other.abilities.is_on_cooldown(PET_ABILITY),
        "the other pet cast nothing"
    );
    assert!(
        other.threat_list.is_empty(),
        "the other pet engaged nothing"
    );
}

/// A hostile NPC's id and an id nothing holds: `not_a_pet`, for every
/// command, and the NPC is not turned into anything.
#[tokio::test]
async fn an_npc_id_and_a_nonexistent_id_are_refused_as_not_a_pet() {
    let World { mut mgr, .. } = world();

    for claimed in [MOB, NOBODY, OTHER] {
        for (method, args) in [
            (PET_INVOKE_ABILITY, invoke_args(claimed, PET_ABILITY, MOB)),
            (PET_ABILITY_TOGGLE, toggle_args(claimed, PET_ABILITY, 0)),
            (PET_CHANGE_STANCE, stance_args(claimed, 2)),
        ] {
            let capture = LogCapture::install();
            let sent = call(&mut mgr, OWNER, method, &args).await;
            let warn = guard_row(&capture, "not_a_pet");
            assert!(
                warn.has_field("pet_id", &claimed.to_string()),
                "claimed {claimed}, method {method}"
            );
            assert_eq!(
                sent.error_codes_to(OWNER).len(),
                1,
                "claimed {claimed}, method {method}: one onErrorCode"
            );
            assert_eq!(
                sent.error_codes_to(OWNER)[0].1,
                FEEDBACK_IS_NOT_PET_OWNER,
                "claimed {claimed}, method {method}"
            );
            assert!(sent.witness_calls().is_empty());
        }
    }
    let mob = mgr.get_entity(MOB).unwrap();
    assert!(!mob.extensions.contains::<PetState>());
    assert!(mob.threat_list.is_empty());
}

/// A negative wire id wraps to an id no pet holds.
#[tokio::test]
async fn a_negative_pet_id_is_refused() {
    let World { mut mgr, .. } = world();
    let capture = LogCapture::install();
    let mut args = (-1i32).to_le_bytes().to_vec();
    args.push(2);
    let sent = call(&mut mgr, OWNER, PET_CHANGE_STANCE, &args).await;
    let warn = guard_row(&capture, "not_a_pet");
    // `owned_pet` takes the id as the u32 the registry keys on.
    assert!(warn.has_field("pet_id", &u32::MAX.to_string()));
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(-1, FEEDBACK_IS_NOT_PET_OWNER)]
    );
}

/// The registry still lists a pet whose entity is gone (the sweep has not
/// run): `pet_gone`, answered with DoesNotHavePet.
#[tokio::test]
async fn a_registered_pet_without_an_entity_is_refused_as_pet_gone() {
    let World { mut mgr, .. } = world();
    let summoner = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, NOBODY, summoner);
    let capture = LogCapture::install();
    let sent = stance(&mut mgr, OWNER, NOBODY, 2).await;
    guard_row(&capture, "pet_gone");
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(NOBODY as i32, FEEDBACK_DOES_NOT_HAVE_PET)]
    );
}

/// The caller holds the owner's entity id but is not the player who summoned
/// the pet: the owner left and its id was reused before the sweep removed
/// the pet (#870). A bare owner-id comparison would hand the new holder the
/// pet; the per-pet summoner check refuses it, for every command.
#[tokio::test]
async fn a_reused_owner_id_does_not_inherit_the_pet() {
    let World { mut mgr, pet, .. } = world();
    // Same entity id, another character behind it.
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID + 1);

    for (method, args) in [
        (PET_INVOKE_ABILITY, invoke_args(pet, PET_ABILITY, MOB)),
        (PET_ABILITY_TOGGLE, toggle_args(pet, PET_ABILITY, 0)),
        (PET_CHANGE_STANCE, stance_args(pet, 2)),
    ] {
        let capture = LogCapture::install();
        let sent = call(&mut mgr, OWNER, method, &args).await;
        let row = guard_row(&capture, "owner_identity_mismatch");
        assert!(row.has_field("pet_id", &pet.to_string()), "method {method}");
        assert_eq!(
            sent.error_codes_to(OWNER).len(),
            1,
            "method {method}: one onErrorCode"
        );
        assert_eq!(
            sent.error_codes_to(OWNER)[0].1,
            FEEDBACK_IS_NOT_PET_OWNER,
            "method {method}"
        );
        assert!(sent.witness_calls().is_empty(), "method {method}");
    }
    let target = mgr.get_entity(pet).unwrap();
    let state = target.extensions.get::<PetState>().unwrap();
    assert_eq!(state.stance, PetStance::Defensive, "stance unchanged");
    assert!(state.toggled_off.is_empty(), "nothing toggled off");
    assert!(!target.abilities.is_on_cooldown(PET_ABILITY), "no cast");
    assert!(target.threat_list.is_empty(), "no engage");
}

/// A dead owner commands nothing (the legacy handlers were `@mustBeAlive`).
#[tokio::test]
async fn a_dead_owner_is_refused() {
    let World { mut mgr, pet, .. } = world();
    mgr.get_entity_mut(OWNER).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
    let sent = stance(&mut mgr, OWNER, pet, 2).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(pet as i32, FEEDBACK_NOT_LIVING)]
    );
    let state = mgr
        .get_entity(pet)
        .unwrap()
        .extensions
        .get::<PetState>()
        .unwrap()
        .stance;
    assert_eq!(state, PetStance::Defensive, "stance unchanged");
}

/// The owner's own pet passes the guard: no refusal is logged.
#[tokio::test]
async fn the_owners_own_pet_passes_the_guard() {
    let World { mut mgr, pet, .. } = world();
    let capture = LogCapture::install();
    let sent = stance(&mut mgr, OWNER, pet, 2).await;
    assert_eq!(sent.all_error_codes(), 0);
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.target == "pets.command" && c.level == Level::WARN),
        "no pets.command WARN for the owner's own pet: {:#?}",
        capture.all()
    );
}

/// A pet in another space than its owner (the sweep has not run yet) is
/// refused: the command would otherwise act across spaces.
#[tokio::test]
async fn a_pet_in_another_space_is_refused() {
    let World { mut mgr, .. } = world();
    const STRAY: u32 = 200_020;
    mgr.spawn_npc(STRAY, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(STRAY).unwrap().extensions.insert(
        cimmeria_entity::cell_entity::PetState::new(
            OWNER,
            PET_FIXTURE_ABILITIES.to_vec(),
            0b111,
            0,
        ),
    );
    let summoner = mgr.player_identity(OWNER);
    mgr.pets.register(OWNER, STRAY, summoner);
    let capture = LogCapture::install();
    let sent = stance(&mut mgr, OWNER, STRAY, 2).await;
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "pets.command" && c.has_field("reason", "pet_other_space")));
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(STRAY as i32, FEEDBACK_DOES_NOT_HAVE_PET)]
    );
    let stance_now = mgr
        .get_entity(STRAY)
        .unwrap()
        .extensions
        .get::<PetState>()
        .unwrap()
        .stance;
    assert_eq!(stance_now, PetStance::Defensive);
}
