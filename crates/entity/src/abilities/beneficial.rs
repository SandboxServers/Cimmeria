//! Whether an ability helps whoever it lands on (ability-mechanics D-AB02).
//!
//! A beneficial ability is a heal or a buff: the cell resolves it on the
//! caster or an ally and never on a hostile, with no QR roll, no threat and
//! no in-combat state (`cell-combat`'s `use_ability/beneficial.rs`). Every
//! other ability keeps the damage pipeline and its #444 target gate.
//!
//! The rule, from the 2009 data:
//!
//! - `type_id` is `ABILITY_TYPE_Heal` (597 Heal Focus, 1646 Health Heal,
//!   1218 Recuperation: their effects carry no `EF_Beneficial_Effect` bit,
//!   659 is flags 16), **or**
//! - at least one effect does something ([`effect_is_implemented`]) and
//!   every effect that does something carries `EF_Beneficial_Effect`.
//!
//! **Damage vetoes both.** An ability with an effect that deals damage
//! (`HealthDamage` or `FocusDamage` above zero) is never beneficial, whatever
//! its type: the seed has a Heal-typed attack (2228 `MS020_080818_CallTarget`,
//! effect 3091 `RangedPhysicalDamage`), and calling it beneficial would land
//! that damage on the caster or an ally and skip the #444 gate.
//!
//! The "at least one" keeps an ability whose effects do nothing out of the
//! beneficial path: an unimplemented attack must not start skipping the
//! #444 gate because its effect list is vacuously "all beneficial".

use std::collections::HashMap;

use super::{effect_is_implemented, AbilityDef, AbilityType, EffectDef, EF_BENEFICIAL_EFFECT};

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
        .collect();
    if doing.iter().any(|e| effect_deals_damage(e)) {
        return false;
    }
    def.type_id == AbilityType::Heal
        || (!doing.is_empty() && doing.iter().all(|e| e.flags & EF_BENEFICIAL_EFFECT != 0))
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
    /// beneficial bit). The type alone makes it beneficial.
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

    #[test]
    fn nothing_that_does_something_is_not_beneficial_unless_heal_typed() {
        let effects = HashMap::from([(3, effect(3, 1, &[], None))]);
        assert!(!ability_is_beneficial(
            &def(AbilityType::Buff, vec![3]),
            &effects
        ));
        assert!(!ability_is_beneficial(
            &def(AbilityType::DirectDamage, vec![]),
            &effects
        ));
        assert!(ability_is_beneficial(
            &def(AbilityType::Heal, vec![]),
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
}
