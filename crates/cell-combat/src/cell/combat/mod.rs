//! Combat: damage resolution, dead-state flags, and NPC threat management.
//!
//! Submodules:
//! - [`aggression`]: effective NPC aggression (override, else faction
//!   reaction), aggro radius and vertical band (NA13).
//! - [`faction_reaction`]: the 2009 `FACTION_REACTION_TABLE`.
//!
//! `aggression`, `faction_reaction` and `health_threshold` are in
//! `cimmeria-cell-world` (wave C1 of `docs/architecture/services-crate-split.md`):
//! spawning and the NPC detectors read them, and `SpaceManager` queues the
//! health sample. They are re-exported here at their old paths.
//! - [`damage`]: QR (Quality Rating) hit/miss/crit and damage pipeline.
//! - [`damage_credit`]: the seam-level pre-hit health sample every damage
//!   path queues for the `entity_health_below` content trigger.
//! - [`health_threshold`]: health-percentage snapshots for the
//!   `entity_health_below` content trigger.
//! - [`state`]: dead/alive flag bit-packing in `stateField`.
//! - [`threat`]: NPC aggro state, threat list, leash/attack-range constants.

pub use cimmeria_cell_world::cell::combat::{aggression, faction_reaction, health_threshold};
pub mod auto_cycle;
pub mod damage;
pub mod damage_credit;
pub mod state;
pub mod threat;

pub use aggression::{
    aggression_toward_players, aggro_radius, area_candidates, assist_radius, effective_aggression,
    faction_has_npc_enemies, is_hostile_to_players, is_npc_combatant, may_hit_in_area,
    npc_aggression_toward, npc_may_target_npc, override_from_content_level, player_may_attack,
    player_may_attack_pve, seeks_npc_targets, AGGRO_VERTICAL_BAND, DEFAULT_AGGRO_RADIUS,
    DEFAULT_ASSIST_RADIUS, PLAYER_REACTION_FACTION,
};
pub use auto_cycle::{
    arm_auto_cycle, clear_auto_cycle, clear_auto_cycle_for_target, is_auto_cycle_target_valid,
};
pub use damage::{
    attacker_cover_qr, calculate_damage, calculate_damage_penetrating, calculate_damage_scaled,
    calculate_qr, calculate_result, cover_reduction, CoverReduction, CoverSide, QrResult,
};
pub use damage_credit::{note_pre_damage_health, HealthBelowSample};
pub use health_threshold::{health_pct, health_pct_from, HealthPct};

// The hostile-faction sentinel lives with the faction reaction table, in
// cimmeria-cell-world, because spawning reads it.
pub use faction_reaction::HOSTILE_FACTION;

pub use state::{
    is_dead_state, mark_npc_dead, BSF_AUTO_CYCLING, BSF_DEAD, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK,
    PERSISTED_STATE_FIELD_MASK, PLAYER_STATE_DEAD,
};
pub use threat::{
    clear_dead_npc_from_all_player_threat, drain_npc_from_player_combat, enter_player_combat,
    exit_player_combat, generate_threat, AggroCause, HOLSTER_ANIMATION_DURATION, LEASH_DISTANCE,
    NPC_ATTACK_RANGE, NPC_DEFAULT_ABILITY, NPC_MELEE_RANGE, OOC_HOLSTER_DELAY,
};
