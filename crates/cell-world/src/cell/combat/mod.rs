//! The world-side half of combat: what spawning and the NPC detectors read
//! about an NPC's disposition, and the health sample `SpaceManager` queues.
//!
//! - [`aggression`]: effective NPC aggression (override, else faction
//!   reaction), aggro radius and vertical band (NA13), and the default NPC
//!   attack ability.
//! - [`faction_reaction`]: the 2009 `FACTION_REACTION_TABLE` and the
//!   hostile-faction sentinel.
//! - [`health_threshold`]: health-percentage snapshots for the
//!   `entity_health_below` content trigger, and the queued pre-hit sample.
//!
//! Damage, threat, the auto-cycle and the dead-state helpers are combat
//! proper and live in `cimmeria-cell-combat`'s `cell::combat`, which
//! re-exports these modules at their old paths.

pub mod aggression;
pub mod faction_reaction;
pub mod health_threshold;

pub use aggression::{
    aggression_toward_players, aggro_radius, area_candidates, assist_radius, effective_aggression,
    is_hostile_to_players, may_hit_in_area, override_from_content_level, player_may_attack,
    AGGRO_VERTICAL_BAND, DEFAULT_AGGRO_RADIUS, DEFAULT_ASSIST_RADIUS, NPC_DEFAULT_ABILITY,
    PLAYER_REACTION_FACTION,
};
pub use faction_reaction::HOSTILE_FACTION;
pub use health_threshold::{health_pct, health_pct_from, HealthBelowSample, HealthPct};
