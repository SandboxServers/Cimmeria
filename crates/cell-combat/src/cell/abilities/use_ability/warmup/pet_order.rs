//! The warmup half of an owner's attack order (`petInvokeAbility`, pets
//! PT-04): the checks a pet's ordered cast gets again at fire time, the
//! engagement it earns once it has fired, and the owner's feedback when it
//! does not fire.
//!
//! The command engages at once for an instant cast. For a cast with a
//! warmup it records `PetState::deferred_order`. A warmup ends one of two
//! ways, and both pop the order, so a stale one can never engage later:
//!
//! - it fires: [`engage_fired_order`] engages the order's target through
//!   the pet AI's own `engage_pet_target` (`PetEngagement::OwnerOrder`,
//!   both sides of the fight);
//! - it is interrupted (`interrupt_pending_cast`, every trigger: the target
//!   turned friendly or started resetting, a wall, range, the pet moved or
//!   died): [`on_cast_interrupted`] drops the order and tells the owner.

use cimmeria_cell_world::cell::pets::{
    order_feedback_text, order_refusal_code, take_deferred_order,
};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::cell_entity::PetState;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::abilities::wire_ledger::{self, WireCtx};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::service::npc_ai::pet::{
    engage_pet_target, fight_refusal, target_state_refusal, PetEngagement,
};
use crate::cell::space_manager::SpaceManager;

/// Why the pet `caster` may not fire its warmed-up cast at `target`, or
/// `None`: the pet rule (`fight_refusal`: a combatant SGWMob its owner could
/// attack), then the engagement's state rule (`target_state_refusal`: not
/// walking home or leaving the world). Content and the AI can change a
/// target during the warmup, so the launch's checks are not enough, and
/// both run here, before any damage.
///
/// An ordered cast is judged as an owner order (a surrendered NPC is still
/// a target), any other pet cast as the pet's own pick. The rule is judged
/// against whoever holds the owner's entity id, not only the verified
/// summoner: an owner whose id was reused before the sweep still lets the
/// cast land, and kill credit (`credit_recipient`, PT-06) is what refuses
/// the new holder. With no entity at that id the rule is skipped;
/// `pet_owner_sweep` despawns the pet within a tick.
pub(super) fn pet_fire_refusal(
    space_mgr: &SpaceManager,
    caster: &CellEntity,
    target: &CellEntity,
) -> Option<&'static str> {
    let pet = caster.extensions.get::<PetState>()?;
    let kind = if pet.deferred_order.is_some() {
        PetEngagement::OwnerOrder
    } else {
        PetEngagement::Automatic
    };
    if let Some(owner) = space_mgr.get_entity(pet.owner_id) {
        if let Some(why) = fight_refusal(owner, target) {
            return Some(why);
        }
    }
    target_state_refusal(target, kind)
}

/// After `pet_id`'s warmed-up cast of `ability_id` at `fired_target`
/// fired: pop its deferred order and, when the cast was the ordered one,
/// engage the order's target.
///
/// Logs `event = "order_engaged"` (DEBUG, `pets.command`) whenever an order
/// was pending: `engaged`, and on a refusal the engagement's `reason`
/// (`target_changed` for a stale order). A refusal other than `target_dead`
/// (the cast killed the target) or `target_changed` answers the owner with
/// `onErrorCode` plus a `CHAN_FEEDBACK` line.
pub(super) async fn engage_fired_order(
    pet_id: u32,
    ability_id: i32,
    fired_target: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(order) = take_deferred_order(space_mgr, pet_id, fired_target) else {
        return;
    };
    let result = if order.matched {
        engage_pet_target(
            space_mgr,
            pet_id,
            order.target_id,
            PetEngagement::OwnerOrder,
        )
    } else {
        Err("target_changed")
    };
    let summoner = space_mgr.pets.summoner_identity(pet_id);
    let template_id = space_mgr.get_entity(pet_id).and_then(|e| e.template_id);
    let reason = result.err();
    tracing::debug!(
        target: "pets.command",
        event = "order_engaged",
        entity_id = order.owner_id,
        entity_name = space_mgr.entity_label(order.owner_id),
        pet_id,
        pet_name = space_mgr.entity_label(pet_id),
        owner_id = order.owner_id,
        owner_name = summoner.player_name,
        account_id = summoner.account_id,
        account_name = summoner.account_name,
        player_id = summoner.player_id,
        player_name = summoner.player_name,
        template_id,
        template_name = cimmeria_cell_world::cell::effects::content_names::template_name(template_id),
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        target_id = order.target_id,
        target_name = space_mgr.entity_label(order.target_id),
        fired_target_id = fired_target,
        fired_target_name = space_mgr.entity_label(fired_target as u32),
        engaged = reason.is_none(),
        reason,
        "pet order: the ordered cast fired"
    );
    let Some(reason) = reason else {
        return;
    };
    if matches!(reason, "target_dead" | "target_changed") {
        return;
    }
    send_order_feedback(pet_id, order.owner_id, ability_id, reason, tx, space_mgr).await;
}

/// `pet_id`'s warming cast of `ability_id` was interrupted with `reason`
/// (an `InterruptReason` label). Drops the pet's deferred order, if it had
/// one, so it can never engage on a later fire, and tells the owner why the
/// order failed: `onErrorCode` plus a `CHAN_FEEDBACK` line, and a DEBUG
/// `event = "order_interrupted"` row on `pets.command`. A cast with no
/// pending order (a pet's own pick, any other caster) changes nothing.
pub(crate) async fn on_cast_interrupted(
    pet_id: u32,
    ability_id: i32,
    reason: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // An interrupted cast fired at nothing: `take_deferred_order` only pops.
    let Some(order) = take_deferred_order(space_mgr, pet_id, 0) else {
        return;
    };
    let summoner = space_mgr.pets.summoner_identity(pet_id);
    tracing::debug!(
        target: "pets.command",
        event = "order_interrupted",
        entity_id = order.owner_id,
        entity_name = space_mgr.entity_label(order.owner_id),
        pet_id,
        pet_name = space_mgr.entity_label(pet_id),
        owner_id = order.owner_id,
        owner_name = summoner.player_name,
        account_id = summoner.account_id,
        account_name = summoner.account_name,
        player_id = summoner.player_id,
        player_name = summoner.player_name,
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        target_id = order.target_id,
        target_name = space_mgr.entity_label(order.target_id),
        reason,
        "pet order: the ordered cast was interrupted in its warmup; nothing engaged"
    );
    send_order_feedback(pet_id, order.owner_id, ability_id, reason, tx, space_mgr).await;
}

/// `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id, code)` and a
/// `CHAN_FEEDBACK` line to the owner. `InstanceID` is the ability id, as on
/// every other pet-command refusal.
async fn send_order_feedback(
    pet_id: u32,
    owner_id: u32,
    ability_id: i32,
    reason: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let mut err = Vec::with_capacity(7);
    err.push(0u8); // SystemID: ERRORCODE_SYSTEM_Ability
    err.extend_from_slice(&ability_id.to_le_bytes());
    err.extend_from_slice(&order_refusal_code(reason).to_le_bytes());
    let chat =
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, order_feedback_text(reason));
    for (method_index, args) in [
        (crate::mercury::method_idx::ON_ERROR_CODE, err),
        (crate::mercury::method_idx::ON_PLAYER_COMMUNICATION, chat),
    ] {
        let row = wire_ledger::prepare(method_index, &args);
        let msg = CellToBaseMsg::EntityMethodCall {
            entity_id: owner_id,
            method_index,
            args,
        };
        if tx.send(msg).await.is_err() {
            let summoner = space_mgr.pets.summoner_identity(pet_id);
            crate::cell::abilities::metrics::wire_send_failed_in(
                space_mgr,
                owner_id,
                crate::cell::abilities::metrics::WireMessage::from_method(method_index),
            );
            tracing::warn!(
                target: "pets.command",
                event = "order_feedback_send_failed",
                entity_id = owner_id,
                entity_name = space_mgr.entity_label(owner_id),
                pet_id,
                pet_name = space_mgr.entity_label(pet_id),
                owner_id,
                owner_name = summoner.player_name,
                account_id = summoner.account_id,
                account_name = summoner.account_name,
                player_id = summoner.player_id,
                player_name = summoner.player_name,
                method_index,
                method_name = cimmeria_wire::names::player_client_method(method_index),
                reason = "feedback_send_failed",
                "pet order feedback could not be queued (base channel closed)"
            );
            return;
        }
        row.sent_to_owner(
            space_mgr,
            owner_id,
            WireCtx::new("pet_order").ability(ability_id),
        );
    }
}
