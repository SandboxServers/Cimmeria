//! Ability invocation handler for the CellService.
//!
//! Processes `useAbility` calls from the client: validates the ability exists
//! in the entity's known list, checks cooldowns, starts warmup/cooldown timers,
//! resolves damage against the target, and sends results to the client.
//!
//! Submodule layout:
//! - `dispatch` — ground-targeted auto-aim entry point.
//! - `use_ability` — main `handle_use_ability` flow (validate → consume → fire → resolve).
//! - `damage_apply` — per-target damage application, shared between the
//!   targeted path and ground-target AoE so cooldown/ammo consume happens
//!   once per invocation but damage applies to each target in radius.
//! - `death` — ordered wire protocol burst when a target dies.
//! - `messaging` — entity-method routing (player vs witness) + dirty-stat flush.
//! - `loot_drop` — on-death loot generation + interaction-flag updates.
//! - `resolve` — per-weapon ability resolution (items_event_sets lookup).
//! - `rng` — deterministic pseudo-random for combat rolls.
//!
//! Reference: `python/cell/AbilityManager.py:1004-1056`

mod cone_aoe;
mod damage_apply;
mod death;
mod dispatch;
mod loot_drop;
mod messaging;
mod resolve;
mod rng;
mod use_ability;

#[cfg(test)]
mod tests;

// Public re-exports — keep `crate::cell::abilities::Foo` paths stable for callers.
pub use cone_aoe::{collect_cone_targets, fan_out_cone_effects, log_effect_flag_categories};
pub(crate) use death::gm_kill_npc;
pub use dispatch::handle_use_ability_on_ground;
pub(crate) use loot_drop::INT_NORMAL_LOOT;
pub(crate) use messaging::{
    broadcast_movement_type, request_appearance_refresh, send_entity_method,
    send_entity_method_to_witnesses,
};
// `send_entity_method_to_witnesses` and `send_entity_method_to_self_and_witnesses`
// land here for #278 child PRs to adopt. They stay private to the `messaging`
// module until the first child callsite migrates — at which point the
// migrating PR adds the re-exports it needs.
pub use resolve::{
    ability_for_active_weapon, ability_for_item, is_ability_granted_by_active_weapon,
};
pub use use_ability::{handle_use_ability, handle_use_ability_with_kill_credit};

#[cfg(test)]
pub(crate) use use_ability::maybe_trigger_auto_reload_for_test;
