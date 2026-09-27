//! CM 90 `petChangeStance`: real stance ids, the small pet bar's 1-based
//! slot index (A-07), refusals, and the owner-only `onPetStanceUpdate`.

use cimmeria_entity::cell_entity::{PetStance, PetState, ALL_STANCES_MASK};

use super::*;
use crate::cell::cell_methods::player::pet::{resolve_requested_stance, StanceSource};
use crate::cell::client_methods::pet::ON_PET_STANCE_UPDATE;

fn current(mgr: &SpaceManager, pet: u32) -> PetStance {
    mgr.get_entity(pet).unwrap().pet.as_deref().unwrap().stance
}

/// Exactly one `onPetStanceUpdate(stance)` on the pet, to the owner only.
fn assert_stance_sent_to_owner_only(sent: &Sent, pet: u32, stance: PetStance) {
    assert_eq!(
        sent.witness_calls(),
        vec![(OWNER, pet, ON_PET_STANCE_UPDATE, vec![stance.wire() as u8])],
        "one onPetStanceUpdate, owner only"
    );
}

fn pet_with_mask(mask: u8) -> PetState {
    PetState::new(OWNER, vec![], mask, 0)
}

// ── resolve_requested_stance, the pure half ──

#[test]
fn listed_stance_ids_resolve_to_themselves() {
    let pet = pet_with_mask(ALL_STANCES_MASK);
    for s in PetStance::ALL {
        assert_eq!(
            resolve_requested_stance(&pet, s.wire()),
            Some((s, StanceSource::StanceId))
        );
    }
}

/// The small pet bar sends slots 1-5. With the full list [0, 1, 2], slot 3
/// is Aggressive; 4 and 5 name no slot.
#[test]
fn values_outside_epetstance_are_one_based_slots() {
    let pet = pet_with_mask(ALL_STANCES_MASK);
    assert_eq!(
        resolve_requested_stance(&pet, 3),
        Some((PetStance::Aggressive, StanceSource::SlotIndex))
    );
    for bad in [4i8, 5, -1, i8::MIN, i8::MAX] {
        assert_eq!(resolve_requested_stance(&pet, bad), None, "{bad}");
    }
}

/// A NoDefensive pet's list is [Passive, Aggressive]. `1` is Defensive,
/// which the pet may not take, so it can only be the small bar's slot 1:
/// Passive. `2` is a listed id and stays Aggressive. `3` names no slot.
#[test]
fn an_unlisted_stance_id_is_read_as_a_slot() {
    let pet = pet_with_mask(PetStance::Passive.mask_bit() | PetStance::Aggressive.mask_bit());
    assert_eq!(
        resolve_requested_stance(&pet, 1),
        Some((PetStance::Passive, StanceSource::SlotIndex))
    );
    assert_eq!(
        resolve_requested_stance(&pet, 2),
        Some((PetStance::Aggressive, StanceSource::StanceId))
    );
    assert_eq!(resolve_requested_stance(&pet, 3), None);
    // 0 is neither listed nor a 1-based slot.
    let no_passive =
        pet_with_mask(PetStance::Defensive.mask_bit() | PetStance::Aggressive.mask_bit());
    assert_eq!(resolve_requested_stance(&no_passive, 0), None);
}

// ── the handler ──

#[tokio::test]
async fn a_listed_stance_is_set_and_sent_to_the_owner() {
    let World { mut mgr, pet, .. } = world();
    let sent = stance(&mut mgr, OWNER, pet, PetStance::Aggressive.wire()).await;
    assert_eq!(current(&mgr, pet), PetStance::Aggressive);
    assert_eq!(sent.all_error_codes(), 0);
    assert_stance_sent_to_owner_only(&sent, pet, PetStance::Aggressive);

    let sent = stance(&mut mgr, OWNER, pet, PetStance::Passive.wire()).await;
    assert_eq!(current(&mgr, pet), PetStance::Passive);
    assert_stance_sent_to_owner_only(&sent, pet, PetStance::Passive);
}

/// A-07 end to end: the small bar's third button sends `3`, which is slot 3
/// of the list the owner was sent, Aggressive.
#[tokio::test]
async fn the_small_bar_slot_index_maps_through_the_sent_list() {
    let World { mut mgr, pet, .. } = world();
    let sent = stance(&mut mgr, OWNER, pet, 3).await;
    assert_eq!(current(&mgr, pet), PetStance::Aggressive);
    assert_stance_sent_to_owner_only(&sent, pet, PetStance::Aggressive);
}

/// A value that is neither a listed stance nor a slot changes nothing; the
/// current stance is re-sent so the client's highlight snaps back.
#[tokio::test]
async fn an_unresolvable_stance_is_refused_with_a_refresh() {
    let World { mut mgr, pet, .. } = world();
    for bad in [4i8, 5, -1, 100] {
        let sent = stance(&mut mgr, OWNER, pet, bad).await;
        assert_eq!(current(&mgr, pet), PetStance::Defensive, "{bad}");
        assert_stance_sent_to_owner_only(&sent, pet, PetStance::Defensive);
    }
}
