//! A pet's owner-only `createOnClient` replay (A-23, A-24).
//!
//! `SGWPet.createOnClient` (`deprecated/python/cell/SGWPet.py`) sends
//! `onPetAbilityList` then `onPetStanceList`. Both describe the owner's pet
//! bar, so only the owner may receive them: they go out as
//! `WitnessEntityMethod { witness_id: owner }`, the single-recipient form.
//! `send_entity_method` / `send_entity_method_to_witnesses` would fan the
//! lists to every observer.
//!
//! Client contract (`docs/reverse-engineering/findings/pet-client-contract.md`):
//! the owner's client binds the pet into `Unit.Pet1..4` once the entity has
//! `ENTITYFLAG_Pet` and has received both `onEntityProperty(PetOwnerId)`
//! (in the create cascade, every witness) and `onPetStanceList` (here, owner
//! only), in either order. `onPetAbilityList` fills the bar but does not gate
//! the bind. `onPetStanceUpdate` does not gate it either, and the client
//! starts at the `.def` default (Defensive), so it is sent only for a
//! non-default stance: a pet re-met after a stance change must not show
//! Defensive.
//!
//! Called from **both** `EnteredAoI` sites: the AoI tick
//! (`space_manager/aoi.rs`) and the client's `requestEntityUpdate` re-emit
//! (`cimmeria-cell`'s `request_entity_update.rs`). The events must follow the
//! pet's `EnteredAoI` in the same batch, which is what keeps them behind the
//! CREATE_ENTITY on the base's `deferred_aoi` path.

use cimmeria_entity::cell_entity::{CellEntity, PetStance};
use cimmeria_wire::cell::client_methods::pet::{
    build_pet_ability_list, build_pet_stance_list, build_pet_stance_update, ON_PET_ABILITY_LIST,
    ON_PET_STANCE_LIST, ON_PET_STANCE_UPDATE,
};

use super::super::messages::CellToBaseMsg;

/// The stance a client assumes for a freshly created pet (`SGWPet.def`
/// `petStance` default 1).
pub const CLIENT_DEFAULT_STANCE: PetStance = PetStance::Defensive;

/// The owner-only pet messages for `witness` meeting `entity`, in send
/// order. Empty unless `entity` is a pet and `witness` is its owner.
pub fn pet_create_on_client_events(witness: u32, entity: &CellEntity) -> Vec<CellToBaseMsg> {
    let Some(pet) = entity.pet.as_deref() else {
        return Vec::new();
    };
    if pet.owner_id != witness {
        return Vec::new();
    }
    let pet_id = entity.entity_id.0 as u32;
    let stances: Vec<i8> = pet
        .allowed_stances()
        .into_iter()
        .map(|s| s.wire())
        .collect();
    let call = |method_index: u16, args: Vec<u8>| CellToBaseMsg::WitnessEntityMethod {
        witness_id: witness,
        entity_id: pet_id,
        method_index,
        args,
        entity_is_player: false,
    };
    let mut events = vec![
        call(
            ON_PET_ABILITY_LIST,
            build_pet_ability_list(&pet.ability_list),
        ),
        call(ON_PET_STANCE_LIST, build_pet_stance_list(&stances)),
    ];
    if pet.stance != CLIENT_DEFAULT_STANCE {
        events.push(call(
            ON_PET_STANCE_UPDATE,
            build_pet_stance_update(pet.stance.wire()),
        ));
    }
    events
}
