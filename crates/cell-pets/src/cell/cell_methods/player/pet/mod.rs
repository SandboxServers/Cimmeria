//! The owner's pet commands (pets campaign PT-04, issue #570):
//! `petInvokeAbility` (cell method 88), `petAbilityToggle` (89) and
//! `petChangeStance` (90).
//!
//! # The ownership guard comes first (CAT-C-11 / #462)
//!
//! All three carry a client-supplied pet entity id. Before anything else a
//! handler resolves it through [`owned_pet_or_refuse`], which asks
//! `SpaceManager::owned_pet(caller, claimed)`. Another player's pet, an NPC,
//! a player, an id that names nothing, or a pet summoned by an earlier
//! holder of the caller's entity id is refused with `onErrorCode` to the
//! caller so the press gets visible feedback. The command never reaches the
//! pet. `owned_pet` logs the refusal itself (DEBUG, `pets.command`,
//! `event = "ownership_rejected"`, `reason`); this module adds no second row.
//!
//! # Feedback
//!
//! Every refusal after the guard also answers the owner: `onErrorCode`
//! (`ERRORCODE_SYSTEM_Ability`, an `EConditionHandlerFeedback` code) or, where
//! no code fits, a visible refresh (the owner's pet bar or stance row is sent
//! again). `onErrorCode` goes to the **owner** (`EntityMethodCall` on the
//! owner's id), never to the pet id: an `EntityMethodCall` addressed to an
//! NPC reaches no client.
//!
//! # Owner-only sends
//!
//! `onPetAbilityList` and `onPetStanceUpdate` describe the owner's pet bar,
//! so they go out as `WitnessEntityMethod { witness_id: owner }` (A-24), like
//! PT-01's `pet_create_on_client_events`.
//!
//! # Client facts this code relies on
//!
//! - Every pet-bar click, ability or "command", arrives as CM 88; CM 89 has
//!   no call site in the shipped Lua (A-06). It is still guarded.
//! - The small pet bar sends a 1-based **slot index** as the stance id (A-07,
//!   an original-client bug); see [`stance::resolve_requested_stance`].

mod invoke;
mod stance;
mod toggle;

#[cfg(test)]
mod tests;

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::{PET_ABILITY_TOGGLE, PET_CHANGE_STANCE, PET_INVOKE_ABILITY};
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::combat::is_dead_state;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::pets::PetReject;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_PLAYER_COMMUNICATION;
use cimmeria_cell_world::cell::pets::order_feedback_text;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

pub use stance::{resolve_requested_stance, StanceSource};

// ── onErrorCode ErrorCodeID values (`EConditionHandlerFeedback`,
// `entities/defs/enumerations.xml`). The client has no Lua consumer for
// onErrorCode (AT-E1), so these are the closest honest names, not a
// verified on-screen text.

/// `CONDITION_FEEDBACK_InvalidEntity`: the target names nothing usable.
pub(crate) const FEEDBACK_INVALID_ENTITY: u16 = 0;
/// `CONDITION_FEEDBACK_NotLiving`: the owner, the pet or the target is dead.
pub(crate) const FEEDBACK_NOT_LIVING: u16 = 14;
/// `CONDITION_FEEDBACK_RelationshipFriend`: the target is not hostile.
pub(crate) const FEEDBACK_RELATIONSHIP_FRIEND: u16 = 37;
/// `CONDITION_FEEDBACK_LOS`: a wall between the pet and the target. The code
/// a player's own shot gets (`fire_los`), and the one LoS entry in
/// `ErrorStrings.pak` with authored text.
pub(crate) const FEEDBACK_NO_LINE_OF_SIGHT: u16 = 39;
/// `CONDITION_FEEDBACK_OutsideWeaponRange`: the pet is too far from the
/// target.
pub(crate) const FEEDBACK_OUTSIDE_WEAPON_RANGE: u16 = 42;
/// `CONDITION_FEEDBACK_WeaponCooldownNotReady`: the pet's ability is cooling
/// down or the pet is still warming up another one. The enum has no
/// ability-cooldown code; this is the nearest.
pub(crate) const FEEDBACK_NOT_READY: u16 = 99;
/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility`: the ability is not on the
/// pet's bar, the owner toggled it off, or it does nothing on this server
/// yet (`ability_not_implemented`).
pub(crate) const FEEDBACK_NO_SUCH_PET_ABILITY: u16 = 167;
/// `CONDITION_FEEDBACK_EntityDoesNotHavePet`: the registry still lists the
/// pet but its entity is gone (the teardown sweep has not run yet).
pub(crate) const FEEDBACK_DOES_NOT_HAVE_PET: u16 = 190;
/// `CONDITION_FEEDBACK_IsNotPetOwner`: the claimed id is not the caller's
/// pet.
pub(crate) const FEEDBACK_IS_NOT_PET_OWNER: u16 = 236;

/// `ERRORCODE_SYSTEM_Ability`, the only `EErrorCodeSystem` value.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// Which pet command a log row is about. The label is the `command` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PetCommand {
    /// CM 88 `petInvokeAbility`.
    InvokeAbility,
    /// CM 89 `petAbilityToggle`.
    AbilityToggle,
    /// CM 90 `petChangeStance`.
    ChangeStance,
}

impl PetCommand {
    /// Stable snake_case label. Treat as API.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::InvokeAbility => "invoke_ability",
            Self::AbilityToggle => "ability_toggle",
            Self::ChangeStance => "change_stance",
        }
    }
}

/// Who sent a pet command, as every `pets.command` row names them: the
/// owner's entity id plus the owner's `account_id` / `player_id`
/// (instrumentation Rule 5), each with its name (Rule 6), resolved once per
/// command through `SpaceManager::player_identity`. The identity fields are
/// `Option`s and are logged as such; an unresolved one is omitted, never `0`
/// or `""`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Caller {
    pub(crate) command: PetCommand,
    /// The caller's entity id (the owner, when the guard passes).
    pub(crate) owner_id: u32,
    pub(crate) account_id: Option<u32>,
    pub(crate) account_name: Option<&'static str>,
    pub(crate) player_id: Option<i32>,
    /// The character name: `owner_name` and `player_name` on every row.
    pub(crate) player_name: Option<&'static str>,
}

impl Caller {
    pub(crate) fn resolve(command: PetCommand, owner_id: u32, space_mgr: &SpaceManager) -> Self {
        let id = space_mgr.player_identity(owner_id);
        Self {
            command,
            owner_id,
            account_id: id.account_id,
            account_name: id.account_name,
            player_id: id.player_id,
            player_name: id.player_name,
        }
    }
}

/// Route cell methods 88-90. Returns `true` for all three, whatever the
/// outcome, so the outer dispatcher never reports them unhandled.
pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    let command = match method_index {
        PET_INVOKE_ABILITY => PetCommand::InvokeAbility,
        PET_ABILITY_TOGGLE => PetCommand::AbilityToggle,
        PET_CHANGE_STANCE => PetCommand::ChangeStance,
        _ => return false,
    };
    let caller = Caller::resolve(command, entity_id, space_mgr);
    match command {
        PetCommand::InvokeAbility => match invoke::InvokeArgs::parse(args) {
            Some(parsed) => invoke::handle(caller, parsed, tx, space_mgr, engine).await,
            None => malformed(caller, args, 12),
        },
        PetCommand::AbilityToggle => match toggle::ToggleArgs::parse(args) {
            Some(parsed) => toggle::handle(caller, parsed, tx, space_mgr).await,
            None => malformed(caller, args, 9),
        },
        PetCommand::ChangeStance => match stance::StanceArgs::parse(args) {
            Some(parsed) => stance::handle(caller, parsed, tx, space_mgr).await,
            None => malformed(caller, args, 5),
        },
    }
    true
}

/// A pet command too short to name a pet. No feedback: without a pet id
/// there is nothing to answer about, and a real client never sends one.
fn malformed(caller: Caller, args: &[u8], expected_len: usize) {
    tracing::warn!(
        target: "pets.command",
        decision_outcome = "rejected",
        command = caller.command.label(),
        owner_id = caller.owner_id,
        account_id = caller.account_id,
        player_id = caller.player_id,
        owner_name = caller.player_name,
        account_name = caller.account_name,
        player_name = caller.player_name,
        reason = "malformed_args",
        args_len = args.len(),
        expected_len,
        "pet command rejected: args shorter than the method's fixed layout -- \
         command ignored, the owner's pet does nothing"
    );
}

/// How loud a refusal is. WARN for what a real client does not send: a
/// malformed packet, a known ability not on the bar, or a cast the launch
/// refused after every pre-check passed. DEBUG for ordinary play (cooldown,
/// out of range, a friendly or dead target, a wall in the way) and for
/// forgeable values (an ability id with no definition, a target in another
/// space), which a client could otherwise use to flood the log. A forged
/// pet id is forgeable too; `SpaceManager::owned_pet` logs it at DEBUG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalLevel {
    Warn,
    Debug,
}

/// One refused command: the `reason` for the log and the `onErrorCode` the
/// owner gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Refusal {
    pub(crate) reason: &'static str,
    pub(crate) error_code: u16,
    /// `InstanceID` of the `onErrorCode`: the ability id for 88 and 89, the
    /// claimed pet id for 90.
    pub(crate) instance_id: i32,
    pub(crate) level: RefusalLevel,
}

impl Refusal {
    pub(crate) fn warn(reason: &'static str, error_code: u16, instance_id: i32) -> Self {
        Self {
            reason,
            error_code,
            instance_id,
            level: RefusalLevel::Warn,
        }
    }

    pub(crate) fn debug(reason: &'static str, error_code: u16, instance_id: i32) -> Self {
        Self {
            reason,
            error_code,
            instance_id,
            level: RefusalLevel::Debug,
        }
    }
}

/// Log `refusal` on `pets.command` and send its `onErrorCode` to the owner.
/// `pet_name` is the resolved pet's label (`SpaceManager::entity_names`),
/// taken by the caller, which still holds the space manager.
pub(crate) async fn refuse(
    caller: Caller,
    pet_id: i32,
    pet_name: Option<&'static str>,
    refusal: Refusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    match refusal.level {
        RefusalLevel::Warn => tracing::warn!(
            target: "pets.command",
            decision_outcome = "rejected",
            command = caller.command.label(),
            owner_id = caller.owner_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            owner_name = caller.player_name,
            account_name = caller.account_name,
            player_name = caller.player_name,
            pet_id,
            pet_name,
            reason = refusal.reason,
            error_code = refusal.error_code,
            error_name = cimmeria_names::book().error_code(refusal.error_code),
            instance_id = refusal.instance_id, // nt:id-only onErrorCode InstanceID: an ability id or the claimed pet id
            "pet command rejected: {} -- the pet does nothing, onErrorCode sent to the owner",
            refusal.reason
        ),
        RefusalLevel::Debug => tracing::debug!(
            target: "pets.command",
            decision_outcome = "rejected",
            command = caller.command.label(),
            owner_id = caller.owner_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            owner_name = caller.player_name,
            account_name = caller.account_name,
            player_name = caller.player_name,
            pet_id,
            pet_name,
            reason = refusal.reason,
            error_code = refusal.error_code,
            error_name = cimmeria_names::book().error_code(refusal.error_code),
            instance_id = refusal.instance_id, // nt:id-only onErrorCode InstanceID: an ability id or the claimed pet id
            "pet command rejected: {} -- the pet does nothing, onErrorCode sent to the owner",
            refusal.reason
        ),
    }
    send_error_code(
        caller,
        refusal.instance_id,
        refusal.error_code,
        refusal.reason,
        tx,
    )
    .await;
}

/// `onErrorCode(SystemID u8, InstanceID i32, ErrorCodeID u16)` to the owner,
/// then a `CHAN_FEEDBACK` chat line saying why (`order_feedback_text` by
/// `reason`). The shipped client has no Lua consumer for `onErrorCode`
/// (AT-E1), so the line is what the owner actually sees.
async fn send_error_code(
    caller: Caller,
    instance_id: i32,
    error_code: u16,
    reason: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&instance_id.to_le_bytes());
    args.extend_from_slice(&error_code.to_le_bytes());
    let chat =
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, order_feedback_text(reason));
    let mut failed = false;
    for (method_index, args) in [(ON_ERROR_CODE, args), (ON_PLAYER_COMMUNICATION, chat)] {
        let msg = CellToBaseMsg::EntityMethodCall {
            entity_id: caller.owner_id,
            method_index,
            args,
        };
        failed |= tx.send(msg).await.is_err();
    }
    if failed {
        tracing::warn!(
            target: "pets.command",
            command = caller.command.label(),
            owner_id = caller.owner_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            owner_name = caller.player_name,
            account_name = caller.account_name,
            player_name = caller.player_name,
            error_code,
            error_name = cimmeria_names::book().error_code(error_code),
            reason = "feedback_send_failed",
            "pet command feedback: onErrorCode could not be queued (base channel closed) -- \
             the owner sees no reaction to the press"
        );
    }
}

/// An owner-only method on the pet (`onPetAbilityList`, `onPetStanceUpdate`):
/// the single-recipient `WitnessEntityMethod`, never a witness fan-out.
pub(crate) async fn send_to_owner(
    caller: Caller,
    pet_id: u32,
    pet_name: Option<&'static str>,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let msg = CellToBaseMsg::WitnessEntityMethod {
        witness_id: caller.owner_id,
        entity_id: pet_id,
        method_index,
        args,
        entity_is_player: false,
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "pets.command",
            command = caller.command.label(),
            owner_id = caller.owner_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            owner_name = caller.player_name,
            account_name = caller.account_name,
            player_name = caller.player_name,
            pet_id,
            pet_name,
            method_index,
            method_name = cimmeria_wire::names::client_method(cimmeria_wire::names::SGWPET_CLASS_ID, method_index),
            reason = "owner_send_failed",
            "pet command: owner-only pet method could not be queued (base channel closed) -- \
             the owner's pet bar keeps its old state"
        );
    }
}

/// The ownership guard every pet command runs first (CAT-C-11 / #462).
///
/// Returns the pet's entity id when the caller owns the live pet `claimed`,
/// is alive (the legacy handlers were `@mustBeAlive`) and shares its space.
/// Otherwise sends `onErrorCode` to the caller and returns `None`: the
/// command must go no further. An ownership refusal is logged by
/// `SpaceManager::owned_pet`; the liveness and space refusals here.
pub(crate) async fn owned_pet_or_refuse(
    caller: Caller,
    claimed: i32,
    instance_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<u32> {
    // A negative id wraps to one above every NPC id, which no registry
    // entry holds, so it is refused as `not_a_pet`.
    let pet = match space_mgr.owned_pet(caller.owner_id, claimed as u32) {
        Ok(pet) => pet,
        Err(reject) => {
            // `owned_pet` already logged this refusal with its `reason` and
            // the caller's identity; only the feedback is left to do.
            let error_code = match reject {
                PetReject::PetGone => FEEDBACK_DOES_NOT_HAVE_PET,
                PetReject::NotAPet
                | PetReject::NotOwner { .. }
                | PetReject::OwnerIdentityMismatch => FEEDBACK_IS_NOT_PET_OWNER,
            };
            send_error_code(caller, instance_id, error_code, reject.reason(), tx).await;
            return None;
        }
    };
    let owner_dead = space_mgr
        .get_entity(caller.owner_id)
        .is_none_or(|e| is_dead_state(e.state_field));
    if owner_dead {
        refuse(
            caller,
            claimed,
            space_mgr.entity_names(pet).entity_name,
            Refusal::debug("owner_dead", FEEDBACK_NOT_LIVING, instance_id),
            tx,
        )
        .await;
        return None;
    }
    // The teardown sweep despawns a pet whose owner left its space within
    // one AoI tick; a command in that gap goes nowhere.
    if space_mgr.get_entity_space_id(pet) != space_mgr.get_entity_space_id(caller.owner_id) {
        refuse(
            caller,
            claimed,
            space_mgr.entity_names(pet).entity_name,
            Refusal::debug("pet_other_space", FEEDBACK_DOES_NOT_HAVE_PET, instance_id),
            tx,
        )
        .await;
        return None;
    }
    Some(pet)
}

/// The pet's `template_id`, the correlator every accept row carries.
pub(crate) fn pet_template_id(space_mgr: &SpaceManager, pet: u32) -> Option<i32> {
    space_mgr.get_entity(pet).and_then(|e| e.template_id)
}
