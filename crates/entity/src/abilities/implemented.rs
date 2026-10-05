//! Whether an ability does anything a player can see when it is cast.
//!
//! A cast shows up in exactly three ways on this server: an effect deals
//! damage from its `HealthDamage` / `FocusDamage` NVPs (`damage_apply`), an
//! effect runs a registered script (`script_name`), or the ability's event
//! set plays a Kismet sequence (`onSequence`). An ability with none of the
//! three resolves as an empty hit with no animation: to its user, nothing
//! happened. Many 2009 pet-kit rows are like that (pets PT-11: 1654 and the
//! Lo'taur heals 1653, 3326-3329).
//!
//! One predicate, shared by the pet command gate (CM 88 refuses such an
//! order before casting, so the owner's click gets feedback), the pet AI's
//! ability pick (a pet never "attacks" with one) and the NA43 animation
//! linter's allowlist guard (an allowlisted ability must stay like this).

use std::collections::HashMap;

use super::{AbilityDef, EffectDef};

/// Whether `effect` does something when it resolves: positive damage values
/// or a named effect script. An effect missing from `effects` does nothing:
/// the damage pipeline skips an effect id it cannot resolve. A blank
/// `script_name` names no script (seed effect 2907 on ability 2134 carries
/// `''`), so it does not count.
pub fn effect_is_implemented(effect: Option<&EffectDef>) -> bool {
    effect_has_mechanic(effect, |_| true)
}

/// [`effect_is_implemented`] with a say on the script: a named script counts
/// only when `script_resolves(name)`. The cell passes its installed registry,
/// so a name no script answers (`Reload`, a typo) is not a mechanic.
pub fn effect_has_mechanic(
    effect: Option<&EffectDef>,
    script_resolves: impl Fn(&str) -> bool,
) -> bool {
    effect.is_some_and(|e| {
        e.param_i32("HealthDamage") > 0
            || e.param_i32("FocusDamage") > 0
            || e.script_name
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty() && script_resolves(s))
    })
}

/// `true` when casting `def` would have no visible result: no event set, and
/// no effect that deals damage or runs a script.
pub fn ability_is_unimplemented(def: &AbilityDef, effects: &HashMap<i32, EffectDef>) -> bool {
    def.event_set_id.is_none()
        && !def
            .effect_ids
            .iter()
            .any(|id| effect_is_implemented(effects.get(id)))
}

/// `true` when at least one of `def`'s effects does something when it
/// resolves ([`effect_is_implemented`]): a damage NVP or an effect script.
///
/// Stricter than `!`[`ability_is_unimplemented`]: an event set alone does not
/// count, because an animation is not a mechanic (ability-mechanics D-AB10).
/// The player's launch gate (`cell-combat`'s `use_ability/no_mechanics.rs`)
/// adds the bindings that live outside the effect rows (pet summons,
/// deployables, ammo toggles, weapon shots).
///
/// Heal and stat NVPs (`HealAmount`, `HealPercentage`, the `StatBuff` stat
/// names) count only through the script that reads them: no code reads them
/// without one, and every generator family that writes them also binds the
/// script (AB-02, AB-04). So the count grows on its own as the seed fills in.
///
/// A script counts only when `script_resolves(name)` ([`effect_has_mechanic`]):
/// the cell passes its installed registry.
pub fn ability_effects_have_mechanics(
    def: &AbilityDef,
    effects: &HashMap<i32, EffectDef>,
    script_resolves: impl Fn(&str) -> bool,
) -> bool {
    def.effect_ids
        .iter()
        .any(|id| effect_has_mechanic(effects.get(id), &script_resolves))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(event_set_id: Option<i32>, effect_ids: Vec<i32>) -> AbilityDef {
        AbilityDef {
            ability_id: 1654,
            name: "Prime: Focus Degeneration".to_string(),
            cooldown: 4.0,
            warmup: 0.0,
            flags: 84,
            is_ranged: true,
            min_range: 0.0,
            max_range: 0.0,
            target_type_id: 2,
            effect_ids,
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id,
            velocity: 100.0,
            type_id: Default::default(),
            passive: false,
        }
    }

    fn effect(id: i32, params: &[(&str, &str)], script: Option<&str>) -> EffectDef {
        EffectDef {
            effect_id: id,
            script_name: script.map(str::to_string),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            ..EffectDef::default()
        }
    }

    #[test]
    fn an_unscripted_undamaging_silent_ability_is_unimplemented() {
        let effects = HashMap::from([(4086, effect(4086, &[], None))]);
        assert!(ability_is_unimplemented(&def(None, vec![4086]), &effects));
        // No effects at all, or an effect that did not load: still nothing.
        assert!(ability_is_unimplemented(&def(None, vec![]), &effects));
        assert!(ability_is_unimplemented(&def(None, vec![9999]), &effects));
    }

    #[test]
    fn any_one_visible_result_makes_it_implemented() {
        let silent = effect(1, &[], None);
        let damage = effect(2, &[("HealthDamage", "25")], None);
        let focus = effect(3, &[("FocusDamage", "250")], None);
        let script = effect(4, &[], Some("HealHealth"));
        let effects = HashMap::from([(1, silent), (2, damage), (3, focus), (4, script)]);
        assert!(!ability_is_unimplemented(&def(Some(3), vec![1]), &effects));
        assert!(!ability_is_unimplemented(&def(None, vec![1, 2]), &effects));
        assert!(!ability_is_unimplemented(&def(None, vec![3]), &effects));
        assert!(!ability_is_unimplemented(&def(None, vec![4]), &effects));
    }

    /// D-AB10: an event set alone is not a mechanic. The same ability that
    /// `ability_is_unimplemented` calls implemented (it animates) has no
    /// mechanics; any effect that deals damage or runs a script gives it some.
    #[test]
    fn an_animation_alone_is_not_a_mechanic() {
        let silent = effect(1, &[("HealAmount", "30"), ("Coordination", "5")], None);
        let damage = effect(2, &[("HealthDamage", "25")], None);
        let script = effect(3, &[], Some("HealHealth"));
        let effects = HashMap::from([(1, silent), (2, damage), (3, script)]);
        let any = |_: &str| true;

        let animated = def(Some(3), vec![1]);
        assert!(!ability_is_unimplemented(&animated, &effects));
        assert!(
            !ability_effects_have_mechanics(&animated, &effects, any),
            "an event set and an unscripted heal or stat NVP do nothing"
        );
        assert!(!ability_effects_have_mechanics(
            &def(Some(3), vec![]),
            &effects,
            any
        ));
        assert!(!ability_effects_have_mechanics(
            &def(None, vec![9999]),
            &effects,
            any
        ));
        assert!(ability_effects_have_mechanics(
            &def(None, vec![1, 2]),
            &effects,
            any
        ));
        assert!(ability_effects_have_mechanics(
            &def(None, vec![3]),
            &effects,
            any
        ));
    }

    /// Seed effect 2907 (ability 2134) carries
    /// `script_name = ''`, which no script answers. A blank name is not a
    /// script for either predicate, and a name the registry does not answer
    /// (`Reload`, effect 658) is not a mechanic for the launch gate.
    #[test]
    fn a_blank_or_unregistered_script_name_is_not_a_script() {
        let blank = effect(2907, &[], Some(""));
        let spaces = effect(2908, &[], Some("  "));
        let reload = effect(658, &[], Some("Reload"));
        let heal = effect(659, &[], Some("HealFocus"));
        let effects = HashMap::from([(2907, blank), (2908, spaces), (658, reload), (659, heal)]);
        let registered = |s: &str| s == "HealFocus";

        assert!(!effect_is_implemented(effects.get(&2907)));
        assert!(!effect_is_implemented(effects.get(&2908)));
        assert!(ability_is_unimplemented(
            &def(None, vec![2907, 2908]),
            &effects
        ));
        assert!(!ability_effects_have_mechanics(
            &def(None, vec![2907]),
            &effects,
            |_| true
        ));

        // A named but unregistered script: implemented for the pet gate
        // (it names a script), not a mechanic for the player gate.
        assert!(effect_is_implemented(effects.get(&658)));
        assert!(!ability_effects_have_mechanics(
            &def(None, vec![658]),
            &effects,
            registered
        ));
        assert!(ability_effects_have_mechanics(
            &def(None, vec![659]),
            &effects,
            registered
        ));
    }

    #[test]
    fn zero_or_garbage_damage_values_are_not_damage() {
        let effects = HashMap::from([(
            1,
            effect(1, &[("HealthDamage", "0"), ("FocusDamage", "x")], None),
        )]);
        assert!(ability_is_unimplemented(&def(None, vec![1]), &effects));
    }
}
