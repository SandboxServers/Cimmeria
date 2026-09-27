//! The warmup half of an owner's attack order (`petInvokeAbility`, pets
//! PT-04): the checks a pet's ordered cast gets again at fire time, and the
//! engagement it earns once it has fired.
//!
//! The command engages at once for an instant cast. For a cast with a
//! warmup it records `PetState::deferred_order`, and the tick calls
//! [`engage_fired_order`] after the cast fires, so an interrupted warmup
//! (the target turned friendly, went behind a wall, left the space) engages
//! nothing. The engagement is the pet AI's own
//! (`npc_ai::pet::engage_pet_target` with `PetEngagement::OwnerOrder`),
//! which seeds both sides of the fight.

use cimmeria_cell_world::cell::pets::{
    engage_refusal_code, order_feedback_text, take_deferred_order,
};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::service::npc_ai::pet::{engage_pet_target, fight_refusal, PetEngagement};
use crate::cell::space_manager::SpaceManager;

/// Why the pet `caster` may not fire its warmed-up cast at `target`, or
/// `None`: the pet rule (`fight_refusal`: a combatant SGWMob its owner could
/// attack). Content can change a target during the warmup, so the launch's
/// check is not enough.
///
/// The rule is judged against whoever holds the owner's entity id, not
/// only the verified summoner: an owner whose id was reused before the
/// sweep still lets the cast land, and kill credit (`credit_recipient`,
/// PT-06) is what refuses the new holder. With no entity at that id the
/// cast is left alone; `pet_owner_sweep` despawns the pet within a tick.
pub(super) fn pet_fire_refusal(
    space_mgr: &SpaceManager,
    caster: &CellEntity,
    target: &CellEntity,
) -> Option<&'static str> {
    let owner_id = caster.pet.as_deref()?.owner_id;
    let owner = space_mgr.get_entity(owner_id)?;
    fight_refusal(owner, target)
}

/// After `pet_id`'s warmed-up cast at `fired_target` fired: pop its deferred
/// order and, when the cast was the ordered one, engage the order's target.
///
/// Logs `event = "order_engaged"` (DEBUG, `pets.command`) whenever an order
/// was pending: `engaged`, and on a refusal the engagement's `reason`
/// (`target_changed` for a stale order). A refusal other than `target_dead`
/// (the cast killed the target) or `target_changed` answers the owner with
/// `onErrorCode` plus a `CHAN_FEEDBACK` line.
pub(super) async fn engage_fired_order(
    pet_id: u32,
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
        pet_id,
        owner_id = order.owner_id,
        account_id = summoner.account_id,
        player_id = summoner.player_id,
        template_id,
        target_id = order.target_id,
        fired_target_id = fired_target,
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
    let mut err = Vec::with_capacity(7);
    err.push(0u8); // SystemID: ERRORCODE_SYSTEM_Ability
    err.extend_from_slice(&fired_target.to_le_bytes());
    err.extend_from_slice(&engage_refusal_code(reason).to_le_bytes());
    let chat =
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, order_feedback_text(reason));
    for (method_index, args) in [
        (crate::mercury::method_idx::ON_ERROR_CODE, err),
        (crate::mercury::method_idx::ON_PLAYER_COMMUNICATION, chat),
    ] {
        let msg = CellToBaseMsg::EntityMethodCall {
            entity_id: order.owner_id,
            method_index,
            args,
        };
        if tx.send(msg).await.is_err() {
            tracing::warn!(
                target: "pets.command",
                event = "order_feedback_send_failed",
                entity_id = order.owner_id,
                pet_id,
                owner_id = order.owner_id,
                account_id = summoner.account_id,
                player_id = summoner.player_id,
                method_index,
                reason = "feedback_send_failed",
                "pet order refusal feedback could not be queued (base channel closed)"
            );
            return;
        }
    }
}
