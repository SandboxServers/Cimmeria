//! A weapon-granted ability that the player is then granted for good keeps
//! no weapon-granted tag (CS-07 review N1).

use std::collections::HashSet;

use super::AbilityManager;

const HEAL_FOCUS: i32 = 597;
const PISTOL_RANGED: i32 = 579;

/// **Regression guard.** 597 Heal Focus is a USE binding on 158 items and a
/// tutorial grant. Granted by the active weapon first, then by content (or a
/// trainer) through `add_ability`, it must survive the next weapon change.
/// Without the untag in `add_ability` the swap revokes it.
#[test]
fn a_later_grant_takes_a_weapon_granted_ability_off_the_weapon() {
    let mut mgr = AbilityManager::with_abilities(&[]);
    mgr.swap_weapon_granted_abilities([HEAL_FOCUS, PISTOL_RANGED].into_iter().collect());
    assert!(mgr.weapon_granted_ability_ids().contains(&HEAL_FOCUS));

    mgr.add_ability(HEAL_FOCUS);

    let (removed, _) = mgr.swap_weapon_granted_abilities(HashSet::new());
    assert_eq!(removed, vec![PISTOL_RANGED], "only the weapon's own leave");
    assert!(
        mgr.has_ability(HEAL_FOCUS),
        "the granted 597 survives the weapon change"
    );
}
