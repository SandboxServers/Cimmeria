//! `.cleareffects [target]`: every ledger entry and pulsing effect off, with
//! reason `cleansed`, the stats and flags they held restored, and the
//! client's icons cleared.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::{ActiveEffectInstance, TimedEffectSpec};
use cimmeria_entity::stats::ACCURACY;
use tracing::Level;

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
