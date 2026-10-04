//! Held entries (AB-08): toggles switch per press and never stack, stances
//! replace each other by `EFFECT_Stance` and never touch other buffs, held
//! entries land only on their invoker, and passives hold.

use cimmeria_entity::abilities::{
    AbilityDef, AbilityType, EffectDef, AF_TOGGLED, EFFECT_STANCE_MONIKER, EF_ALWAYS_PERSIST,
};
use cimmeria_entity::stats::{ACCURACY, COVER_DEFENSE, MENTAL_RES, SUBTLETY};
use tracing::Level;

use super::*;
use crate::cell::effects::registry;
use crate::cell::effects::test_fixtures::make_mgr_with_target;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const PLAYER: u32 = 1;
const OTHER: u32 = 2;
/// The ability moniker most combat abilities share: never a removal key.
const SHARED_MONIKER: i64 = 1_470_900_795;

const SOLDIER: i32 = 1642;
const RANGED_SPECIALIST: i32 = 1458;
const CONCENTRATION: i32 = 859;
const AIM: i32 = 637;

fn ability(id: i32, flags: u32, effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: format!("ability {id}"),
        cooldown: 2.0,
        warmup: 0.0,
        flags,
        is_ranged: false,
        min_range: 0.0,
        max_range: 0.0,
        target_type_id: 1,
        effect_ids,
        moniker_ids: vec![SHARED_MONIKER, 3_212_632_871],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: AbilityType::Buff,
        passive: false,
    }
}

fn held(id: i32, ability_id: i32, flags: u32, nvps: &[(&str, &str)]) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id,
        flags,
        pulse_count: 1,
        pulse_duration: 0.0,
        script_name: Some("TimedStat".to_string()),
        params: nvps
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..EffectDef::default()
    }
}

fn remove_stance(id: i32, ability_id: i32) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id,
        pulse_count: 1,
        script_name: Some("RemoveByMoniker".to_string()),
        params: [("RemoveMoniker".to_string(), "EFFECT_Stance".to_string())].into(),
        ..EffectDef::default()
    }
}

/// 1642 Stance: Soldier as the generator writes it (effect order 2005,
/// 2004, 2003), 1458 Ranged Specialist (1749), 859 Concentration (922 and
/// its "Remove Effect of moniker EFFECT_Stance" 4294), and Aim (700, a
/// timed buff on the shared ability moniker).
fn world() -> SpaceManager {
    let mut mgr = make_mgr_with_target();
    mgr.create_entity(OTHER, "W", [0.0; 3], [0.0; 3]).unwrap();
    let stance = "EFFECT_Stance";
    let effects = [
        held(
            2005,
            SOLDIER,
            533,
            &[("Subtlety", "-100"), ("EffectMoniker", stance)],
        ),
        held(
            2004,
            SOLDIER,
            533,
            &[("MentalResistance", "50"), ("EffectMoniker", stance)],
        ),
        held(
            2003,
            SOLDIER,
            21,
            &[("CoverDefense", "100"), ("EffectMoniker", stance)],
        ),
        held(
            1749,
            RANGED_SPECIALIST,
            21,
            &[("Accuracy", "100"), ("EffectMoniker", stance)],
        ),
        held(
            922,
            CONCENTRATION,
            85,
            &[("InterruptResistance", "250"), ("EffectMoniker", stance)],
        ),
        remove_stance(4294, CONCENTRATION),
    ];
    for e in effects {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    let mut aim = held(700, AIM, 21, &[("Accuracy", "200")]);
    aim.pulse_duration = 15.0;
    mgr.effect_defs.insert(700, aim);
    mgr.ability_defs.insert(
        SOLDIER,
        ability(SOLDIER, AF_TOGGLED, vec![2005, 2004, 2003]),
    );
    mgr.ability_defs.insert(
        RANGED_SPECIALIST,
        ability(RANGED_SPECIALIST, AF_TOGGLED | 16 | 512, vec![1749]),
    );
    mgr.ability_defs.insert(
        CONCENTRATION,
        ability(CONCENTRATION, AF_TOGGLED | 512, vec![922, 4294]),
    );
    mgr.ability_defs.insert(AIM, ability(AIM, 0, vec![700]));
    mgr
}

/// What `fire_beneficial` does for a press: every effect's script, in
/// `effect_ids` order, from `source` onto `target`.
fn press_at(mgr: &mut SpaceManager, ability_id: i32, source: u32, target: u32) {
    let ids = mgr.ability_defs[&ability_id].effect_ids.clone();
    for id in ids {
        let effect = mgr.effect_defs[&id].clone();
        let script = registry::lookup(effect.script_name.as_deref().unwrap()).unwrap();
        let mut ctx = EffectContext {
            source_id: source,
            target_id: target,
            effect: &effect,
            space_mgr: mgr,
        };
        script.on_apply(&mut ctx);
    }
}

fn press(mgr: &mut SpaceManager, ability_id: i32) {
    press_at(mgr, ability_id, PLAYER, PLAYER);
}

fn cur(mgr: &SpaceManager, entity: u32, stat: i32) -> i32 {
    mgr.get_entity(entity).unwrap().stats.get(stat).unwrap().cur
}

fn effect_ids(mgr: &SpaceManager, entity: u32) -> Vec<i32> {
    let mut ids: Vec<i32> = mgr
        .get_entity(entity)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .map(|b| b.effect_id)
        .collect();
    ids.sort_unstable();
    ids
}

fn soldier_stats(mgr: &SpaceManager) -> [i32; 3] {
    [
        cur(mgr, PLAYER, SUBTLETY),
        cur(mgr, PLAYER, MENTAL_RES),
        cur(mgr, PLAYER, COVER_DEFENSE),
    ]
}

/// Press, press: the stance goes on held, then comes off with every stat
/// back exactly where it was. Fails on revert: without the toggle the
/// second press refreshes the entries and the stats stay raised.
#[test]
fn a_second_press_turns_the_stance_off_and_restores_its_stats_exactly() {
    let capture = LogCapture::install();
    let mut mgr = world();
    let before = soldier_stats(&mgr);
    press(&mut mgr, SOLDIER);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![2003, 2004, 2005]);
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .all(|b| b.expires_at.is_none()));
    assert_eq!(
        soldier_stats(&mgr),
        [before[0] - 100, before[1] + 50, before[2] + 100]
    );

    press(&mut mgr, SOLDIER);
    assert!(
        effect_ids(&mgr, PLAYER).is_empty(),
        "the second press turns it off"
    );
    assert_eq!(soldier_stats(&mgr), before, "every stat restored exactly");
    assert!(capture
        .find_event(Level::INFO, "timed effect removed", "toggled_off")
        .is_some());
}

/// A repeated or forged press only flips the switch: on, off, on leaves one
/// entry per effect, never two, and the stats moved once.
#[test]
fn repeated_presses_never_stack_the_stance() {
    let mut mgr = world();
    let before = soldier_stats(&mgr);
    for _ in 0..3 {
        press(&mut mgr, SOLDIER);
    }
    assert_eq!(effect_ids(&mgr, PLAYER), vec![2003, 2004, 2005]);
    assert_eq!(
        soldier_stats(&mgr),
        [before[0] - 100, before[1] + 50, before[2] + 100]
    );
}

/// The switch is the ability's last held effect (2003). With one of the
/// other entries gone (a cleanse), the next press still turns everything
/// off; with the switch's own entry gone, the next press turns everything
/// back on, refreshing the survivors instead of stacking them.
#[test]
fn a_partly_removed_stance_converges_on_the_next_press() {
    let mut mgr = world();
    let before = soldier_stats(&mgr);
    press(&mut mgr, SOLDIER);
    let _ = mgr.remove_timed_effects(PLAYER, StatBuffRemoval::Cleansed, |b| b.effect_id == 2004);
    press(&mut mgr, SOLDIER);
    assert!(effect_ids(&mgr, PLAYER).is_empty());
    assert_eq!(soldier_stats(&mgr), before);

    press(&mut mgr, SOLDIER);
    let _ = mgr.remove_timed_effects(PLAYER, StatBuffRemoval::Cleansed, |b| b.effect_id == 2003);
    press(&mut mgr, SOLDIER);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![2003, 2004, 2005]);
    assert_eq!(
        soldier_stats(&mgr),
        [before[0] - 100, before[1] + 50, before[2] + 100]
    );
}

/// A new stance replaces the old even when it authors no removal effect
/// (1458): Soldier's entries come off by `EFFECT_Stance`. Fails on revert:
/// both stances stay on.
#[test]
fn switching_stance_removes_the_old_one() {
    let mut mgr = world();
    let before = soldier_stats(&mgr);
    press(&mut mgr, SOLDIER);
    press(&mut mgr, RANGED_SPECIALIST);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![1749]);
    assert_eq!(
        soldier_stats(&mgr),
        before,
        "the old stance's stats restored"
    );
    assert_eq!(cur(&mgr, PLAYER, ACCURACY), 100);
}

/// The authored removal (859's 4294 `RemoveByMoniker`) takes the old stance
/// off and leaves its own stance alone, whichever order the two run in.
#[test]
fn an_authored_stance_removal_takes_off_only_other_stances() {
    let mut mgr = world();
    press(&mut mgr, RANGED_SPECIALIST);
    press(&mut mgr, CONCENTRATION);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![922]);
    // Run the removal on its own with Concentration on: nothing of its own goes.
    let effect = mgr.effect_defs[&4294].clone();
    let mut ctx = EffectContext {
        source_id: PLAYER,
        target_id: PLAYER,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RemoveByMoniker.on_apply(&mut ctx);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![922]);
}

/// The B-74 hazard: Aim shares ability moniker 1470900795 with every
/// stance. Switching stance, and the authored removal, take off only
/// `EFFECT_Stance` entries, so Aim survives both. Fails if removal keys on
/// an ability moniker.
#[test]
fn removing_a_stance_never_strips_aim() {
    let mut mgr = world();
    let aim = mgr.effect_defs[&700].clone();
    let mut ctx = EffectContext {
        source_id: PLAYER,
        target_id: PLAYER,
        effect: &aim,
        space_mgr: &mut mgr,
    };
    TimedStat.on_apply(&mut ctx);
    assert!(mgr.get_entity(PLAYER).unwrap().stat_buffs.entries[0].has_moniker(SHARED_MONIKER));
    press(&mut mgr, SOLDIER);
    press(&mut mgr, CONCENTRATION);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![700, 922]);
    assert_eq!(cur(&mgr, PLAYER, ACCURACY), 200, "Aim's +200 untouched");
    let stance_entry = mgr
        .get_entity(PLAYER)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .find(|b| b.effect_id == 922)
        .unwrap()
        .clone();
    assert!(stance_entry.has_moniker(EFFECT_STANCE_MONIKER));
}

/// A held entry lands only on its own invoker: a toggle cast at another
/// entity puts nothing there (it could never be pressed off again).
#[test]
fn a_held_toggle_at_another_entity_is_refused() {
    let capture = LogCapture::install();
    let mut mgr = world();
    press_at(&mut mgr, SOLDIER, PLAYER, OTHER);
    assert!(effect_ids(&mgr, OTHER).is_empty());
    assert!(capture
        .find_event(Level::WARN, "held toggle effect", "held_not_self")
        .is_some());
}

/// A passive (`EF_AlwaysPersist`, 809 Mental Fortitude's 854) holds on the
/// player and comes off with its `on_remove` (the respec).
#[test]
fn a_passive_holds_until_removed() {
    let mut mgr = world();
    let before = cur(&mgr, PLAYER, MENTAL_RES);
    let passive = held(
        854,
        809,
        EF_ALWAYS_PERSIST | 1,
        &[("MentalResistance", "150")],
    );
    let mut ctx = EffectContext {
        source_id: PLAYER,
        target_id: PLAYER,
        effect: &passive,
        space_mgr: &mut mgr,
    };
    TimedStat.on_apply(&mut ctx);
    // A second login pass refreshes it, never stacks it.
    TimedStat.on_apply(&mut ctx);
    assert_eq!(cur(ctx.space_mgr, PLAYER, MENTAL_RES), before + 150);
    TimedStat.on_remove(&mut ctx);
    assert_eq!(cur(&mgr, PLAYER, MENTAL_RES), before);
    assert!(effect_ids(&mgr, PLAYER).is_empty());
}

/// The removal strips only its caster's own stance: run at another entity
/// (a forged route), it removes nothing there.
#[test]
fn a_stance_removal_at_another_entity_removes_nothing() {
    let capture = LogCapture::install();
    let mut mgr = world();
    press_at(&mut mgr, SOLDIER, OTHER, OTHER);
    assert_eq!(effect_ids(&mgr, OTHER), vec![2003, 2004, 2005], "fixture");
    let effect = mgr.effect_defs[&4294].clone();
    let mut ctx = EffectContext {
        source_id: PLAYER,
        target_id: OTHER,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    RemoveByMoniker.on_apply(&mut ctx);
    assert_eq!(effect_ids(&mgr, OTHER), vec![2003, 2004, 2005]);
    assert!(capture
        .find_event(
            Level::WARN,
            "RemoveByMoniker removes only",
            "remove_not_self"
        )
        .is_some());
}

/// An effect tagged with a moniker the server does not know carries only
/// its ability's monikers, and says so: a typo must never become a key.
#[test]
fn an_unknown_effect_moniker_tag_is_dropped_with_a_warning() {
    let capture = LogCapture::install();
    let mut mgr = world();
    let mut bogus = held(1749, RANGED_SPECIALIST, 21, &[("Accuracy", "100")]);
    bogus
        .params
        .insert("EffectMoniker".to_string(), "EFFECT_Stanse".to_string());
    mgr.effect_defs.insert(1749, bogus);
    press(&mut mgr, RANGED_SPECIALIST);
    let entry = &mgr.get_entity(PLAYER).unwrap().stat_buffs.entries[0];
    assert_eq!(entry.moniker_ids, vec![SHARED_MONIKER, 3_212_632_871]);
    assert!(capture
        .find_message(Level::WARN, "effect names an effect moniker")
        .is_some());
}

/// An unknown effect-moniker name never resolves to an id, so a removal
/// keyed on it removes nothing and says why.
#[test]
fn an_unknown_removal_moniker_removes_nothing() {
    let capture = LogCapture::install();
    let mut mgr = world();
    press(&mut mgr, SOLDIER);
    let mut bad = remove_stance(4999, CONCENTRATION);
    bad.params
        .insert("RemoveMoniker".to_string(), "EFFECT_Shield".to_string());
    let mut ctx = EffectContext {
        source_id: PLAYER,
        target_id: PLAYER,
        effect: &bad,
        space_mgr: &mut mgr,
    };
    RemoveByMoniker.on_apply(&mut ctx);
    assert_eq!(effect_ids(&mgr, PLAYER), vec![2003, 2004, 2005]);
    assert!(capture
        .find_event(Level::WARN, "RemoveByMoniker effect", "unknown_moniker")
        .is_some());
}
