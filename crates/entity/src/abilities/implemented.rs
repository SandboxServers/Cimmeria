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
/// or an effect script. An effect missing from `effects` does nothing: the
/// damage pipeline skips an effect id it cannot resolve.
pub fn effect_is_implemented(effect: Option<&EffectDef>) -> bool {
    effect.is_some_and(|e| {
        e.param_i32("HealthDamage") > 0 || e.param_i32("FocusDamage") > 0 || e.script_name.is_some()
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
            min_range: 0,
            max_range: 0,
            target_type_id: 2,
            effect_ids,
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id,
            velocity: 100.0,
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

    #[test]
    fn zero_or_garbage_damage_values_are_not_damage() {
        let effects = HashMap::from([(
            1,
            effect(1, &[("HealthDamage", "0"), ("FocusDamage", "x")], None),
        )]);
        assert!(ability_is_unimplemented(&def(None, vec![1]), &effects));
    }
}
