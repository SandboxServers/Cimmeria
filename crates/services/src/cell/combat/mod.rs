//! Combat: damage resolution, dead-state flags, and NPC threat management.
//!
//! Submodules:
//! - [`damage`]: QR (Quality Rating) hit/miss/crit and damage pipeline.
//! - [`state`]: dead/alive flag bit-packing in `stateField`.
//! - [`threat`]: NPC aggro state, threat list, leash/attack-range constants.

pub mod auto_cycle;
pub mod damage;
pub mod state;
pub mod threat;

pub use auto_cycle::{arm_auto_cycle, clear_auto_cycle, clear_auto_cycle_for_target};
pub use damage::{calculate_damage, calculate_qr, calculate_result, QrResult};
pub use state::{
    is_dead_state, BSF_AUTO_CYCLING, BSF_DEAD, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK, PLAYER_STATE_DEAD,
};
pub use threat::{
    clear_dead_npc_from_all_player_threat, enter_player_combat, exit_player_combat,
    generate_threat, LEASH_DISTANCE, NPC_ATTACK_RANGE, NPC_DEFAULT_ABILITY, OOC_HOLSTER_DELAY,
};
