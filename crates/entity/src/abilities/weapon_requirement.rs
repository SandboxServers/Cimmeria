//! The player weapon-moniker requirement (Class Start v6 OD-CS11).
//!
//! `resources.abilities.item_monikers` lists the weapon monikers an ability
//! needs: when the list is non-empty, the active bandolier item's
//! `moniker_ids` must contain at least one of them, or the launch is refused
//! with `CONDITION_FEEDBACK_WrongWeaponType` (63). The any-match rule is
//! python's `SGWPlayer.hasItemMoniker`; python applied it only to
//! `TargetTarget` abilities, after the cooldown check
//! (`deprecated/python/cell/AbilityManager.py:528-545`), while OD-CS11 applies
//! it to every target type ahead of the cooldown. This is the pure rule; the
//! launch gate that applies it to player casts is `cell-combat`'s
//! `use_ability::weapon_requirement`, which records the differences.

use super::AbilityDef;

impl AbilityDef {
    /// Whether a player's cast of this ability needs a particular weapon.
    pub fn requires_weapon(&self) -> bool {
        !self.item_monikers.is_empty()
    }

    /// Whether a weapon carrying `weapon_monikers` meets the requirement.
    /// `None` is an empty active slot: it meets only an ability with no
    /// requirement. Any one shared moniker is enough.
    pub fn weapon_satisfies(&self, weapon_monikers: Option<&[i64]>) -> bool {
        if !self.requires_weapon() {
            return true;
        }
        weapon_monikers.is_some_and(|have| self.item_monikers.iter().any(|m| have.contains(m)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_PISTOL: i64 = 2_445_422_768;
    const ITEM_AUTOMATIC_WEAPON: i64 = 3_175_425_141;
    const ITEM_SMG: i64 = 728_213_066;
    const ITEM_LIGHT_MG: i64 = 1_115_110_575;
    const CATEGORY_WEAPONS: i64 = 3_901_383_057;

    fn def(item_monikers: Vec<i64>) -> AbilityDef {
        AbilityDef {
            ability_id: 598,
            name: "Quick Burst".into(),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            item_monikers,
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        }
    }

    #[test]
    fn no_requirement_is_met_by_anything_including_no_weapon() {
        let d = def(vec![]);
        assert!(!d.requires_weapon());
        assert!(d.weapon_satisfies(None));
        assert!(d.weapon_satisfies(Some(&[ITEM_LIGHT_MG])));
    }

    /// 598 + item 21 (SGHC 6: Automatic_Weapon, SMG) passes; 598 + item
    /// 3260 (SK37 LMG: LightMG only) fails.
    #[test]
    fn any_shared_moniker_meets_the_requirement() {
        let d = def(vec![ITEM_AUTOMATIC_WEAPON]);
        assert!(d.weapon_satisfies(Some(&[ITEM_AUTOMATIC_WEAPON, ITEM_SMG, CATEGORY_WEAPONS])));
        assert!(!d.weapon_satisfies(Some(&[ITEM_LIGHT_MG, CATEGORY_WEAPONS])));
    }

    #[test]
    fn one_of_several_required_monikers_is_enough() {
        let d = def(vec![ITEM_AUTOMATIC_WEAPON, ITEM_SMG]);
        assert!(d.weapon_satisfies(Some(&[ITEM_SMG])));
    }

    #[test]
    fn a_requirement_with_no_weapon_fails() {
        let d = def(vec![ITEM_PISTOL]);
        assert!(!d.weapon_satisfies(None));
        assert!(!d.weapon_satisfies(Some(&[])));
    }
}
