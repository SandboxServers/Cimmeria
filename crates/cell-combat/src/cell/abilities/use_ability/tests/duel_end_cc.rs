//! Ability mechanics AB-09: crowd control the duel partner put on lives in
//! the timed-effect ledger, not in `active_effects`, and still ends with
//! the duel. A bystander's entry stays.

use std::collections::HashMap;

use cimmeria_entity::abilities::{serialize_timer_update, EffectDef, TIMER_DURATION_EFFECT};
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::duel_end::{dueling_world, pid};
use super::*;
use crate::cell::effects::{dispatch_by_name, EffectContext};
use cimmeria_cell_world::cell::duel::{end_engaged, DuelResources, EndReason};

const A: u32 = 1;
const B: u32 = 2;
const C: u32 = 3;

fn single_pulse(id: i32, script: &str, nvp: &str, value: &str, secs: f32) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: 1355,
        script_name: Some(script.to_string()),
        pulse_count: 1,
        pulse_duration: secs,
        params: HashMap::from([(nvp.to_string(), value.to_string())]),
        ..Default::default()
    }
}

fn land(mgr: &mut SpaceManager, effect: &EffectDef, source: u32, target: u32) {
    let name = effect.script_name.clone().unwrap();
    let mut ctx = EffectContext {
        source_id: source,
        target_id: target,
        effect,
        space_mgr: mgr,
    };
    assert!(dispatch_by_name(&name, &mut ctx));
}

async fn end_duel(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(4096);
    let duel_id = mgr.resources.duels().duel_of(pid(mgr, A)).unwrap().duel_id;
    end_engaged(&tx, mgr, duel_id, EndReason::GmAborted).await;
    drain(&mut rx)
}

/// **Regression guard (a duel stun outlived the duel).** A's single-pulse
/// stun on B is stripped when the duel ends: the lock is released, B's
/// client hears the cleared state field and the icon clear. A bystander's
/// snare on B stays. Before, the strip walked `active_effects` only and the
/// stun kept B locked for its full length (`B is free` fails).
#[tokio::test]
async fn a_partners_stun_ends_with_the_duel() {
    let mut mgr = dueling_world();
    crate::test_support::install_effect_scripts(&mut mgr);
    land(
        &mut mgr,
        &single_pulse(1599, "Stun", "CcDuration", "5", 5.0),
        A,
        B,
    );
    land(
        &mut mgr,
        &single_pulse(1462, "TimedStat", "MovementSpeedMod", "-30", 15.0),
        C,
        B,
    );
    assert!(mgr.get_entity(B).unwrap().has_state_flag(BSF_MOVEMENT_LOCK));
    let _ = mgr.get_entity_mut(B).unwrap().take_ledger_state_change();

    let msgs = end_duel(&mut mgr).await;
    let b = mgr.get_entity(B).unwrap();
    assert!(!b.has_state_flag(BSF_MOVEMENT_LOCK), "B is free");
    assert_eq!(
        b.stat_buffs
            .entries
            .iter()
            .map(|e| (e.effect_id, e.invoker_id))
            .collect::<Vec<_>>(),
        vec![(1462, C)],
        "only the bystander's snare is left"
    );
    let state_updates: Vec<u32> = super::warmup::calls(&msgs)
        .into_iter()
        .filter(|(e, m, _)| *e == B && *m == method_idx::ON_STATE_FIELD_UPDATE)
        .map(|(_, _, a)| u32::from_le_bytes(a[..4].try_into().unwrap()))
        .collect();
    assert!(
        state_updates.iter().all(|s| s & BSF_MOVEMENT_LOCK == 0) && !state_updates.is_empty(),
        "B's client hears the lock clear: {state_updates:?}"
    );
    let clear = serialize_timer_update(1599, TIMER_DURATION_EFFECT, A as i32, 1599, 0.0, 0.0);
    assert!(
        super::warmup::calls(&msgs)
            .iter()
            .any(|(e, m, a)| *e == B && *m == super::warmup::ON_TIMER_UPDATE && *a == clear),
        "the stun's icon clear"
    );
}

/// **Regression guard, the Tranquilizer.** A's dart slow on B is a ledger
/// entry since AB-09b; the duel end restores B's speed exactly.
#[tokio::test]
async fn a_partners_tranquilizer_slow_ends_with_the_duel() {
    let mut mgr = dueling_world();
    crate::test_support::install_effect_scripts(&mut mgr);
    land(
        &mut mgr,
        &single_pulse(9142, "MovementSlow", "SpeedReduction", "40", 6.0),
        A,
        B,
    );
    let speed = |m: &SpaceManager| {
        m.get_entity(B)
            .unwrap()
            .stats
            .get(MOVEMENT_SPEED_MOD)
            .unwrap()
            .cur
    };
    assert_eq!(speed(&mgr), 60);

    let _ = end_duel(&mut mgr).await;
    assert_eq!(speed(&mgr), 100, "speed restored at the duel's end");
    assert!(mgr.get_entity(B).unwrap().stat_buffs.entries.is_empty());
}
