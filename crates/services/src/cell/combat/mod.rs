//! Combat: damage resolution, dead-state flags, and NPC threat management.
//!
//! Submodules:
//! - [`damage`]: QR (Quality Rating) hit/miss/crit and damage pipeline.
//! - [`state`]: dead/alive flag bit-packing in `stateField`.
//! - [`threat`]: NPC aggro state, threat list, leash/attack-range constants.

pub mod damage;
pub mod state;
pub mod threat;

pub use damage::{calculate_damage, calculate_qr, calculate_result, QrResult};
pub use state::{
    clear_dead_state, is_dead_state, set_dead_state, BSF_DEAD, BSF_HOLSTER, BSF_IN_COMBAT,
    PLAYER_STATE_DEAD,
};
pub use threat::{
    clear_dead_npc_from_all_player_threat, enter_player_combat, exit_player_combat,
    generate_threat, LEASH_DISTANCE, NPC_ATTACK_RANGE, NPC_DEFAULT_ABILITY,
};
