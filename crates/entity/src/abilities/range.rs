//! Ability range units and the server-wide range default.
//!
//! # The unit
//!
//! `resources.abilities.min_range` / `max_range` (and the `MinRange` /
//! `MaxRange` attributes of the client's `CookedDataAbilities.pak`, which
//! carry the same numbers, e.g. 1652 Jaffa: Double Blast `MaxRange="3000"`)
//! are **UE3 units: 100 per BigWorld metre**. The server's positions are
//! BigWorld metres, so the loader divides by
//! [`ABILITY_RANGE_UNITS_PER_METRE`] once and every [`AbilityDef`] range is
//! in metres from then on (issue #919).
//!
//! Evidence (`docs/reverse-engineering/findings/ability-resolution-pipeline.md`
//! § "Range units"): the 2009 client's ground-target reticule
//! (`SGW_ETargetType_TargetGround` `0x00dea330` → `FUN_00eadf00` →
//! the tick at `0x00eae080`) clamps the reticule against the ability's raw
//! `MaxRange`/`MinRange` in UE3 world space, next to AoE radii that the
//! client converts to UE3 units (`FUN_00d29e90`: Medium = 1000 for the
//! 10 m `AE_RADIUS_Medium`). Every non-zero seeded range is a multiple of
//! 100, and 3000 is the 30 m server default below.
//!
//! Weapon ranges are **not** in these units: `resources.items`
//! `*_ranged_range` / `*_melee_range` are already metres (30, 2, 3).

use super::AbilityDef;

/// UE3 units per BigWorld metre in ability `MinRange` / `MaxRange` data.
/// The client's own conversion constant is `BW_TO_UE3_SCALE = 100.0`
/// (`0x018cad90`).
pub const ABILITY_RANGE_UNITS_PER_METRE: f32 = 100.0;

/// Reach, in metres, of an ability whose `max_range` is the `0` sentinel
/// ("no ability-specific range"). Matches the 30 m the shipped ranged
/// weapons express and the `3000` most ranged abilities carry.
pub const DEFAULT_ABILITY_MAX_RANGE: f32 = 30.0;

/// Convert a raw `resources.abilities` range (UE3 units) to metres.
pub fn ability_range_to_metres(raw: i32) -> f32 {
    raw as f32 / ABILITY_RANGE_UNITS_PER_METRE
}

impl AbilityDef {
    /// The ability's reach in metres, or [`DEFAULT_ABILITY_MAX_RANGE`] when
    /// `max_range` is the `0` sentinel.
    pub fn max_range_or_default(&self) -> f32 {
        if self.max_range > 0.0 {
            self.max_range
        } else {
            DEFAULT_ABILITY_MAX_RANGE
        }
    }
}

/// [`AbilityDef::max_range_or_default`] for an optional def: an unknown
/// ability gets [`DEFAULT_ABILITY_MAX_RANGE`].
pub fn ability_max_range(def: Option<&AbilityDef>) -> f32 {
    def.map_or(DEFAULT_ABILITY_MAX_RANGE, AbilityDef::max_range_or_default)
}

/// The radius, in metres, of a `TCM_AERadius` effect's `tcm_param1` tier,
/// by the client's own table (`AbilityInfo_AERadiusFromTier`, `0x00d29e90`:
/// Melee 250, Short 500, Medium 1000, Long 1500, Extreme 2000 UE3 units).
/// `None` for a tier the client does not know.
///
/// Not the cone table (`EffectDef::tcm_range_meters`, whose "Medium" is a
/// server-side 8 m): the client converts only AE radii this way.
pub fn ae_radius_metres(tier: &str) -> Option<f32> {
    let ue3 = match tier {
        "Melee" => 250.0,
        "Short" => 500.0,
        "Medium" => 1000.0,
        "Long" => 1500.0,
        "Extreme" => 2000.0,
        _ => return None,
    };
    Some(ue3 / ABILITY_RANGE_UNITS_PER_METRE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(max_range: f32) -> AbilityDef {
        AbilityDef {
            ability_id: 1652,
            name: "Jaffa: Double Blast".into(),
            cooldown: 4.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range,
            target_type_id: 2,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 100.0,
        }
    }

    #[test]
    fn seeded_ranges_convert_to_metres() {
        assert_eq!(ability_range_to_metres(3000), 30.0, "1652 Double Blast");
        assert_eq!(ability_range_to_metres(800), 8.0, "1653 Heal Health");
        assert_eq!(ability_range_to_metres(300), 3.0, "1205 turret min_range");
        assert_eq!(ability_range_to_metres(0), 0.0, "0 stays the sentinel");
    }

    #[test]
    fn zero_max_range_uses_the_default() {
        assert_eq!(def(0.0).max_range_or_default(), DEFAULT_ABILITY_MAX_RANGE);
        assert_eq!(def(8.0).max_range_or_default(), 8.0);
        assert_eq!(ability_max_range(None), DEFAULT_ABILITY_MAX_RANGE);
    }

    #[test]
    fn ae_radius_tiers_are_the_clients() {
        assert_eq!(ae_radius_metres("Melee"), Some(2.5));
        assert_eq!(ae_radius_metres("Short"), Some(5.0));
        assert_eq!(ae_radius_metres("Medium"), Some(10.0), "5066, 1012's pulse");
        assert_eq!(ae_radius_metres("Long"), Some(15.0));
        assert_eq!(ae_radius_metres("Extreme"), Some(20.0));
        assert_eq!(ae_radius_metres("Weapon"), None);
        assert_eq!(ae_radius_metres(""), None);
    }
}
