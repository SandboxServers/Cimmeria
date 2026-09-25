//! Per-NPC aggression: the `EMobAggressionLevel` enum and the runtime
//! override/radius pair the Idle auto-aggro scan reads (NA13, D-NA01).
//!
//! The level runs the way the client and the 2009 server ran it: **low is
//! hostile**. `EMobAggressionLevel` in `entities/defs/enumerations.xml`
//! (1 HOSTILE, 2 SUSPICIOUS, 3 NEUTRAL, 4 FRIENDLY, 5 DEFAULT), mirrored in
//! `deprecated/python/Atrea/enums.py` (`AGGRESSION_*`). Before NA13 the Rust
//! field was an `i32` where `> 0` meant "aggressive", which read 2-5 as
//! hostile too (audit A5/A6). Only [`MobAggression::Hostile`] aggroes on
//! sight now.
//!
//! The *effective* level of an NPC toward a viewer is the override when one
//! is set, otherwise the faction reaction table
//! (`cimmeria_services::cell::combat::faction_reaction`), exactly as
//! `SGWPlayer.getAggressionLevel` derived it in the python reference.

use serde::{Deserialize, Serialize};

/// `EMobAggressionLevel` (`enumerations.xml:437-446`). The discriminants are
/// the wire/seed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MobAggression {
    /// Attacks on sight. The only level the Idle auto-aggro scan acts on.
    Hostile = 1,
    /// Wary; not aggressive on sight.
    Suspicious = 2,
    /// Fights back only when attacked.
    Neutral = 3,
    /// Friendly.
    Friendly = 4,
    /// The enum's "no opinion" value. Treated as not hostile, matching the
    /// python reference, which compared the stored level against NEUTRAL.
    Default = 5,
}

impl MobAggression {
    /// The level for a seed or wire value, or `None` outside 1..=5.
    pub fn from_level(level: i32) -> Option<Self> {
        match level {
            1 => Some(Self::Hostile),
            2 => Some(Self::Suspicious),
            3 => Some(Self::Neutral),
            4 => Some(Self::Friendly),
            5 => Some(Self::Default),
            _ => None,
        }
    }

    /// The seed/wire value (1..=5).
    pub fn level(self) -> u8 {
        self as u8
    }

    /// Whether this level aggroes on sight.
    pub fn is_hostile(self) -> bool {
        self == Self::Hostile
    }

    /// Lower-case name for logs and GM feedback.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hostile => "hostile",
            Self::Suspicious => "suspicious",
            Self::Neutral => "neutral",
            Self::Friendly => "friendly",
            Self::Default => "default",
        }
    }
}

/// Aggression configuration of one NPC. Server-side runtime state; the
/// override is seeded from `spawnlist.aggression_override` and changed at
/// runtime by the `set_aggression` content action, the `spawn_entity`
/// action's `aggression` parameter, the GM `.aggression` command and the
/// surrender path.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AggroProfile {
    /// Runtime override of the faction-derived level. `None` means the
    /// faction reaction table decides (python `aggressionOverride = None`).
    pub override_level: Option<MobAggression>,
    /// Per-template proximity-aggro radius in world units, from
    /// `entity_templates.aggro_radius`. `None` means the server default
    /// (`cell::combat::DEFAULT_AGGRO_RADIUS`, 18 u) applies.
    pub radius_override: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_round_trip_and_follow_the_enum_direction() {
        for v in 1..=5 {
            assert_eq!(MobAggression::from_level(v).unwrap().level() as i32, v);
        }
        assert_eq!(MobAggression::from_level(0), None);
        assert_eq!(MobAggression::from_level(6), None);
        assert_eq!(MobAggression::from_level(-1), None);
        // Low is hostile: 1 and only 1 aggroes on sight.
        assert!(MobAggression::Hostile.is_hostile());
        for v in 2..=5 {
            assert!(!MobAggression::from_level(v).unwrap().is_hostile());
        }
    }
}
