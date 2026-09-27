//! CM 89 `petAbilityToggle(INT32 petId, INT32 abilityId, INT8 toggle)`.
//!
//! Turns one of the pet's abilities on or off. `SGWPet.toggledAbilities` is
//! "the list of pet toggled abilities that are toggled OFF"
//! (`entities/defs/SGWPet.def`), so `toggle` is read as the ability's new
//! state: `1` on (removed from `toggled_off`), `0` off (added). The `.def`
//! only says "toggle on/off", and the shipped Lua never calls it (A-06), so
//! the polarity is an assumption; any other value is refused.
//!
//! An off ability is refused by CM 88 and skipped by the AI's selector
//! (PT-05). The owner's pet bar is re-sent after every toggle, and after a
//! refused one, so the press always gets a visible reaction.

use tokio::sync::mpsc;

use super::{owned_pet_or_refuse, pet_template_id, refuse, send_to_owner, Caller};
use crate::cell::client_methods::pet::{build_pet_ability_list, ON_PET_ABILITY_LIST};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Parsed CM 89 args (9 bytes: INT32, INT32, INT8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ToggleArgs {
    pub(super) pet_id: i32,
    pub(super) ability_id: i32,
    pub(super) toggle: i8,
}

impl ToggleArgs {
    pub(super) fn parse(args: &[u8]) -> Option<Self> {
        let word = |i: usize| -> Option<i32> {
            Some(i32::from_le_bytes(args.get(i..i + 4)?.try_into().ok()?))
        };
        Some(Self {
            pet_id: word(0)?,
            ability_id: word(4)?,
            toggle: *args.get(8)? as i8,
        })
    }
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
        ability_id = args.ability_id,
        toggle = args.toggle,
    )
)]
pub(super) async fn handle(
    caller: Caller,
    args: ToggleArgs,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let ToggleArgs {
        pet_id,
        ability_id,
        toggle,
    } = args;
    let Some(pet) = owned_pet_or_refuse(caller, pet_id, ability_id, tx, space_mgr).await else {
        return;
    };

    let on = match toggle {
        0 => false,
        1 => true,
        other => {
            // DEBUG: no shipped UI sends CM 89 (A-06), so only a forged or
            // hand-typed packet lands here; a WARN would let it flood the log.
            tracing::debug!(
                target: "pets.command",
                decision_outcome = "rejected",
                command = caller.command.label(),
                owner_id = caller.owner_id,
                account_id = caller.account_id,
                player_id = caller.player_id,
                pet_id,
                ability_id,
                toggle = other,
                reason = "bad_toggle_value",
                "pet command rejected: toggle is neither 0 (off) nor 1 (on) -- \
                 nothing changed, the pet bar is re-sent"
            );
            resend_ability_list(caller, pet, tx, space_mgr).await;
            return;
        }
    };

    let in_list = space_mgr
        .get_entity(pet)
        .and_then(|e| e.pet.as_deref())
        .is_some_and(|s| s.ability_list.contains(&ability_id));
    if !in_list {
        refuse(
            caller,
            pet_id,
            super::invoke::not_in_list(space_mgr, ability_id),
            tx,
        )
        .await;
        return;
    }

    if let Some(state) = space_mgr
        .get_entity_mut(pet)
        .and_then(|e| e.pet.as_deref_mut())
    {
        set_toggled(&mut state.toggled_off, ability_id, on);
    }
    resend_ability_list(caller, pet, tx, space_mgr).await;
    tracing::debug!(
        target: "pets.command",
        event = "toggled",
        decision_outcome = "toggled",
        command = caller.command.label(),
        owner_id = caller.owner_id,
        account_id = caller.account_id,
        player_id = caller.player_id,
        pet_id = pet,
        template_id = pet_template_id(space_mgr, pet),
        ability_id,
        on,
        "pet command: the owner toggled a pet ability"
    );
}

/// Add `ability_id` to the OFF list (`on == false`) or remove it. Never
/// duplicates an entry.
pub(crate) fn set_toggled(toggled_off: &mut Vec<i32>, ability_id: i32, on: bool) {
    if on {
        toggled_off.retain(|&id| id != ability_id);
    } else if !toggled_off.contains(&ability_id) {
        toggled_off.push(ability_id);
    }
}

/// `onPetAbilityList` to the owner only (A-24).
async fn resend_ability_list(
    caller: Caller,
    pet: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(list) = space_mgr
        .get_entity(pet)
        .and_then(|e| e.pet.as_deref())
        .map(|s| build_pet_ability_list(&s.ability_list))
    else {
        return;
    };
    send_to_owner(caller, pet, ON_PET_ABILITY_LIST, list, tx).await;
}
