//! AB-01 against the real seed: the starter heals load as beneficial, and
//! weapon attacks and the Heal-typed attack do not.
//!
//! `beneficial.rs` builds its defs by hand; this guard takes the loaders'
//! output, so it also fails if `load_ability_defs` stops reading `type_id`
//! (597's effect 659 carries no `EF_Beneficial_Effect` bit: the Heal type lets
//! its `HealFocus` script stand in for the bit; empty, debuff or damaging Heal
//! abilities stay non-beneficial).

use cimmeria_entity::abilities::{ability_is_beneficial, AbilityType, TARGET_SELF};

use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn seeded_starter_heals_load_as_beneficial_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("ability defs load");
    let effects = load_effect_defs(&pool).await.expect("effect defs load");

    for id in [597, 1646, 1218] {
        let def = abilities
            .get(&id)
            .unwrap_or_else(|| panic!("{id} is seeded"));
        assert_eq!(def.type_id, AbilityType::Heal, "{id} is ABILITY_TYPE_Heal");
        assert!(
            ability_is_beneficial(def, &effects),
            "{id} must be beneficial"
        );
    }
    assert_eq!(
        abilities[&597].target_type_id, TARGET_SELF,
        "597 Heal Focus is a Self ability"
    );
    assert_eq!(
        effects[&659].flags & cimmeria_entity::abilities::EF_BENEFICIAL_EFFECT,
        0,
        "659 has no beneficial bit: on Heal-typed 597 its heal script stands in for it"
    );

    // 592 Pistol Shot and 559 Auto Attack are attacks; 2228 is Heal-typed but
    // deals damage. All three keep the damage pipeline and the #444 gate.
    for id in [592, 559, 2228] {
        let def = abilities
            .get(&id)
            .unwrap_or_else(|| panic!("{id} is seeded"));
        assert!(
            !ability_is_beneficial(def, &effects),
            "{id} must not be beneficial"
        );
    }
    assert_eq!(abilities[&2228].type_id, AbilityType::Heal);

    // Server-authority review S1: Heal-typed debuffs and crowd control are
    // not beneficial. Reverting to "the Heal type wins" fails here.
    for id in [1874, 1937, 1988, 2090, 2154, 3253] {
        let def = abilities
            .get(&id)
            .unwrap_or_else(|| panic!("{id} is seeded"));
        assert_eq!(def.type_id, AbilityType::Heal, "{id} is Heal-typed");
        assert!(
            !ability_is_beneficial(def, &effects),
            "{id} is a Heal-typed debuff and must not be beneficial"
        );
    }
}
