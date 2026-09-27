//! An owner's attack order (`petInvokeAbility`, cell method 88, pets PT-04):
//! the pieces both halves of the order share.
//!
//! The command handler (`cimmeria-cell-methods`) engages at once for an
//! instant cast. A cast with a warmup records [`PetState::deferred_order`]
//! instead, and the warmup tick (`cimmeria-cell-combat`) engages when the
//! ordered cast fires, after [`take_deferred_order`] pops the order. An
//! interrupted warmup never fires, so it engages nothing. The engagement
//! itself, and the rule for what a pet may fight, are the pet AI's
//! (`npc_ai::pet::engage_pet_target` / `fight_refusal`, PT-05); they live in
//! `cimmeria-cell-combat`, which this crate cannot call.
//!
//! [`PetState::deferred_order`]: cimmeria_entity::cell_entity::PetState::deferred_order

use super::super::space_manager::SpaceManager;

/// A deferred order popped by the warmup tick when the pet's cast fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakenOrder {
    /// The pet's owner (the player who gave the order).
    pub owner_id: u32,
    /// The target the owner named.
    pub target_id: u32,
    /// Whether the fired cast was aimed at that target. A pet has one
    /// warming cast at a time, so a mismatch means the order is stale.
    pub matched: bool,
}

/// Pop `pet`'s deferred order, if it is a pet with one, and say whether the
/// cast that just fired at `fired_target` was the ordered one. Taken by any
/// fired cast, so an order never outlives the pet's next fire.
pub fn take_deferred_order(
    space_mgr: &mut SpaceManager,
    pet: u32,
    fired_target: i32,
) -> Option<TakenOrder> {
    let state = space_mgr.get_entity_mut(pet)?.pet.as_deref_mut()?;
    let target_id = state.deferred_order.take()?;
    Some(TakenOrder {
        owner_id: state.owner_id,
        target_id,
        matched: i64::from(target_id) == i64::from(fired_target),
    })
}

/// The `onErrorCode` `ErrorCodeID` (`EConditionHandlerFeedback`) for an
/// order that failed after the command accepted it: a refused engagement
/// (the `reason` `npc_ai::pet::engage_pet_target` returns) or an
/// interrupted warmup (an `InterruptReason` label). `InvalidEntity` (0) for
/// a target that is gone, elsewhere or resetting, `NotLiving` (14) for a
/// dead target or pet, `RelationshipFriend` (37) for one the pet may not
/// fight, `LOS` (39), `OutsideWeaponRange` (42), `WeaponCooldownNotReady`
/// (99) for an attack the pet itself broke off, `EntityDoesNotHavePet`
/// (190) when the pet or its owner no longer holds.
pub fn order_refusal_code(reason: &str) -> u16 {
    match reason {
        "target_dead" | "caster_died" => 14,
        "target_not_combatant" | "target_not_hostile" => 37,
        "no_line_of_sight" => 39,
        "target_out_of_range" | "out_of_range" => 42,
        "caster_moved" | "bandolier_slot_change" | "ammo_unavailable" | "ability_unlearned" => 99,
        "not_a_pet" | "owner_gone" | "owner_identity_mismatch" => 190,
        _ => 0,
    }
}

/// The `CHAN_FEEDBACK` line an owner reads when a pet command is refused,
/// by the refusal's `reason`. `onErrorCode` alone has no Lua consumer in
/// the shipped client (AT-E1), so this line is what makes the refusal
/// visible. One table for the command handler and the warmup tick.
pub fn order_feedback_text(reason: &str) -> &'static str {
    match reason {
        "not_owner" | "not_a_pet" | "owner_identity_mismatch" => "That is not your pet.",
        "pet_gone" | "pet_other_space" => "Your pet is not here.",
        "owner_dead" => "You cannot command your pet while you are dead.",
        "pet_dead" => "Your pet is dead.",
        "ability_not_in_list" => "Your pet does not have that ability.",
        "ability_toggled_off" => "That pet ability is turned off.",
        "ability_not_implemented" => "Your pet can't use that ability yet.",
        "pet_casting" | "ability_on_cooldown" => "Your pet is not ready.",
        "cast_refused" => "Your pet could not use that ability.",
        "target_gone" | "target_other_space" => "Your pet cannot reach that target.",
        "target_not_combatant" | "target_not_hostile" | "target_refused_threat" => {
            "Your pet cannot attack that."
        }
        "target_dead" => "That target is already dead.",
        "target_resetting" | "target_not_engageable" => "That target cannot be attacked right now.",
        "out_of_range" | "target_out_of_range" => "Your pet is too far from that target.",
        "target_lost" => "Your pet lost its target.",
        "caster_died" => "Your pet is dead.",
        "caster_moved" | "bandolier_slot_change" | "ammo_unavailable" | "ability_unlearned" => {
            "Your pet's attack was interrupted."
        }
        "no_line_of_sight" => "Your pet cannot see that target.",
        _ => "Your pet cannot do that.",
    }
}
