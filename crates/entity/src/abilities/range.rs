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
//! `*_ranged_range` / `*_melee_range` are already metres (30, 2, 3), and
//! an ability flagged `UseWeaponRange` takes its bounds from them
//! ([`ability_range_bounds`], #1017).

use std::collections::HashMap;

use super::{AbilityDef, AF_USE_WEAPON_RANGE};
use crate::cell_entity::CellEntity;

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

/// The reach, in metres, of a cast of `def` with `weapon` equipped: the
/// `max` of [`ability_range_bounds`]. An unknown ability gets
/// [`DEFAULT_ABILITY_MAX_RANGE`].
pub fn ability_max_range(def: Option<&AbilityDef>, weapon: Option<&WeaponRanges>) -> f32 {
    ability_range_bounds(def, weapon).max
}

/// An item's weapon reach, in metres, from `resources.items`
/// `min_ranged_range` / `max_ranged_range` / `min_melee_range` /
/// `max_melee_range`. These are already metres (30 / 35 / 40, melee 2 / 3)
/// and are never converted: the cooked items' `RangeRanges` / `MeleeRanges`
/// carry the same numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeaponRanges {
    pub min_ranged: f32,
    pub max_ranged: f32,
    pub min_melee: f32,
    pub max_melee: f32,
}

impl WeaponRanges {
    /// The `(min, max)` pair for a ranged or a melee ability, the choice the
    /// client's getter makes on the ability's `IsRanged` (`FUN_00d29da0`) and
    /// python's `getWeaponRange(self.ability.ranged)`. `None` when the weapon
    /// has no reach of that kind (`max` 0), which falls back to the
    /// ability's own range like the `0` sentinel does.
    pub fn for_kind(&self, ranged: bool) -> Option<(f32, f32)> {
        let (min, max) = if ranged {
            (self.min_ranged, self.max_ranged)
        } else {
            (self.min_melee, self.max_melee)
        };
        (max > 0.0).then_some((min.max(0.0), max))
    }
}

/// Where a cast's [`RangeBounds`] came from (logged with a refusal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeSource {
    /// The ability's own `min_range` / `max_range`.
    Ability,
    /// The equipped weapon's reach (`UseWeaponRange`, #1017).
    Weapon,
    /// `UseWeaponRange`, but no weapon with a reach of the ability's kind is
    /// equipped: the ability's own range applies (#1017).
    AbilityNoWeapon,
}

impl RangeSource {
    /// The `range_source=` value of a log row.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ability => "ability",
            Self::Weapon => "weapon",
            Self::AbilityNoWeapon => "ability_no_weapon",
        }
    }
}

/// The distances, in metres, a targeted cast may reach: at least `min`, at
/// most `max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeBounds {
    /// The minimum; `0` means none.
    pub min: f32,
    /// The reach.
    pub max: f32,
    /// Ability data or the weapon's.
    pub source: RangeSource,
}

/// Why a target's distance fails a cast's [`RangeBounds`]. Both get the same
/// client feedback, `CONDITION_FEEDBACK_OutsideWeaponRange` (42): the 2009
/// Python reference (`AbilityManager.py:561`) refuses
/// `distance < minRange or distance > maxRange` with that one code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeRefusal {
    /// Farther than `max`.
    TooFar,
    /// Closer than `min` (#1016).
    TooClose,
}

impl RangeRefusal {
    /// The `reason=` value of the refusal's log row.
    pub fn reason(self) -> &'static str {
        match self {
            Self::TooFar => "target_out_of_range",
            Self::TooClose => "target_too_close",
        }
    }
}

impl RangeBounds {
    /// Whether `distance` fails these bounds. `enforce_min` is false for an
    /// NPC caster: the NPC fight tick owns its minimum-range behaviour (a
    /// mobile NPC backs away, a stationary one keeps firing), so only a
    /// player's cast is refused inside `min` (#1016).
    pub fn refusal(&self, distance: f32, enforce_min: bool) -> Option<RangeRefusal> {
        if distance > self.max {
            Some(RangeRefusal::TooFar)
        } else if enforce_min && distance < self.min {
            Some(RangeRefusal::TooClose)
        } else {
            None
        }
    }
}

/// The range bounds of a cast of `def` with `weapon` equipped, in metres.
/// The single place every targeted-cast range check resolves its numbers.
///
/// - An ability flagged [`AF_USE_WEAPON_RANGE`] (4) takes both bounds from
///   the equipped weapon, ranged or melee by the ability's `is_ranged` (the
///   client's getters `FUN_00d29e00` / `FUN_00d29e30`, python
///   `AbilityManager.py:555`). #1017.
/// - With no weapon, or a weapon with no reach of that kind, it falls back
///   to the ability's own range. Python has no deliberate answer (its
///   player path would raise on `getActiveItem().type`, and its mobs always
///   had a template weapon); here NPCs and pets carry no weapon item, and
///   every seeded NPC attack (592 included) carries the flag, so refusing
///   would disarm every NPC.
/// - Otherwise the ability's `min_range` and
///   [`AbilityDef::max_range_or_default`]. An unknown ability has no
///   minimum and the default reach.
pub fn ability_range_bounds(
    def: Option<&AbilityDef>,
    weapon: Option<&WeaponRanges>,
) -> RangeBounds {
    let own = |source| RangeBounds {
        min: def.map_or(0.0, |d| d.min_range.max(0.0)),
        max: def.map_or(DEFAULT_ABILITY_MAX_RANGE, AbilityDef::max_range_or_default),
        source,
    };
    let Some(d) = def.filter(|d| d.flags & AF_USE_WEAPON_RANGE != 0) else {
        return own(RangeSource::Ability);
    };
    match weapon.and_then(|w| w.for_kind(d.is_ranged)) {
        Some((min, max)) => RangeBounds {
            min,
            max,
            source: RangeSource::Weapon,
        },
        None => own(RangeSource::AbilityNoWeapon),
    }
}

/// The reach of the weapon in `entity`'s active bandolier slot, looked up
/// by its design id in `table` (`SpaceManager::weapon_ranges`). `None` for
/// an empty slot, an NPC or pet (no bandolier), or an item with no reach.
pub fn active_weapon_ranges<'a>(
    entity: &CellEntity,
    table: &'a HashMap<i32, WeaponRanges>,
) -> Option<&'a WeaponRanges> {
    let item = entity.bandolier_items.get(&entity.active_bandolier_slot)?;
    table.get(&item.item_id)
}

/// [`ability_range_bounds`] for `caster`, with its active weapon.
pub fn caster_range_bounds(
    def: Option<&AbilityDef>,
    caster: &CellEntity,
    weapons: &HashMap<i32, WeaponRanges>,
) -> RangeBounds {
    ability_range_bounds(def, active_weapon_ranges(caster, weapons))
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
            type_id: Default::default(),
            passive: false,
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
        assert_eq!(ability_max_range(None, None), DEFAULT_ABILITY_MAX_RANGE);
    }

    /// #1016: a 3 m minimum refuses a player at 1 m and allows one at 5 m.
    /// An NPC caster is not held to the minimum.
    #[test]
    fn min_range_refuses_a_player_inside_it() {
        let mut turret = def(30.0);
        turret.min_range = 3.0;
        let bounds = ability_range_bounds(Some(&turret), None);
        assert_eq!(bounds.refusal(1.0, true), Some(RangeRefusal::TooClose));
        assert_eq!(bounds.refusal(5.0, true), None);
        assert_eq!(bounds.refusal(31.0, true), Some(RangeRefusal::TooFar));
        assert_eq!(bounds.refusal(1.0, false), None, "NPCs keep their own rule");
        assert_eq!(ability_range_bounds(None, None).min, 0.0);
    }

    /// A 40 m rifle: ranged 2-40 m, melee 0-2 m.
    const RIFLE: WeaponRanges = WeaponRanges {
        min_ranged: 2.0,
        max_ranged: 40.0,
        min_melee: 0.0,
        max_melee: 2.0,
    };

    fn weapon_ranged_ability() -> AbilityDef {
        let mut d = def(0.0);
        d.flags = AF_USE_WEAPON_RANGE;
        d
    }

    /// #1017: a `UseWeaponRange` ability with a 40 m weapon is in range at
    /// 35 m and refused at 45 m. Revert proof: with the flag ignored the
    /// ability's own 30 m default refuses 35 m.
    #[test]
    fn use_weapon_range_takes_the_weapons_reach() {
        let b = ability_range_bounds(Some(&weapon_ranged_ability()), Some(&RIFLE));
        assert_eq!((b.min, b.max, b.source), (2.0, 40.0, RangeSource::Weapon));
        assert_eq!(b.refusal(35.0, true), None);
        assert_eq!(b.refusal(45.0, true), Some(RangeRefusal::TooFar));
        assert_eq!(b.refusal(1.0, true), Some(RangeRefusal::TooClose));
    }

    #[test]
    fn a_melee_weapon_range_ability_takes_the_melee_pair() {
        let mut melee = weapon_ranged_ability();
        melee.is_ranged = false;
        let b = ability_range_bounds(Some(&melee), Some(&RIFLE));
        assert_eq!((b.min, b.max), (0.0, 2.0));
    }

    /// No weapon, or a weapon with no reach of the ability's kind: the
    /// ability's own range applies.
    #[test]
    fn use_weapon_range_without_a_weapon_falls_back_to_the_ability() {
        let mut d = weapon_ranged_ability();
        d.max_range = 8.0;
        let b = ability_range_bounds(Some(&d), None);
        assert_eq!(
            (b.min, b.max, b.source),
            (0.0, 8.0, RangeSource::AbilityNoWeapon)
        );
        let melee_only = WeaponRanges {
            max_melee: 2.0,
            ..WeaponRanges::default()
        };
        let b = ability_range_bounds(Some(&d), Some(&melee_only));
        assert_eq!((b.max, b.source), (8.0, RangeSource::AbilityNoWeapon));
    }

    /// An unflagged ability ignores the weapon.
    #[test]
    fn an_unflagged_ability_ignores_the_weapon() {
        let b = ability_range_bounds(Some(&def(0.0)), Some(&RIFLE));
        assert_eq!(
            (b.max, b.source),
            (DEFAULT_ABILITY_MAX_RANGE, RangeSource::Ability)
        );
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
