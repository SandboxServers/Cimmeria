//! `.cleareffects [target]`: every ledger entry and pulsing effect off, with
//! reason `cleansed`, the stats and flags they held restored, and the
//! client's icons cleared.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::{ActiveEffectInstance, TimedEffectSpec};
use cimmeria_entity::stats::ACCURACY;
use tracing::Level;

use cimmeria_cell_world::cell::effects::registry::EffectScripts;
use cimmeria_cell_world::cell::effects::{EffectContext, EffectScript};
use cimmeria_common::EntityId;
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::QR_MOD;

use super::{aim, console, lines, timers, world, CALLER};
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::{ON_STATE_FIELD_UPDATE, ON_STAT_UPDATE};
use crate::test_support::LogCapture;

const MOVEMENT_LOCK: u32 = 1 << 6;

fn pulse(effect_id: i32, invoker: u32) -> ActiveEffectInstance {
    ActiveEffectInstance {
        effect_id,
        ability_id: 800,
        invoker_id: invoker,
        remaining_pulses: 3,
        total_pulses: 5,
        next_pulse_at: Instant::now() + Duration::from_secs(2),
        pulse_interval_secs: 2.0,
        invoker_position_at_register: None,
        cast_id: Some(12),
        invoker_identity: Default::default(),
    }
}

/// `(effect_id, type, total, complete)` of each `onTimerUpdate` to `eid`.
fn effect_clears(msgs: &[CellToBaseMsg], eid: u32) -> Vec<(i32, u8, [u8; 8])> {
    timers(msgs)
        .into_iter()
        .filter(|(e, _)| *e == eid)
        .map(|(_, a)| {
            (
                i32::from_le_bytes(a[0..4].try_into().unwrap()),
                a[4],
                a[13..21].try_into().unwrap(),
            )
        })
        .collect()
}

fn sent(msgs: &[CellToBaseMsg], eid: u32, method: u16) -> bool {
    msgs.iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, method_index, .. }
            if *entity_id == eid && *method_index == method)
    })
}

#[tokio::test]
async fn ab_l2_cleareffects_strips_the_ledger_and_pulses_and_clears_the_icons() {
    let (mut mgr, _npc) = world(2);
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    let accuracy_before = caller.stats.get(ACCURACY).unwrap().cur;
    let now = Instant::now();
    caller.apply_timed_effect(aim(CALLER), now).unwrap();
    let stun = TimedEffectSpec {
        effect_id: 900,
        stats: vec![],
        state_flags: MOVEMENT_LOCK,
        duration_secs: None,
        ..aim(2)
    };
    caller.apply_timed_effect(stun, now).unwrap();
    caller.active_effects.push(pulse(5001, 2));
    let _ = caller.take_ledger_state_change();
    caller.stats.clear_dirty();
    caller.stat_buffs.pending_timer_clears.clear();
    assert!(caller.state_field & MOVEMENT_LOCK != 0, "fixture: stunned");
    let logs = LogCapture::install();

    let msgs = console(&mut mgr, None, ".cleareffects").await;

    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(caller.stat_buffs.entries.is_empty());
    assert!(caller.active_effects.is_empty());
    assert!(
        caller.stat_buffs.pending_timer_clears.is_empty(),
        "all sent"
    );
    assert_eq!(caller.stats.get(ACCURACY).unwrap().cur, accuracy_before);
    assert_eq!(
        caller.state_field & MOVEMENT_LOCK,
        0,
        "the stun's hold is released"
    );

    let mut clears = effect_clears(&msgs, CALLER);
    clears.sort_unstable();
    assert_eq!(
        clears,
        vec![(700, 5, [0; 8]), (900, 5, [0; 8]), (5001, 5, [0; 8])],
        "one zero-time TIMER_DURATION_EFFECT clear per effect"
    );
    assert!(
        sent(&msgs, CALLER, ON_STATE_FIELD_UPDATE),
        "lock release sent"
    );
    assert!(sent(&msgs, CALLER, ON_STAT_UPDATE), "restored stats sent");

    let out = lines(&msgs);
    assert_eq!(out.len(), 1);
    assert!(
        out[0].starts_with(&format!("cleareffects [{CALLER}]"))
            && out[0].contains("2 ledger entr(ies) [700, 900]")
            && out[0].contains("1 pulsing effect(s) [5001]"),
        "{out:?}"
    );
    let removed: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|e| e.has_field("event", "stat_buff_removed"))
        .collect();
    assert_eq!(removed.len(), 2);
    assert!(removed.iter().all(|r| r.has_field("reason", "cleansed")));
    let row = logs
        .find_message(Level::INFO, "GM stripped every timed and pulsing effect")
        .expect("one abilities.gm row");
    assert!(row.has_field("target_id", &CALLER.to_string()));
    assert!(row.has_field("pulses_removed", "1"));
}

#[tokio::test]
async fn ab_l2_cleareffects_on_a_selected_npc_leaves_the_caller_alone() {
    let (mut mgr, npc) = world(2);
    let now = Instant::now();
    mgr.get_entity_mut(npc)
        .unwrap()
        .apply_timed_effect(aim(CALLER), now)
        .unwrap();
    mgr.get_entity_mut(CALLER)
        .unwrap()
        .apply_timed_effect(aim(CALLER), now)
        .unwrap();

    let out = lines(&console(&mut mgr, Some(npc), ".cleareffects").await);

    assert!(mgr.get_entity(npc).unwrap().stat_buffs.entries.is_empty());
    assert_eq!(
        mgr.get_entity(CALLER).unwrap().stat_buffs.entries.len(),
        1,
        "the caller's own effects stay"
    );
    assert!(
        out[0].starts_with(&format!("cleareffects [{npc}]")),
        "{out:?}"
    );
}

#[tokio::test]
async fn ab_l2_cleareffects_with_nothing_on_says_so() {
    let (mut mgr, _npc) = world(2);
    let msgs = console(&mut mgr, None, ".cleareffects").await;
    assert!(timers(&msgs).is_empty());
    assert!(lines(&msgs)[0].ends_with("nothing to clear"));
}

const WITNESS: u32 = 2;

/// Copilot finding (#1177): a player's restored stats went only to the
/// player. A witness must get the same `onStatUpdate`, or its view of the
/// player keeps the buffed value (the flush clears the dirty bits, so nothing
/// repairs it later). Revert proof: send with `send_entity_method` again and
/// the witness gets nothing.
#[tokio::test]
async fn ab_l2_cleareffects_sends_the_restored_stats_to_observers() {
    let (mut mgr, _npc) = world(2);
    mgr.get_entity_mut(WITNESS)
        .unwrap()
        .witnesses
        .insert(EntityId(CALLER as i32));
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    caller
        .apply_timed_effect(aim(CALLER), Instant::now())
        .unwrap();
    caller.stats.clear_dirty();

    let msgs = console(&mut mgr, None, ".cleareffects").await;

    let own: Vec<&Vec<u8>> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: CALLER,
                method_index: ON_STAT_UPDATE,
                args,
            } => Some(args),
            _ => None,
        })
        .collect();
    let seen: Vec<&Vec<u8>> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: WITNESS,
                entity_id: CALLER,
                method_index: ON_STAT_UPDATE,
                args,
                ..
            } => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(own.len(), 1, "the owner gets the restored stats once");
    assert_eq!(seen, own, "the observer gets the same restored stats");
}

/// Counts `on_remove` calls on the target, as +1 on its `QR_MOD` max and cur:
/// state on the entity, so parallel tests cannot share it.
struct CleanupProbe;

const PROBE: &str = "AbL2CleanupProbe";

impl EffectScript for CleanupProbe {
    fn on_apply(&self, _ctx: &mut EffectContext) {}
    fn on_remove(&self, ctx: &mut EffectContext) {
        if let Some(s) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|e| e.stats.get_mut(QR_MOD))
        {
            let (min, cur, max) = (s.min, s.cur, s.max);
            s.update(min, cur + 1, max + 1);
        }
    }
}

/// Copilot finding (#1177): the pulse strip runs each removed pulse's
/// script `on_remove` exactly once, so a stateful script undoes its state.
/// Revert proof: drop the `dispatch_on_remove` call and the count is 0.
#[tokio::test]
async fn ab_l2_cleareffects_runs_each_pulse_scripts_on_remove_once() {
    let (mut mgr, _npc) = world(2);
    mgr.install_effect_scripts(
        EffectScripts::build([(PROBE, &CleanupProbe as &'static dyn EffectScript)]).unwrap(),
    );
    mgr.effect_defs.insert(
        5001,
        EffectDef {
            effect_id: 5001,
            ability_id: 800,
            script_name: Some(PROBE.to_string()),
            ..Default::default()
        },
    );
    let caller = mgr.get_entity_mut(CALLER).unwrap();
    caller.active_effects.push(pulse(5001, WITNESS));
    let before = caller.stats.get(QR_MOD).unwrap().cur;

    let msgs = console(&mut mgr, None, ".cleareffects").await;

    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(caller.active_effects.is_empty());
    assert_eq!(
        caller.stats.get(QR_MOD).unwrap().cur - before,
        1,
        "on_remove ran exactly once"
    );
    assert_eq!(effect_clears(&msgs, CALLER), vec![(5001, 5, [0; 8])]);
}
