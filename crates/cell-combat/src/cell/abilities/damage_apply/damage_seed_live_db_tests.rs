//! Live-DB guards on the damage rows the ability-mechanics generator writes
//! (AB-03, `tools/ability_mechanics/effect_nvps_from_desc.py`, block
//! `ability-mechanics generated damage` in `effect_nvps.sql`).
//!
//! Every value is RECONSTRUCTION from the effect's own `effect_desc`. Drop
//! the block (or a row) and the NVP assertions fail; revert the per-effect
//! resolution (B-21) and the Point Blank Shot hit reports one HEALTH entry.

use cimmeria_entity::abilities::RC_MISS;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::apply_damage_to_target;
use super::single_damage_path_tests::seq_rolling;
use super::tests::{drain, make_mgr_player_vs_npc};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::mercury::method_idx;
use crate::test_support::require_db_or_skip;

/// `(ability, effect, FocusDamage, HealthDamage)` with the designer text.
const GENERATED: [(i32, i32, &str, &str); 6] = [
    (598, 660, "200", "20"),   // Quick Burst "Single Target\n-200F / -20H"
    (717, 744, "100", "10"),   // Snare Shot "Single Target\n-100F / -10H"
    (856, 919, "100", "10"),   // Takedown "Single Target Melee Damage\n-100F / -10H"
    (1879, 2393, "200", "20"), // Point Blank Shot "Single Target\n-200F / -20H"
    (1879, 2394, "150", "30"), // its DoT "DOT: -150F -30H (8 Ticks)", per tick
    (2419, 3511, "500", "50"), // Frag Grenade "Frag Grenade Damage:\n- 500 F\n- 50 H"
];

/// Conditional and sequenced variants the generator must leave without a
/// damage row: Execution vs low Focus (1604), Red Mist vs low Focus (1609),
/// EMP Grenade vs (non-)mechanical (4200, 4202), Surprise Attack from the
/// flank and rear (1559, 1560).
const LEFT_OUT: [i32; 6] = [1604, 1609, 4200, 4202, 1559, 1560];

#[tokio::test]
async fn generated_damage_effects_carry_their_numbers_live_db() {
    let pool = require_db_or_skip!();
    let effects = load_effect_defs(&pool).await.expect("effect defs load");
    let abilities = load_ability_defs(&pool).await.expect("ability defs load");

    for (ability_id, effect_id, focus, health) in GENERATED {
        assert!(
            abilities[&ability_id].effect_ids.contains(&effect_id),
            "{effect_id} belongs to {ability_id}"
        );
        let def = &effects[&effect_id];
        assert_eq!(
            def.params.get("FocusDamage").map(String::as_str),
            Some(focus),
            "effect {effect_id} FocusDamage"
        );
        assert_eq!(
            def.params.get("HealthDamage").map(String::as_str),
            Some(health),
            "effect {effect_id} HealthDamage"
        );
        assert_eq!(
            def.script_name, None,
            "effect {effect_id} feeds the NVP path"
        );
    }
    assert_eq!(effects[&2394].pulse_count, 8, "the DoT row pulses 8 times");
    for effect_id in LEFT_OUT {
        let params = &effects[&effect_id].params;
        assert!(
            !params.contains_key("FocusDamage") && !params.contains_key("HealthDamage"),
            "effect {effect_id} is a conditional variant and must stay unparsed"
        );
    }
}

/// Point Blank Shot (1879) on the seeded rows: the direct hit (2393) and
/// the DoT's first tick (2394) both land, each with its own HEALTH entry in
/// `onEffectResults`, and the DoT registers its remaining pulses. Before
/// AB-03 the hit dealt 0 (no NVPs); with the NVPs but the old "last
/// positive value" rule it reported one entry.
#[tokio::test]
async fn point_blank_shot_lands_its_hit_and_its_dot_live_db() {
    let pool = require_db_or_skip!();
    let mut mgr = make_mgr_player_vs_npc();
    mgr.effect_defs = load_effect_defs(&pool).await.expect("effect defs load");
    mgr.ability_defs = load_ability_defs(&pool).await.expect("ability defs load");
    let ability = mgr.ability_defs.get(&1879).cloned();
    let npc = mgr.get_entity_mut(2).unwrap();
    for stat in [HEALTH, FOCUS] {
        let s = npc.stats.get_mut(stat).unwrap();
        s.update(0, 5000, 5000);
        s.clear_dirty();
    }
    let seq = seq_rolling(&mgr, (1, 2), 1879, false);
    let (tx, mut rx) = mpsc::channel(256);

    apply_damage_to_target(1, 2, 1879, &ability, seq, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let args = msgs
        .iter()
        .find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if *method_index == method_idx::ON_EFFECT_RESULTS => Some(args.clone()),
            _ => None,
        })
        .expect("onEffectResults to the attacker");
    assert_ne!(args[16], RC_MISS, "the seed rolls a hit");
    // Four i32 ids, the result code, then the list's u32 count.
    let entries = u32::from_le_bytes(args[17..21].try_into().unwrap());
    assert_eq!(entries, 2, "one HEALTH entry per damage effect");
    let npc = mgr.get_entity(2).unwrap();
    assert!(npc.stats.get(HEALTH).unwrap().cur < 5000);
    assert!(npc.stats.get(FOCUS).unwrap().cur < 5000);
    assert_eq!(npc.active_effects.len(), 1, "the DoT ticks on");
}
