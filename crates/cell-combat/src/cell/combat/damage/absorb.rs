//! Absorb shields in the damage path: the drain every damage seam shares.
//!
//! A shield is capacity in the `absorb*` stats (`alias.xml`: "how much
//! physical damage to absorb"). The ability shields put it there through
//! the timed effect ledger (`AbsorbShield`, ability-mechanics AB-10), which
//! owns each shield's share; the damage seams drain the stats here and then
//! call `SpaceManager::settle_absorb_shields`, which charges the drain to
//! the shields and takes off the empty ones.
//!
//! **Shields stand in front of Focus.** SGW's Focus is itself the outer
//! damage pool, so a shield that only caught Health damage would sit behind
//! it and do nothing until Focus was gone. The drain applies to Focus and
//! Health damage alike, in the pool's own points, Focus first wherever a
//! seam deals both. Python's `DamageCalc` (which this pipeline follows
//! otherwise) shielded Health only; that left the absorb stats inert for
//! every authored shield, so AB-10 departs from it.
//!
//! Three seams drain:
//!
//! - the pipeline (`calculate_damage_penetrating`): NVP damage on a hit and
//!   a DoT pulse with no script, after armour;
//! - [`absorb_damage_nvps`]: a damage script's `FocusDamage` /
//!   `HealthDamage`, before the script writes the pools (the scripts write
//!   Focus and Health directly, so they never pass the pipeline).

use cimmeria_entity::abilities::{EffectDef, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC};
use cimmeria_entity::stats::{
    StatList, ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM, ABSORB_HAZMAT,
    ABSORB_HAZMAT_ENERGY, ABSORB_HAZMAT_ITEM, ABSORB_PHYSICAL, ABSORB_PHYSICAL_ENERGY,
    ABSORB_PHYSICAL_ITEM, ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM,
    ABSORB_UNTYPED, ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM,
};

/// The `absorb*` stats that soak `damage_type`, the elemental pool first,
/// then its Energy- and item-charged siblings. Untyped damage drains only
/// the untyped pools, and typed damage never the untyped ones.
fn absorb_pools(damage_type: i8) -> &'static [i32] {
    match damage_type {
        DT_PHYSICAL => &[
            ABSORB_PHYSICAL,
            ABSORB_PHYSICAL_ENERGY,
            ABSORB_PHYSICAL_ITEM,
        ],
        DT_ENERGY => &[ABSORB_ENERGY, ABSORB_ENERGY_ENERGY, ABSORB_ENERGY_ITEM],
        DT_HAZMAT => &[ABSORB_HAZMAT, ABSORB_HAZMAT_ENERGY, ABSORB_HAZMAT_ITEM],
        DT_PSIONIC => &[ABSORB_PSIONIC, ABSORB_PSIONIC_ENERGY, ABSORB_PSIONIC_ITEM],
        _ => &[ABSORB_UNTYPED, ABSORB_UNTYPED_ENERGY, ABSORB_UNTYPED_ITEM],
    }
}

/// Drain the pools matching `damage_type` by up to `incoming`. Returns
/// `(damage left after absorption, total absorbed)`.
pub(crate) fn drain_absorption_pools(
    defender: &mut StatList,
    damage_type: i8,
    incoming: i32,
) -> (i32, i32) {
    let mut remaining = incoming.max(0);
    let mut absorbed_total = 0;
    for &pool_id in absorb_pools(damage_type) {
        if remaining == 0 {
            break;
        }
        let Some(pool) = defender.get_mut(pool_id) else {
            continue;
        };
        let available = pool.cur.max(0);
        if available == 0 {
            continue;
        }
        let drain = remaining.min(available);
        // `change(-drain)` returns the actual delta (clamped by stat min/max)
        let actual = pool.change(-drain).unsigned_abs() as i32;
        absorbed_total += actual;
        remaining -= actual;
    }
    (remaining, absorbed_total)
}

/// Run a damage script's `FocusDamage` then `HealthDamage` NVPs through the
/// target's shields, rewriting each to what gets through. Returns the total
/// absorbed. `effect` is the caller's copy (already scaled for cover and
/// ammo), never the shared def.
pub(crate) fn absorb_damage_nvps(
    defender: &mut StatList,
    effect: &mut EffectDef,
    damage_type: i8,
) -> i32 {
    let mut absorbed = 0;
    for name in ["FocusDamage", "HealthDamage"] {
        if !effect.params.contains_key(name) {
            continue;
        }
        let asked = effect.param_i32(name);
        if asked <= 0 {
            continue;
        }
        let (left, took) = drain_absorption_pools(defender, damage_type, asked);
        if took > 0 {
            effect.params.insert(name.to_string(), left.to_string());
            absorbed += took;
        }
    }
    absorbed
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::DT_ENERGY;

    fn defender(physical: i32) -> StatList {
        let mut s = StatList::new();
        s.get_mut(ABSORB_PHYSICAL)
            .unwrap()
            .update(0, physical, 1000);
        s
    }

    fn script_effect(focus: i32, health: i32) -> EffectDef {
        let mut e = EffectDef::default();
        e.params.insert("FocusDamage".into(), focus.to_string());
        e.params.insert("HealthDamage".into(), health.to_string());
        e
    }

    /// Focus first: a 160-point shield eats Pistol Shot's 150 Focus and 10
    /// of its 15 Health.
    #[test]
    fn a_scripts_damage_drains_the_shield_focus_first() {
        let mut stats = defender(160);
        let mut e = script_effect(150, 15);
        assert_eq!(absorb_damage_nvps(&mut stats, &mut e, DT_PHYSICAL), 160);
        assert_eq!(e.param_i32("FocusDamage"), 0);
        assert_eq!(e.param_i32("HealthDamage"), 5);
        assert_eq!(stats.get(ABSORB_PHYSICAL).unwrap().cur, 0);
    }

    #[test]
    fn a_shield_of_another_type_lets_the_script_through() {
        let mut stats = defender(500);
        let mut e = script_effect(150, 15);
        assert_eq!(absorb_damage_nvps(&mut stats, &mut e, DT_ENERGY), 0);
        assert_eq!(e.param_i32("FocusDamage"), 150);
        assert_eq!(stats.get(ABSORB_PHYSICAL).unwrap().cur, 500);
    }
}
