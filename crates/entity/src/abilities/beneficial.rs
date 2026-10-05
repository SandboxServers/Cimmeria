//! Whether an ability helps whoever it lands on (ability-mechanics D-AB02).
//!
//! A beneficial ability is a heal or a buff: the cell resolves it on the
//! caster or an ally and never on a hostile, with no QR roll, no threat and
//! no in-combat state (`cell-combat`'s `use_ability/beneficial.rs`). Every
//! other ability keeps the damage pipeline and its #444 target gate.
//!
//! The rule, from the 2009 data. At least one effect must do something
//! ([`effect_is_implemented`]), and every effect that does something must be
//! one of:
//!
//! - an effect carrying `EF_Beneficial_Effect`, **or**
//! - on an `ABILITY_TYPE_Heal` ability only, an effect whose script is a heal
//!   ([`HEAL_SCRIPTS`]). 597 Heal Focus, 1646 Health Heal and 1218
//!   Recuperation qualify this way: their effects carry no beneficial bit
//!   (659 is flags 16), **or**
//! - on any ability, an effect whose script only ever helps its target
//!   (AB-10): an absorb shield (`AbsorbShield`) or a cleanse of harmful
//!   effects (`RemoveEffects` without `RemovePolarity = Beneficial`). Their
//!   rows carry no beneficial bit either (4306 Personal Shield is flags 342,
//!   4168 Absolution's purge is 0), and on the hostile path a Self shield or
//!   purge would land on the client's target. A buff strip
//!   (`RemovePolarity = Beneficial`) is the opposite and does not qualify.
//!
//! **The type alone is not enough.** About 200 seed abilities are typed Heal
//! but are debuffs or crowd control (1874 Impose Weakness, 1937 FocusDegen,
//! 1988 InduceDaze, 2090 TurnAndDie, 2154 ShutDown, 3253 ConvertEnergy:
//! Enemy). Today their effects do nothing, so they fail the "at least one"
//! rule; once a generator binds a debuff script to one, that script is not a
//! heal and has no beneficial bit, so it still fails.
//!
//! **Damage vetoes both.** An ability with an effect that deals damage
//! (`HealthDamage` or `FocusDamage` above zero) is never beneficial, whatever
//! its type: the seed has a Heal-typed attack (2228 `MS020_080818_CallTarget`,
//! effect 3091 `RangedPhysicalDamage`), and calling it beneficial would land
//! that damage on the caster or an ally and skip the #444 gate.
//!
//! The "at least one" keeps an ability whose effects do nothing out of the
//! beneficial path: an unimplemented attack must not start skipping the
//! #444 gate because its effect list is vacuously "all beneficial", and
//! neither may a Heal-typed debuff.
//!
//! **A stance's removal half is not counted** (ability mechanics AB-08).
//! "Remove Effect of moniker EFFECT_Stance" (`RemoveByMoniker`) carries
//! flags 0 and changes nothing on its own; it only clears the caster's
//! previous stance. Counted, it would send every stance down the hostile
//! path.

use std::collections::HashMap;

use super::{
    effect_is_implemented, AbilityDef, AbilityType, EffectDef, EF_BENEFICIAL_EFFECT,
    REMOVE_BY_MONIKER_SCRIPT,
};

/// The scripts that only restore a pool. On a Heal-typed ability they count
/// as beneficial without the `EF_Beneficial_Effect` bit.
pub const HEAL_SCRIPTS: [&str; 3] = ["HealHealth", "HealFocus", "HealPetHealth"];

/// `true` when `effect`'s script only ever helps whoever it lands on, on
/// any ability type (module docs). The names are `cimmeria-cell-effect-scripts`'
/// (`shield/`, `cleanse/`).
fn effect_script_only_helps(effect: &EffectDef) -> bool {
    match effect.script_name.as_deref() {
        Some("AbsorbShield") => true,
        Some("RemoveEffects") => !effect
            .params
            .get("RemovePolarity")
            .is_some_and(|p| p.trim().eq_ignore_ascii_case("beneficial")),
        _ => false,
    }
}

/// `true` when `effect` deals damage from its NVPs.
fn effect_deals_damage(effect: &EffectDef) -> bool {
    effect.param_i32("HealthDamage") > 0 || effect.param_i32("FocusDamage") > 0
}

/// Whether `def` is beneficial (module docs). An effect id missing from
/// `effects` does nothing, as in [`effect_is_implemented`].
pub fn ability_is_beneficial(def: &AbilityDef, effects: &HashMap<i32, EffectDef>) -> bool {
    let doing: Vec<&EffectDef> = def
        .effect_ids
        .iter()
        .filter_map(|id| effects.get(id))
        .filter(|e| effect_is_implemented(Some(e)))
        // A stance's "Remove Effect of moniker EFFECT_Stance" half carries
        // no beneficial bit (flags 0) but only clears the caster's previous
        // stance: counted, it would send every stance down the hostile path.
        .filter(|e| e.script_name.as_deref() != Some(REMOVE_BY_MONIKER_SCRIPT))
        .collect();
    if doing.is_empty() || doing.iter().any(|e| effect_deals_damage(e)) {
        return false;
    }
    let heal_typed = def.type_id == AbilityType::Heal;
    doing.iter().all(|e| {
        e.flags & EF_BENEFICIAL_EFFECT != 0
            || effect_script_only_helps(e)
            || (heal_typed
                && e.script_name
                    .as_deref()
                    .is_some_and(|s| HEAL_SCRIPTS.contains(&s)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(type_id: AbilityType, effect_ids: Vec<i32>) -> AbilityDef {
        AbilityDef {
            ability_id: 597,
            name: "Heal Focus".to_string(),
            cooldown: 30.0,
            warmup: 2.0,
            flags: 656,
            is_ranged: false,
            min_range: 0.0,
            max_range: 0.0,
            target_type_id: 1,
            effect_ids,
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 100.0,
            type_id,
            passive: false,
        }
    }

    fn effect(id: i32, flags: u32, params: &[(&str, &str)], script: Option<&str>) -> EffectDef {
        EffectDef {
            effect_id: id,
            flags,
            script_name: script.map(str::to_string),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ..EffectDef::default()
        }
    }

    /// 597's shape: Heal type, effect 659 `HealFocus` with flags 16 (no
    /// beneficial bit). A heal script on a Heal-typed ability qualifies.
    #[test]
    fn a_heal_typed_ability_is_beneficial_without_the_effect_bit() {
        let effects = HashMap::from([(
            659,
            effect(659, 16, &[("HealPercentage", "35.00")], Some("HealFocus")),
        )]);
        assert!(ability_is_beneficial(
            &def(AbilityType::Heal, vec![659]),
            &effects
        ));
        // The same effect on an Undefined-typed ability is not: bit 1 is off.
        assert!(!ability_is_beneficial(
            &def(AbilityType::Undefined, vec![659]),
            &effects
        ));
    }

    #[test]
    fn every_doing_effect_flagged_beneficial_makes_a_buff_beneficial() {
        let effects = HashMap::from([
            (1, effect(1, 21, &[], Some("PetStatBuff"))),
            (2, effect(2, 1, &[], Some("StatBuff"))),
            (3, effect(3, 0, &[], None)), // does nothing: ignored
            (4, effect(4, 0, &[], Some("StatBuff"))),
        ]);
        assert!(ability_is_beneficial(
            &def(AbilityType::Buff, vec![1, 2, 3]),
            &effects
        ));
        // One unflagged effect that does something spoils it.
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![1, 4]),
            &effects
        ));
    }

    /// **Regression guard (server-authority review S1).** The Heal type is
    /// not enough: 1874 Impose Weakness and its kin are Heal-typed debuffs.
    /// With no effect that does something, or with a non-heal script that
    /// lacks the beneficial bit, they are not beneficial. On revert to
    /// "Heal type wins" the first three asserts fail.
    #[test]
    fn a_heal_typed_debuff_is_not_beneficial() {
        let effects = HashMap::from([
            (3, effect(3, 1, &[], None)),
            (7, effect(7, 2, &[], Some("StatBuff"))),
        ]);
        assert!(!ability_is_beneficial(
            &def(AbilityType::Heal, vec![]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Heal, vec![3]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Heal, vec![7]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![3]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::DirectDamage, vec![]),
            &effects
        ));
    }

    /// A heal script without the bit counts only on a Heal-typed ability.
    #[test]
    fn a_heal_script_without_the_bit_needs_the_heal_type() {
        let effects = HashMap::from([(
            2008,
            effect(2008, 16, &[("HealPercentage", "10")], Some("HealHealth")),
        )]);
        assert!(ability_is_beneficial(
            &def(AbilityType::Heal, vec![2008]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![2008]),
            &effects
        ));
    }

    /// 2228's shape: Heal-typed, effect 3091 deals damage. Never beneficial,
    /// so it keeps the damage pipeline and the #444 gate.
    #[test]
    fn damage_vetoes_the_heal_type_and_the_beneficial_bit() {
        let effects = HashMap::from([
            (
                3091,
                effect(
                    3091,
                    528,
                    &[("HealthDamage", "30")],
                    Some("RangedPhysicalDamage"),
                ),
            ),
            (5, effect(5, 1, &[("FocusDamage", "100")], None)),
        ]);
        assert!(!ability_is_beneficial(
            &def(AbilityType::Heal, vec![3091]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![5]),
            &effects
        ));
    }

    /// 859 Concentration's shape: a beneficial held buff (922, flags 85) and
    /// its "Remove Effect of moniker EFFECT_Stance" half (4294, flags 0,
    /// `RemoveByMoniker`). The removal does not make the stance hostile; on
    /// its own it is not a beneficial cast either.
    #[test]
    fn a_stance_removal_half_does_not_make_the_stance_hostile() {
        let effects = HashMap::from([
            (
                922,
                effect(
                    922,
                    85,
                    &[("InterruptResistance", "250")],
                    Some("TimedStat"),
                ),
            ),
            (
                4294,
                effect(
                    4294,
                    0,
                    &[("RemoveMoniker", "EFFECT_Stance")],
                    Some(REMOVE_BY_MONIKER_SCRIPT),
                ),
            ),
        ]);
        assert!(ability_is_beneficial(
            &def(AbilityType::Buff, vec![922, 4294]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![4294]),
            &effects
        ));
    }

    /// AB-10: Personal Shield (1013, DD-typed, 4306 flags 342) and
    /// Absolution (2865, 4168 flags 0) carry no beneficial bit, but a shield
    /// and a purge only ever help their target, so the Self cast lands on
    /// the caster. A buff strip is the opposite.
    #[test]
    fn a_shield_or_a_purge_is_beneficial_and_a_buff_strip_is_not() {
        let effects = HashMap::from([
            (
                4306,
                effect(4306, 342, &[("ShieldAmount", "500")], Some("AbsorbShield")),
            ),
            (
                4168,
                effect(
                    4168,
                    0,
                    &[("RemoveCategories", "Mental:2")],
                    Some("RemoveEffects"),
                ),
            ),
            (
                9,
                effect(
                    9,
                    0,
                    &[
                        ("RemoveCategories", "Mental"),
                        ("RemovePolarity", "Beneficial"),
                    ],
                    Some("RemoveEffects"),
                ),
            ),
        ]);
        assert!(ability_is_beneficial(
            &def(AbilityType::DirectDamage, vec![4306]),
            &effects
        ));
        assert!(ability_is_beneficial(
            &def(AbilityType::Buff, vec![4168]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![9]),
            &effects
        ));
    }
}
