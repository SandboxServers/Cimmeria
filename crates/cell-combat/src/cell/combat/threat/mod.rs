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
//! - [`release`]: the same drain, with its state-field sends, for an NPC that
//!   leaves without dying (GM despawn, reset, content despawn).

mod aggro;
mod player_combat;
mod release;

pub use aggro::{
    generate_threat, AggroCause, HOLSTER_ANIMATION_DURATION, LEASH_DISTANCE, NPC_ATTACK_RANGE,
    NPC_DEFAULT_ABILITY, NPC_MELEE_RANGE, OOC_HOLSTER_DELAY,
};
pub use player_combat::{
    clear_dead_npc_from_all_player_threat, drain_npc_from_player_combat, enter_player_combat,
    exit_player_combat,
};
pub use release::{despawn_npc_releasing_combat, release_npc_from_player_combat};
