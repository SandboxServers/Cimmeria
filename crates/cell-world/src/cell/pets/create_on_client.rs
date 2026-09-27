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
//!
//! "The owner" is the player who summoned the pet, not whoever holds the
//! owner's entity id now. Entity ids are reused, and between the owner's
//! `destroy_entity` and the next-tick sweep another player can be given the
//! same id; `pet.owner_id == witness` alone would hand that player the old
//! pet's bar and bind it into their `Unit.Pet` slots. The summon-time
//! identity in [`PetRegistry`] decides (Copilot, #870).

use cimmeria_entity::cell_entity::{CellEntity, PetStance};
use cimmeria_wire::cell::client_methods::pet::{
    build_pet_ability_list, build_pet_stance_list, build_pet_stance_update, ON_PET_ABILITY_LIST,
    ON_PET_STANCE_LIST, ON_PET_STANCE_UPDATE,
};

use super::super::messages::CellToBaseMsg;
use super::PetRegistry;

/// The stance a client assumes for a freshly created pet (`SGWPet.def`
/// `petStance` default 1).
pub const CLIENT_DEFAULT_STANCE: PetStance = PetStance::Defensive;

/// The owner-only pet messages for `witness` meeting `entity`, in send
/// order. Empty unless `entity` is a pet and `witness` is the player who
/// summoned it: its entity id is the pet's `owner_id` **and** its live
/// identity matches the one `pets` captured at summon.
///
/// An id match with an identity mismatch is the id-reuse window before the
/// sweep: nothing owner-only is sent, and a WARN on `pets.lifecycle`
/// records it (`event = pet_list_replay_refused`,
/// `reason = owner_identity_mismatch`). The ordinary non-owner witness is
/// not a refusal and logs nothing.
pub fn pet_create_on_client_events(
    witness: &CellEntity,
    entity: &CellEntity,
    pets: &PetRegistry,
) -> Vec<CellToBaseMsg> {
    let Some(pet) = entity.pet.as_deref() else {
        return Vec::new();
    };
    let witness_id = witness.entity_id.0 as u32;
    if pet.owner_id != witness_id {
        return Vec::new();
    }
    let pet_id = entity.entity_id.0 as u32;
    let live = witness.identity();
    if !witness.is_player || !pets.owner_identity_matches(witness_id, live) {
        // Server-side id reuse, not something a client can trigger at will:
        // WARN (negative-logging convention). `account_id` / `player_id` are
        // the summoner's (Rule 5), the `witness_*` pair the id's new holder.
        let owner = pets.owner_identity(witness_id);
        tracing::warn!(
            target: "pets.lifecycle",
            event = "pet_list_replay_refused",
            reason = "owner_identity_mismatch",
            entity_id = pet_id,
            pet_id,
            owner_id = pet.owner_id,
            witness_id,
            account_id = owner.account_id,
            player_id = owner.player_id,
            witness_account_id = live.account_id,
            witness_player_id = live.player_id,
            template_id = entity.template_id,
            "pet owner-only lists withheld: the owner's entity id now belongs to another entity"
        );
        return Vec::new();
    }
    let stances: Vec<i8> = pet
        .allowed_stances()
        .into_iter()
        .map(|s| s.wire())
        .collect();
    let call = |method_index: u16, args: Vec<u8>| CellToBaseMsg::WitnessEntityMethod {
        witness_id,
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
