//! CM 90 `petChangeStance(INT32 petId, INT8 stance)`.
//!
//! # Two clients send two meanings (A-07)
//!
//! The pet info window sends a real `EPetStance` id: its stance icons carry
//! the ids of the `onPetStanceList` the server sent (`PetInfo.lua`
//! `setStance`). The small pet bar never loads that list and sends its
//! button's **1-based slot index** instead (`PetContainer.lua`, left as a
//! `-- TODO:` in the 2009 client). The server resolves both through the list
//! it sent, in [`resolve_requested_stance`]:
//!
//! 1. an `EPetStance` value in the pet's stance list is that stance;
//! 2. anything else is read as a 1-based slot into that list;
//! 3. a value that is neither is refused.
//!
//! Step 2 also covers a real stance id the pet may not take (`1` on a
//! `NoDefensive` pet): the info window only shows listed stances, so that
//! value can only be a slot from the small bar. Either way the result is a
//! stance from the pet's own list; nothing outside it is ever set.
//!
//! On success the stance is set and `onPetStanceUpdate` goes to the owner
//! only. A refused value re-sends the current stance, which puts the
//! client's highlighted stance back.

use cimmeria_entity::cell_entity::{PetStance, PetState};
use tokio::sync::mpsc;

use super::{owned_pet_or_refuse, pet_template_id, send_to_owner, Caller};
use crate::cell::client_methods::pet::{build_pet_stance_update, ON_PET_STANCE_UPDATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Parsed CM 90 args (5 bytes: INT32, INT8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StanceArgs {
    pub(super) pet_id: i32,
    pub(super) stance: i8,
}

impl StanceArgs {
    pub(super) fn parse(args: &[u8]) -> Option<Self> {
        Some(Self {
            pet_id: i32::from_le_bytes(args.get(0..4)?.try_into().ok()?),
            stance: *args.get(4)? as i8,
        })
    }
}

/// How a requested stance value was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StanceSource {
    /// A listed `EPetStance` id (the pet info window).
    StanceId,
    /// A 1-based slot into the stance list (the small pet bar, A-07).
    SlotIndex,
}

impl StanceSource {
    /// Stable snake_case label for the `source` log field.
    pub fn label(self) -> &'static str {
        match self {
            Self::StanceId => "stance_id",
            Self::SlotIndex => "slot_index",
        }
    }
}

/// The stance `raw` asks for on `pet`, or `None` when it names none of the
/// pet's listed stances. See the module doc for the order.
pub fn resolve_requested_stance(pet: &PetState, raw: i8) -> Option<(PetStance, StanceSource)> {
    if let Ok(stance) = PetStance::try_from(raw) {
        if pet.allows(stance) {
            return Some((stance, StanceSource::StanceId));
        }
    }
    let slot = usize::try_from(raw).ok()?.checked_sub(1)?;
    pet.allowed_stances()
        .get(slot)
        .map(|&stance| (stance, StanceSource::SlotIndex))
}

#[tracing::instrument(
    name = "pets.command",
    target = "pets.command",
    level = "info",
    skip_all,
    fields(
        command = caller.command.label(),
        entity_id = caller.owner_id,
        pet_id = args.pet_id,
        requested = args.stance,
    )
)]
pub(super) async fn handle(
    caller: Caller,
    args: StanceArgs,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let StanceArgs { pet_id, stance } = args;
    let Some(pet) = owned_pet_or_refuse(caller, pet_id, pet_id, tx, space_mgr).await else {
        return;
    };
    let template_id = pet_template_id(space_mgr, pet);
    let Some(state) = space_mgr
        .get_entity_mut(pet)
        .and_then(|e| e.extensions.get_mut::<PetState>())
    else {
        return;
    };

    let Some((resolved, source)) = resolve_requested_stance(state, stance) else {
        let current = state.stance;
        // DEBUG: the small pet bar has five buttons, so a pet with fewer
        // stances gets slot 4 or 5 from ordinary clicks.
        tracing::debug!(
            target: "pets.command",
            decision_outcome = "rejected",
            command = caller.command.label(),
            owner_id = caller.owner_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            pet_id,
            requested = stance,
            current = current.label(),
            reason = "stance_not_allowed",
            "pet command rejected: the stance is neither a listed EPetStance id nor a slot \
             in the pet's stance list -- stance unchanged, the current stance is re-sent"
        );
        send_to_owner(
            caller,
            pet,
            ON_PET_STANCE_UPDATE,
            build_pet_stance_update(current.wire()),
            tx,
        )
        .await;
        return;
    };

    let stance_before = state.stance;
    state.stance = resolved;
    // Passive means the pet picks no fight. An order still warming up
    // would engage its target when the cast fires, so it is dropped: the
    // cast lands, the pet does not stay on the target.
    let order_dropped = resolved == PetStance::Passive && state.deferred_order.take().is_some();
    send_to_owner(
        caller,
        pet,
        ON_PET_STANCE_UPDATE,
        build_pet_stance_update(resolved.wire()),
        tx,
    )
    .await;
    tracing::debug!(
        target: "pets.command",
        event = "stance_set",
        decision_outcome = "stance_set",
        command = caller.command.label(),
        owner_id = caller.owner_id,
        account_id = caller.account_id,
        player_id = caller.player_id,
        pet_id = pet,
        template_id,
        requested = stance,
        source = source.label(),
        stance_before = stance_before.label(),
        stance_after = resolved.label(),
        order_dropped,
        "pet command: the owner changed the pet's stance"
    );
}
