//! AB-04: a beneficial single-pulse timed effect (a buff) reaches the timed
//! effect ledger through `fire_beneficial`, on the caster, with its icon.
//!
//! `duel_gate`'s fixture: player A (1) casts, player B (2) is an ally, NPC 4
//! a hostile mob. The abilities copy the seed after the `stat` family:
//! 637 Aim (Self, effect 700 `TimedStat` `Accuracy 200`, 15 s, flags 21) and
//! 1619 Combat Sprint (Self, effect 1962 `TimedStat` `MovementSpeedMod 50`,
//! 10 s, flags 23; its non-beneficial "-100 ACC" half 2002 stays unbound).

use std::collections::HashMap;

use cimmeria_entity::abilities::{AbilityType, EffectDef, TARGET_SELF};
use cimmeria_entity::stats::{ACCURACY, MOVEMENT_SPEED_MOD};

use super::duel_gate::{duel_mgr, A, MOB};
use super::warmup::calls;
use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;

pub(super) const AIM: i32 = 637;
pub(super) const AIM_EFFECT: i32 = 700;
const COMBAT_SPRINT: i32 = 1619;
const SPRINT_RUN: i32 = 1962;
const SPRINT_PENALTY: i32 = 2002;

fn timed(id: i32, ability: i32, nvp: &str, value: &str, secs: f32, flags: u32) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: ability,
        script_name: Some("TimedStat".to_string()),
        flags,
        pulse_count: 1,
        pulse_duration: secs,
        params: HashMap::from([(nvp.to_string(), value.to_string())]),
        ..Default::default()
    }
}

pub(super) fn buff_mgr() -> SpaceManager {
    let mut mgr = duel_mgr();
    crate::test_support::install_effect_scripts(&mut mgr);
    for (id, effects) in [
        (AIM, vec![AIM_EFFECT]),
        (COMBAT_SPRINT, vec![SPRINT_RUN, SPRINT_PENALTY]),
    ] {
        mgr.ability_defs.insert(
            id,
            AbilityDef {
                cooldown: 30.0,
                target_type_id: TARGET_SELF,
                effect_ids: effects,
                type_id: AbilityType::Buff,
                ..make_ability(id, 0, 30)
            },
        );
    }
    mgr.effect_defs.insert(
        AIM_EFFECT,
        timed(AIM_EFFECT, AIM, "Accuracy", "200", 15.0, 21),
    );
    mgr.effect_defs.insert(
        SPRINT_RUN,
        timed(
            SPRINT_RUN,
            COMBAT_SPRINT,
            "MovementSpeedMod",
            "50",
            10.0,
            23,
        ),
    );
    // The generator leaves the penalty half unbound (no script, no NVP).
    mgr.effect_defs.insert(
        SPRINT_PENALTY,
        EffectDef {
            effect_id: SPRINT_PENALTY,
            ability_id: COMBAT_SPRINT,
            flags: 534,
            pulse_count: 1,
            pulse_duration: 10.0,
            ..Default::default()
        },
    );
    let a = mgr.get_entity_mut(A).unwrap();
    a.abilities.add_ability(AIM);
    a.abilities.add_ability(COMBAT_SPRINT);
    for eid in [A, MOB] {
        mgr.get_entity_mut(eid).unwrap().stats.clear_dirty();
    }
    mgr
}

fn stat(mgr: &SpaceManager, eid: u32, id: i32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(id).unwrap().cur
}

/// **Regression guard (B-30, B-32).** Aim with a mob selected raises the
/// caster's Accuracy by 200, never the mob's, and the caster's client gets
/// the 15 s icon (timer id 700, SecondaryId 700) in the same press. Before
/// AB-04 effect 700 had no script and no NVP: Accuracy stays 0
/// (`A's Accuracy` fails).
#[tokio::test]
async fn aim_buffs_the_casters_accuracy_with_its_icon() {
    let mut mgr = buff_mgr();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, AIM, MOB as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    assert_eq!(stat(&mgr, A, ACCURACY), 200, "A's Accuracy");
    assert_eq!(
        stat(&mgr, MOB, ACCURACY),
        0,
        "the mob's Accuracy must not change"
    );
    let icons: Vec<Vec<u8>> = calls(&msgs)
        .into_iter()
        .filter(|(e, m, _)| *e == A && *m == ON_TIMER_UPDATE)
        .map(|(_, _, a)| a)
        .filter(|a| a[4] == 5)
        .collect();
    assert_eq!(icons.len(), 1, "one duration icon: {msgs:?}");
    assert_eq!(&icons[0][..4], &AIM_EFFECT.to_le_bytes());
    assert_eq!(&icons[0][9..13], &AIM_EFFECT.to_le_bytes(), "SecondaryId");
    assert!(
        calls(&msgs)
            .iter()
            .any(|(e, m, _)| *e == A && *m == method_idx::ON_STAT_UPDATE),
        "the caster hears the Accuracy change"
    );
}

/// Pressing Aim again refreshes rather than stacks: one entry, Accuracy
/// still +200, and a later expiry.
#[tokio::test]
async fn aim_twice_refreshes() {
    let mut mgr = buff_mgr();
    let (tx, _rx) = mpsc::channel(256);
    let expiry = |mgr: &SpaceManager| {
        let entries = &mgr.get_entity(A).unwrap().stat_buffs.entries;
        assert_eq!(entries.len(), 1, "exactly one Aim entry");
        entries[0].expires_at.expect("a timed entry")
    };
    assert!(handle_use_ability(A, AIM, 0, &tx, &mut mgr).await);
    let first = expiry(&mgr);
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    mgr.get_entity_mut(A)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    assert!(handle_use_ability(A, AIM, 0, &tx, &mut mgr).await);
    assert_eq!(stat(&mgr, A, ACCURACY), 200);
    assert!(expiry(&mgr) > first, "the refresh restarts the 15 s");
}

/// **Regression guard (B-27 routing).** Combat Sprint with a mob selected
/// speeds the caster up by 50 % and leaves the mob alone: the unbound
/// penalty half keeps the ability beneficial, so it resolves on the caster.
#[tokio::test]
async fn combat_sprint_speeds_the_caster_not_the_mob() {
    let mut mgr = buff_mgr();
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(A, COMBAT_SPRINT, MOB as i32, &tx, &mut mgr).await);

    assert_eq!(stat(&mgr, A, MOVEMENT_SPEED_MOD), 150);
    assert_eq!(stat(&mgr, MOB, MOVEMENT_SPEED_MOD), 100);
}
