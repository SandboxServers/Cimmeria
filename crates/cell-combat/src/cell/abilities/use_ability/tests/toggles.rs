//! AB-08 through the whole press: a stance toggles on and off through
//! `handle_use_ability`, shows a held icon, and a new stance replaces the
//! old without touching Aim.
//!
//! `duel_gate`'s fixture (player A casts, NPC 4 is selected). The abilities
//! copy the seed after the `stat` family: 1642 Stance: Soldier (Self,
//! `AF_TOGGLED`, effects 2005 / 2004 / 2003 held `TimedStat` with
//! `EffectMoniker` `EFFECT_Stance`), 859 Concentration (922 held, plus its
//! "Remove Effect of moniker EFFECT_Stance" 4294 `RemoveByMoniker`, flags
//! 0), and 637 Aim (700, 15 s). Cooldowns are cleared between presses: each
//! press is charged its cooldown at launch, as for the owner-pet toggles.

use std::collections::HashMap;

use cimmeria_entity::abilities::{AbilityType, EffectDef, AF_TOGGLED, TARGET_SELF};
use cimmeria_entity::stats::{ACCURACY, COVER_DEFENSE, INTERRUPT_RES, MENTAL_RES, SUBTLETY};

use super::duel_gate::{duel_mgr, A, MOB};
use super::warmup::calls;
use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::effects::HELD_ICON_SECS;

const SOLDIER: i32 = 1642;
const CONCENTRATION: i32 = 859;
const AIM: i32 = 637;
const SHARED_MONIKER: i64 = 1_470_900_795;

fn effect(
    id: i32,
    ability: i32,
    flags: u32,
    secs: f32,
    script: &str,
    nvps: &[(&str, &str)],
) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: ability,
        script_name: Some(script.to_string()),
        flags,
        pulse_count: 1,
        pulse_duration: secs,
        params: nvps
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        ..Default::default()
    }
}

fn stance_mgr() -> SpaceManager {
    let mut mgr = duel_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    let tag = ("EffectMoniker", "EFFECT_Stance");
    for e in [
        effect(
            2005,
            SOLDIER,
            533,
            0.0,
            "TimedStat",
            &[("Subtlety", "-100"), tag],
        ),
        effect(
            2004,
            SOLDIER,
            533,
            0.0,
            "TimedStat",
            &[("MentalResistance", "50"), tag],
        ),
        effect(
            2003,
            SOLDIER,
            21,
            0.0,
            "TimedStat",
            &[("CoverDefense", "100"), tag],
        ),
        effect(
            922,
            CONCENTRATION,
            85,
            0.0,
            "TimedStat",
            &[("InterruptResistance", "250"), tag],
        ),
        effect(
            4294,
            CONCENTRATION,
            0,
            0.0,
            "RemoveByMoniker",
            &[("RemoveMoniker", "EFFECT_Stance")],
        ),
        effect(700, AIM, 21, 15.0, "TimedStat", &[("Accuracy", "200")]),
    ] {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    for (id, flags, effects) in [
        (SOLDIER, AF_TOGGLED, vec![2005, 2004, 2003]),
        (CONCENTRATION, AF_TOGGLED | 512, vec![922, 4294]),
        (AIM, 0, vec![700]),
    ] {
        mgr.ability_defs.insert(
            id,
            AbilityDef {
                cooldown: 2.0,
                flags,
                target_type_id: TARGET_SELF,
                effect_ids: effects,
                moniker_ids: vec![SHARED_MONIKER],
                type_id: AbilityType::Buff,
                ..make_ability(id, 0, 30)
            },
        );
        mgr.get_entity_mut(A).unwrap().abilities.add_ability(id);
    }
    for eid in [A, MOB] {
        mgr.get_entity_mut(eid).unwrap().stats.clear_dirty();
    }
    mgr
}

fn stat(mgr: &SpaceManager, eid: u32, id: i32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(id).unwrap().cur
}

fn soldier(mgr: &SpaceManager, eid: u32) -> [i32; 3] {
    [
        stat(mgr, eid, SUBTLETY),
        stat(mgr, eid, MENTAL_RES),
        stat(mgr, eid, COVER_DEFENSE),
    ]
}

async fn press(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>, ability: i32) {
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    assert!(handle_use_ability(A, ability, MOB as i32, tx, mgr).await);
}

/// The caster's duration timers in `msgs`: `(effect id, TotalTime)`.
fn icons(msgs: &[CellToBaseMsg]) -> Vec<(i32, f32)> {
    calls(msgs)
        .into_iter()
        .filter(|(e, m, a)| *e == A && *m == ON_TIMER_UPDATE && a[4] == 5)
        .map(|(_, _, a)| {
            (
                i32::from_le_bytes(a[..4].try_into().unwrap()),
                f32::from_le_bytes(a[13..17].try_into().unwrap()),
            )
        })
        .collect()
}

/// **Regression guard (B-35).** The first press puts Stance: Soldier on the
/// caster (never the selected mob) with a held icon per effect; the second
/// takes it off, every stat back exactly, and clears the icons. On revert
/// (no toggle) the held effects are refused and the stats never move.
#[tokio::test]
async fn stance_soldier_toggles_on_and_off_through_the_press() {
    let mut mgr = stance_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    let before = soldier(&mgr, A);
    let mob_before = soldier(&mgr, MOB);

    press(&mut mgr, &tx, SOLDIER).await;
    let on = drain(&mut rx);
    assert_eq!(
        soldier(&mgr, A),
        [before[0] - 100, before[1] + 50, before[2] + 100]
    );
    assert_eq!(soldier(&mgr, MOB), mob_before, "the mob is untouched");
    let mut started = icons(&on);
    started.sort_by_key(|i| i.0);
    assert_eq!(
        started,
        vec![
            (2003, HELD_ICON_SECS),
            (2004, HELD_ICON_SECS),
            (2005, HELD_ICON_SECS)
        ],
        "one held icon per effect"
    );

    press(&mut mgr, &tx, SOLDIER).await;
    let off = drain(&mut rx);
    assert_eq!(soldier(&mgr, A), before, "every stat restored exactly");
    assert!(mgr.get_entity(A).unwrap().stat_buffs.entries.is_empty());
    let mut cleared = icons(&off);
    cleared.sort_by_key(|i| i.0);
    assert_eq!(
        cleared,
        vec![(2003, 0.0), (2004, 0.0), (2005, 0.0)],
        "the clears"
    );
}

/// **Regression guard (B-35, B-74).** Aim, then Soldier, then Concentration:
/// Concentration replaces Soldier and Aim survives, although all three share
/// ability moniker 1470900795. Concentration lands on the caster: its
/// flags-0 removal half does not make it a hostile cast (on revert of that
/// rule the cast goes down the hostile path and Interrupt Resistance never
/// moves).
#[tokio::test]
async fn a_new_stance_replaces_the_old_and_keeps_aim() {
    let mut mgr = stance_mgr();
    let (tx, _rx) = mpsc::channel(256);
    let before = soldier(&mgr, A);
    let ir = stat(&mgr, A, INTERRUPT_RES);

    press(&mut mgr, &tx, AIM).await;
    press(&mut mgr, &tx, SOLDIER).await;
    press(&mut mgr, &tx, CONCENTRATION).await;

    assert_eq!(soldier(&mgr, A), before, "Soldier is gone");
    assert_eq!(
        stat(&mgr, A, INTERRUPT_RES),
        ir + 250,
        "Concentration is on"
    );
    assert_eq!(stat(&mgr, A, ACCURACY), 200, "Aim is untouched");
    let mut ids: Vec<i32> = mgr
        .get_entity(A)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .map(|b| b.effect_id)
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![700, 922]);
}
