//! Live-DB guards on the stat rows the ability-mechanics generator writes
//! (AB-04, `tools/ability_mechanics/families/stat.py`, block
//! `ability-mechanics generated stat` in `effect_nvps.sql`).
//!
//! They load the real seed through the cell's own loaders, run each
//! effect's seeded `script_name` through the registry, and check the
//! routing the generator relies on (the buffs' abilities stay beneficial).
//! Every value is RECONSTRUCTION from the effect's own `effect_desc`, with
//! D-AB09's units.

use cimmeria_entity::abilities::{
    ability_is_beneficial, AF_TOGGLED, EFFECT_MONIKER_NVP, EFFECT_STANCE_MONIKER,
    EF_ALWAYS_PERSIST, REMOVE_BY_MONIKER_SCRIPT, REMOVE_MONIKER_NVP,
};
use cimmeria_entity::stats::{ACCURACY, COVER_DEFENSE, DEFENSE, MOVEMENT_SPEED_MOD};

use super::super::test_fixtures::make_mgr_with_target;
use super::super::{dispatch_by_name, EffectContext};
use crate::cell::spawner::{load_ability_defs, load_effect_defs};
use crate::test_support::require_db_or_skip;

/// `(effect_id, NVP name, value, pulse_duration)`, with the designer text.
const PACKET: [(i32, &str, &str, f32); 4] = [
    (700, "Accuracy", "200", 15.0),      // Aim "+200 Accuracy: 15 Seconds"
    (903, "Defense", "-100", 15.0),      // Call Target "-100 Defense: 15 Seconds"
    (1747, "CoverDefense", "100", 15.0), // Hunker Down "+100 Cover Defense: 15 seconds"
    (1962, "MovementSpeedMod", "50", 10.0), // Combat Sprint "User +50% Run Speed" (D-AB09: percent)
];

#[tokio::test]
async fn generated_stat_effects_carry_their_script_and_nvp_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, name, value, secs) in PACKET {
        let def = defs
            .get(&effect_id)
            .unwrap_or_else(|| panic!("effect {effect_id} is seeded"));
        assert_eq!(
            def.script_name.as_deref(),
            Some("TimedStat"),
            "effect {effect_id} script_name"
        );
        assert_eq!(
            def.params.get(name).map(String::as_str),
            Some(value),
            "effect {effect_id} {name}"
        );
        assert_eq!(
            (def.pulse_count, def.pulse_duration),
            (1, secs),
            "{effect_id}"
        );
    }
    // The deferred halves stay unbound: Leadership's regen (AB-05) and
    // Hunker Down's secondary half. Combat Sprint's penalty 2002 is bound
    // since AB-07 lands it on the user (`cell-combat`'s
    // `effect_routing_live_db`).
    for effect_id in [1211, 1746] {
        assert_eq!(
            defs[&effect_id].script_name, None,
            "{effect_id} stays unbound"
        );
    }
}

/// The seeded rows move the stat their text states, through the real script.
#[tokio::test]
async fn generated_stat_effects_move_their_stat_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (effect_id, stat, want) in [
        (700, ACCURACY, 200),
        (903, DEFENSE, -100),
        (1747, COVER_DEFENSE, 100),
        (1962, MOVEMENT_SPEED_MOD, 150),
    ] {
        let mut mgr = make_mgr_with_target();
        let def = &defs[&effect_id];
        let script = def.script_name.clone().expect("seeded script");
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: def,
            space_mgr: &mut mgr,
        };
        assert!(dispatch_by_name(&script, &mut ctx), "{script} registered");
        let cur = mgr.get_entity(1).unwrap().stats.get(stat).unwrap().cur;
        assert_eq!(cur, want, "effect {effect_id}");
        let entries = &mgr.get_entity(1).unwrap().stat_buffs.entries;
        assert_eq!(entries.len(), 1, "one ledger entry for {effect_id}");
    }
}

/// The routing the generator's scope rules promise: the buffs' abilities
/// are beneficial (so they land on the caster or an ally), Call Target is
/// not (so it lands on the hostile through `damage_apply`). Combat Sprint
/// stopped being beneficial when AB-07 bound its penalty half: both halves
/// land on its user through the per-effect routing instead.
#[tokio::test]
async fn generated_stat_abilities_route_as_the_generator_assumes_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("load_ability_defs");
    let effects = load_effect_defs(&pool).await.expect("load_effect_defs");
    // 637 Aim, 1454 Hunker Down.
    for id in [637, 1454] {
        assert!(
            ability_is_beneficial(&abilities[&id], &effects),
            "{id} must be beneficial"
        );
    }
    // 847 Call Target, and 1619 Combat Sprint (routed per effect).
    assert!(!ability_is_beneficial(&abilities[&847], &effects));
    assert!(!ability_is_beneficial(&abilities[&1619], &effects));
}

/// AB-08's held rows, with the designer text: the stances carry
/// `EffectMoniker EFFECT_Stance`, the passives do not, and every one is a
/// `pulse_duration = 0` `TimedStat` on a toggle or a passive.
#[tokio::test]
async fn held_stance_and_passive_rows_carry_their_nvps_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    let abilities = load_ability_defs(&pool).await.expect("load_ability_defs");
    for (effect_id, name, value, stance) in [
        (2003, "CoverDefense", "100", true), // 1642 "Cover Defense +100"
        (2004, "MentalResistance", "50", true), // 1642 "+50 (5%) Mental Resist buff"
        (2005, "Subtlety", "-100", true),    // 1642 "Subtlety -100 (10% increase to threat)"
        (922, "InterruptResistance", "250", true), // 859 "Target +250 Interrupt Resistance"
        (2645, "KineticResistance", "150", false), // 1731 passive "+15%" (D-AB09)
        (1741, "CoverAccuracy", "100", false), // 1450 passive "+100 CoverAccuracy"
        (4782, "Defense", "100", false),     // 1574 passive "Defense: +100"
    ] {
        let def = &defs[&effect_id];
        assert_eq!(def.script_name.as_deref(), Some("TimedStat"), "{effect_id}");
        assert_eq!(
            def.params.get(name).map(String::as_str),
            Some(value),
            "{effect_id} {name}"
        );
        assert_eq!(def.pulse_duration, 0.0, "{effect_id} is held");
        assert_eq!(
            def.params.get(EFFECT_MONIKER_NVP).map(String::as_str),
            stance.then_some("EFFECT_Stance"),
            "{effect_id} stance tag"
        );
        let ability = &abilities[&def.ability_id];
        let toggled = ability.flags & AF_TOGGLED != 0;
        let persists = def.flags & EF_ALWAYS_PERSIST != 0;
        assert!(toggled != persists, "{effect_id}: a toggle or a passive");
    }
    let removal = &defs[&4294];
    assert_eq!(
        removal.script_name.as_deref(),
        Some(REMOVE_BY_MONIKER_SCRIPT)
    );
    assert_eq!(
        removal.params.get(REMOVE_MONIKER_NVP).map(String::as_str),
        Some("EFFECT_Stance")
    );
}

/// The removal can only ever reach stance entries: no seeded ability lists
/// `EFFECT_Stance` among its `moniker_ids`, so an entry carries it only
/// through its effect's `EffectMoniker` row. Read from the table itself:
/// the ability loader does not load `moniker_ids`.
#[tokio::test]
async fn no_ability_carries_the_stance_moniker_live_db() {
    let pool = require_db_or_skip!();
    let with = |moniker: i64| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i32>(
                "SELECT ability_id FROM resources.abilities \
                 WHERE $1 = ANY(moniker_ids::bigint[]) ORDER BY ability_id",
            )
            .bind(moniker)
            .fetch_all(&pool)
            .await
            .expect("moniker query")
        }
    };
    // The query can see monikers at all: the shared group is on hundreds.
    assert!(
        with(1_470_900_795).await.len() > 100,
        "moniker_ids must be readable"
    );
    let carriers = with(EFFECT_STANCE_MONIKER).await;
    assert!(carriers.is_empty(), "{carriers:?}");
}

/// The stances stay beneficial (their flags-0 removal half included), so a
/// press lands on the caster whatever is selected.
#[tokio::test]
async fn seeded_stances_are_beneficial_live_db() {
    let pool = require_db_or_skip!();
    let abilities = load_ability_defs(&pool).await.expect("load_ability_defs");
    let effects = load_effect_defs(&pool).await.expect("load_effect_defs");
    // 1642 Soldier, 1458 Ranged Specialist, 859 Concentration (with 4294),
    // 857 Leading the Target (with 920), 714 Mobility, 2067 Warrior's.
    for id in [1642, 1458, 859, 857, 714, 2067] {
        assert!(ability_is_beneficial(&abilities[&id], &effects), "{id}");
        assert!(abilities[&id].flags & AF_TOGGLED != 0, "{id} toggles");
    }
}
