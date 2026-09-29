//! Ability invocation handler for the CellService.
//!
//! Processes `useAbility` calls from the client: validates the ability exists
//! in the entity's known list, checks cooldowns, starts warmup/cooldown timers,
//! resolves damage against the target, and sends results to the client.
//!
//! Submodule layout:
//! - `dispatch` — ground-targeted auto-aim entry point.
//! - `use_ability` — main `handle_use_ability` flow (validate → consume → fire → resolve),
//!   including the warmup between launch and fire (AT-10).
//! - `damage_apply` — per-target damage application, shared between the
//!   targeted path and ground-target AoE so cooldown/ammo consume happens
//!   once per invocation but damage applies to each target in radius.
//! - `death` — ordered wire protocol burst when a target dies.
//! - `deployable` — deployable abilities (Phase 0): the ground-point
//!   launch, the fire that places the object, and its pulse tick.
//! - `messaging` — entity-method routing (player vs witness) + dirty-stat flush.
//! - `timer_update` — `onTimerUpdate` goes to the owning player's client only.
//! - `loot_drop` — on-death loot generation + interaction-flag updates.
//! - `resolve` — per-weapon ability resolution (items_event_sets lookup).
//! - `rng` — deterministic pseudo-random for combat rolls.
//!
//! Reference: `python/cell/AbilityManager.py:1004-1056`

mod cone_aoe;
mod damage_apply;
mod death;
mod deployable;
mod dispatch;
mod loot_drop;
mod messaging;
#[cfg(test)]
mod movement_type_log_tests;
mod resolve;
mod rng;
mod timer_update;
mod use_ability;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::Foo` paths stable for callers.
pub use cone_aoe::{collect_cone_targets, fan_out_cone_effects, log_effect_flag_categories};
pub use death::kill_npc_out_of_band;
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use death::resolve_death_for_test;
pub use deployable::{deployable_tick, deployable_tick_at};
pub use dispatch::handle_use_ability_on_ground;
pub use loot_drop::{roll_loot_entries, INT_NORMAL_LOOT};
pub use messaging::{
    broadcast_movement_type, request_appearance_refresh, send_entity_method,
    send_entity_method_to_self_and_witnesses, send_entity_method_to_witnesses,
};
// `send_entity_method_to_witnesses` and `send_entity_method_to_self_and_witnesses`
// land here for #278 child PRs to adopt. They stay private to the `messaging`
// module until the first child callsite migrates — at which point the
// migrating PR adds the re-exports it needs.
pub use resolve::{
    ability_for_active_weapon, ability_for_item, is_ability_granted_by_active_weapon,
};
pub use timer_update::{send_timer_update, TimerRoute};
pub use use_ability::{
    credit_ground_deaths, fire_line_of_sight, interrupt_unlearned_cast, warmup_tick, FireLos,
};
pub(crate) use use_ability::{
    credited_player, interrupt_pending_cast, is_casting, InterruptReason,
};
pub use use_ability::{handle_use_ability, handle_use_ability_with_kill_credit};
pub use use_ability::{is_owner_pet_ability, owner_pet_tick, owner_pet_tick_at};

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use use_ability::maybe_trigger_auto_reload_for_test;
#[cfg(test)]
pub(crate) use use_ability::resolve_warmups;
