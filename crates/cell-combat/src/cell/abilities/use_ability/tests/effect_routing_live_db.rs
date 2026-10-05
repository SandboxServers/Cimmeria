//! AB-07 against the real seed: the rows the generator binds now that the
//! server routes user halves and beneficial radius halves, and Morale Boost
//! and Combat Sprint fired from them.
//!
//! The rows are RECONSTRUCTION (each quotes its effect text in
//! `effect_nvps.sql`). Reverting the generator's routing rules leaves 1215
//! and 2002 unbound, and the casts then miss the allies and the penalty.

use cimmeria_entity::abilities::ability_is_beneficial;
use cimmeria_entity::stats::{ACCURACY, FOCUS, MOVEMENT_SPEED_MOD};

use super::duel_gate::{A, B, MOB};
use super::effect_routing::routing_mgr;
use super::*;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn routed_effects_carry_their_rows_and_land_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("ability defs load");
    let effects = load_effect_defs(&pool).await.expect("effect defs load");

    let row = |id: i32| {
        effects
            .get(&id)
            .unwrap_or_else(|| panic!("effect {id} is seeded"))
    };
    for (id, script, nvp, value) in [
        (1215, "HealFocus", "HealPercentage", "35.00"), // Morale Boost's radius heal
        (2002, "TimedStat", "Accuracy", "-100"),        // Combat Sprint's penalty
        (948, "TimedStat", "Defense", "-200"),          // Forward Observer, ground radius
        (4779, "TimedStat", "Response", "100"),         // TimeShift, a pure Self ability
    ] {
        let e = row(id);
        assert_eq!(e.script_name.as_deref(), Some(script), "effect {id} script");
        assert_eq!(
            e.params.get(nvp).map(String::as_str),
            Some(value),
            "effect {id} {nvp}"
        );
    }
    // Binding the penalty takes Combat Sprint off the beneficial path; the
    // routing is what keeps both halves on its user.
    assert!(!ability_is_beneficial(&abilities[&1619], &effects));
    assert!(ability_is_beneficial(&abilities[&869], &effects));

    // Fire both from the seeded rows on the hand fixture's entities.
    let mut mgr = routing_mgr();
    for id in [869, 1619] {
        // The seeded warmups (869's 2 s) are the warmup tests' business.
        let def = AbilityDef {
            warmup: 0.0,
            ..abilities[&id].clone()
        };
        for eid in &def.effect_ids {
            mgr.effect_defs.insert(*eid, row(*eid).clone());
        }
        mgr.ability_defs.insert(id, def);
    }
    let (tx, _rx) = mpsc::channel(256);

    assert!(
        handle_use_ability(A, 869, 0, &tx, &mut mgr).await,
        "Morale Boost commits"
    );
    let focus =
        |mgr: &SpaceManager, eid| mgr.get_entity(eid).unwrap().stats.get(FOCUS).unwrap().cur;
    assert_eq!(focus(&mgr, A), 350, "the caster's 35%");
    assert_eq!(focus(&mgr, B), 350, "the ally's 35%, from 1215");
    assert_eq!(focus(&mgr, MOB), 0, "never the mob");

    assert!(
        handle_use_ability(A, 1619, MOB as i32, &tx, &mut mgr).await,
        "Combat Sprint at the mob commits"
    );
    let stat = |eid, id| mgr.get_entity(eid).unwrap().stats.get(id).unwrap().cur;
    assert_eq!(stat(A, MOVEMENT_SPEED_MOD), 150, "1962 on the caster");
    assert_eq!(stat(A, ACCURACY), -100, "2002 on the caster");
    assert_eq!(stat(MOB, ACCURACY), 0, "the mob keeps its Accuracy");
}
