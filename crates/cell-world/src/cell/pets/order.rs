//! An owner's attack order (`petInvokeAbility`, cell method 88, pets PT-04):
//! what makes the pet keep fighting the target the owner named.
//!
//! The command handler lives in `cimmeria-cell-methods`; the engagement is
//! here so the warmup tick (`cimmeria-cell-combat`) can apply a deferred
//! order when the ordered cast actually fires. An instant cast engages at
//! once; a cast with a warmup records [`PetState::deferred_order`] and
//! engages from [`engage_deferred_order`]. An interrupted warmup never
//! reaches it, so a target that turned friendly or went out of sight during
//! the warmup is not engaged.
//!
//! [`PetState::deferred_order`]: cimmeria_entity::cell_entity::PetState::deferred_order

use cimmeria_entity::cell_entity::{AiState, CellEntity};
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_DEAD;

use super::super::service::npc_ai::{set_ai_state_on, world_label, AiTransitionReason};
use super::super::space_manager::SpaceManager;

/// Threat a commanded target gets on top of the pet's current highest, so
/// the fight tick's top-threat pick is the target the owner named.
pub const PET_COMMAND_THREAT: f32 = 10.0;

/// Ceiling on a commanded target's threat.
pub const PET_COMMAND_THREAT_CAP: f32 = 1_000_000.0;

/// Whether an owner may order a pet at `target`: a hostile-faction SGWMob
/// (class 0x04), the #444 rule a player attacker gets plus the class check
/// the threat path applies. Never a player, never a pet (whatever its
/// faction), never an SGWBeing (class 0x01): beings are props and story
/// actors that never enter combat (`generate_threat` refuses them, NA42),
/// even when seeded on faction 10. The command pre-check and the warmup
/// fire re-check both ask this, so a warmed-up order obeys the same rule
/// as an instant one.
pub fn is_order_target(target: &CellEntity) -> bool {
    !target.is_player
        && target.pet.is_none()
        && target.class_id == crate::mercury::SGWMOB_CLASS_ID
        && target.faction == super::super::combat::HOSTILE_FACTION
}

/// `BSF_DEAD` or no health left: the same two tests the fight tick uses.
fn is_dead(entity: &CellEntity) -> bool {
    entity.state_field & BSF_DEAD != 0 || entity.stats.get(HEALTH).is_none_or(|s| s.cur <= 0)
}

/// Put the commanded target on top of the pet's threat list and the pet into
/// Fighting, so the AI keeps fighting it after this one cast. Returns
/// whether it did (not when the cast killed the target).
///
/// Stance rules (a Passive pet never engages on its own, D-PT09) are PT-05's
/// and govern autonomous engagement; an explicit order is obeyed here.
pub fn engage_commanded_target(space_mgr: &mut SpaceManager, pet: u32, target: u32) -> bool {
    if space_mgr.get_entity(target).is_none_or(is_dead) {
        return false;
    }
    let world = world_label(space_mgr, pet);
    let Some(entity) = space_mgr.get_entity_mut(pet) else {
        return false;
    };
    let top = entity.threat_list.values().copied().fold(0.0_f32, f32::max);
    // Capped so repeated orders cannot grow the value without bound.
    entity.threat_list.insert(
        target,
        (top + PET_COMMAND_THREAT).min(PET_COMMAND_THREAT_CAP),
    );
    set_ai_state_on(
        entity,
        &world,
        AiState::Fighting,
        AiTransitionReason::PetCommand,
    );
    true
}

/// Called by the warmup tick after `caster`'s warmed-up cast at
/// `fired_target` has fired. Takes the caster's deferred order, if it is a
/// pet with one, and engages the order's target when the fired cast was
/// aimed at it. Returns whether the pet engaged.
///
/// Logs `event = "order_engaged"` (DEBUG, `pets.command`) whenever an order
/// was pending, with the owner's identity and `engaged`; `reason =
/// "target_changed"` when the fired cast was aimed elsewhere, or
/// `"target_dead"` when the cast killed the target.
pub fn engage_deferred_order(space_mgr: &mut SpaceManager, caster: u32, fired_target: i32) -> bool {
    let Some((owner_id, order)) = space_mgr.get_entity_mut(caster).and_then(|e| {
        let pet = e.pet.as_deref_mut()?;
        Some((pet.owner_id, pet.deferred_order.take()?))
    }) else {
        return false;
    };
    let (engaged, reason) = if i64::from(order) != i64::from(fired_target) {
        (false, Some("target_changed"))
    } else if engage_commanded_target(space_mgr, caster, order) {
        (true, None)
    } else {
        (false, Some("target_dead"))
    };
    let summoner = space_mgr.pets.summoner_identity(caster);
    let template_id = space_mgr.get_entity(caster).and_then(|e| e.template_id);
    tracing::debug!(
        target: "pets.command",
        event = "order_engaged",
        pet_id = caster,
        owner_id,
        account_id = summoner.account_id,
        player_id = summoner.player_id,
        template_id,
        target_id = order,
        fired_target_id = fired_target,
        engaged,
        reason,
        "pet order: the warmed-up cast fired; engaged = {engaged}"
    );
    engaged
}
