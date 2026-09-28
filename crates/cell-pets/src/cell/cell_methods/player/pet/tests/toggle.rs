//! CM 89 `petAbilityToggle`: off and back on, the owner-only bar re-send,
//! and the refusals.

use super::*;
use crate::cell::cell_methods::player::pet::FEEDBACK_NO_SUCH_PET_ABILITY;
use crate::cell::client_methods::pet::{build_pet_ability_list, ON_PET_ABILITY_LIST};
use cimmeria_entity::cell_entity::PetState;

fn toggled_off(mgr: &SpaceManager, pet: u32) -> Vec<i32> {
    mgr.get_entity(pet)
        .unwrap()
        .extensions
        .get::<PetState>()
        .unwrap()
        .toggled_off
        .clone()
}

/// The bar re-send: exactly one `onPetAbilityList` on the pet, to the owner
/// only, carrying the whole bar.
fn assert_bar_resent_to_owner_only(sent: &Sent, pet: u32) {
    assert_eq!(
        sent.witness_calls(),
        vec![(
            OWNER,
            pet,
            ON_PET_ABILITY_LIST,
            build_pet_ability_list(&PET_FIXTURE_ABILITIES),
        )],
        "one onPetAbilityList, owner only (a second witness gets none)"
    );
}

/// Off, then on again: the OFF list follows, CM 88 refuses the ability
/// while it is off and casts it once it is back on, and every toggle
/// re-sends the bar to the owner.
#[tokio::test]
async fn toggle_off_and_on_round_trips() {
    let World { mut mgr, pet, .. } = world();

    let sent = toggle(&mut mgr, OWNER, pet, PET_ABILITY, 0).await;
    assert_eq!(toggled_off(&mgr, pet), vec![PET_ABILITY]);
    assert_eq!(sent.all_error_codes(), 0);
    assert_bar_resent_to_owner_only(&sent, pet);

    // Off twice stays one entry.
    let _ = toggle(&mut mgr, OWNER, pet, PET_ABILITY, 0).await;
    assert_eq!(toggled_off(&mgr, pet), vec![PET_ABILITY]);

    let refused = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(
        refused.error_codes_to(OWNER),
        vec![(PET_ABILITY, FEEDBACK_NO_SUCH_PET_ABILITY)],
        "an OFF ability is refused by CM 88"
    );

    let sent = toggle(&mut mgr, OWNER, pet, PET_ABILITY, 1).await;
    assert!(toggled_off(&mgr, pet).is_empty());
    assert_bar_resent_to_owner_only(&sent, pet);

    let cast = invoke(&mut mgr, OWNER, pet, PET_ABILITY, MOB).await;
    assert_eq!(cast.all_error_codes(), 0, "back ON, the pet casts it");
    assert!(mgr
        .get_entity(pet)
        .unwrap()
        .abilities
        .is_on_cooldown(PET_ABILITY));
}

/// An ability that is not on the bar cannot be toggled.
#[tokio::test]
async fn toggling_an_ability_off_the_bar_is_refused() {
    let World { mut mgr, pet, .. } = world();
    let sent = toggle(&mut mgr, OWNER, pet, NOT_A_PET_ABILITY, 0).await;
    assert_eq!(
        sent.error_codes_to(OWNER),
        vec![(NOT_A_PET_ABILITY, FEEDBACK_NO_SUCH_PET_ABILITY)]
    );
    assert!(toggled_off(&mgr, pet).is_empty());
    assert!(sent.witness_calls().is_empty());
}

/// A toggle value other than 0 or 1 changes nothing; the bar is re-sent as
/// the visible reaction.
#[tokio::test]
async fn an_unknown_toggle_value_is_refused_with_a_bar_refresh() {
    let World { mut mgr, pet, .. } = world();
    for bad in [2i8, -1, i8::MAX] {
        let sent = toggle(&mut mgr, OWNER, pet, PET_ABILITY, bad).await;
        assert!(toggled_off(&mgr, pet).is_empty(), "toggle {bad}");
        assert_bar_resent_to_owner_only(&sent, pet);
    }
}
