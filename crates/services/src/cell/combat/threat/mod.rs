//! NPC aggro and threat list management.
//!
//! Threat is accumulated per attacker on each NPC. The NPC AI picks its
//! current target as the entity with the highest threat. First hit also
//! transitions the NPC from `Idle` to `Fighting`.
//!
//! Submodules:
//! - [`aggro`]: NPC threat-table mutation, the Idle→Fighting preempt, and
//!   the leash/attack-range constants.
//! - [`player_combat`]: player-side `threatened_mobs` / `BSF_IN_COMBAT`
//!   bookkeeping and the dead-NPC sweep.

mod aggro;
mod player_combat;

pub use aggro::{
    generate_threat, LEASH_DISTANCE, NPC_ATTACK_RANGE, NPC_DEFAULT_ABILITY, OOC_HOLSTER_DELAY,
};
pub use player_combat::{
    clear_dead_npc_from_all_player_threat, enter_player_combat, exit_player_combat,
};
