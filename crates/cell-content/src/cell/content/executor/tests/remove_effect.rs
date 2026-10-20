//! `Action::RemoveEffect` executor coverage (HA-03c): the self-only,
//! idempotent removal of an effect's pulsing instances and timed-effect
//! ledger entries, through `content::effect_apply::remove_effect`.
//!
//! Revert proof: with the `RemoveEffect` arm reverted to the `other =>`
//! fallthrough, every case that runs the action through `run_remove` fails:
//! the removal cases because nothing is removed, the no-op and unknown cases
//! because the helper's `content_remove_effect_noop` / `_unknown` event is
//! never logged. Only the direct `remove_effect` return-value checks in cases
//! 4 and 5 are independent of the arm; each is paired with a `run_remove`
//! check that is not.

use std::time::Instant;

use tracing::Level;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::FOCUS;
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::content::effect_apply::remove_effect;
use crate::cell::effects::registry::EffectScripts;
use crate::cell::effects::{flush_stat_buff_timers, EffectContext, EffectScript};
use crate::mercury::method_idx::{ON_STATE_FIELD_UPDATE, ON_STAT_UPDATE};
use crate::test_support::{LogCapture, LogCaptureGuard};

const PLAYER: u32 = 1;
const RIVAL: u32 = 2;
/// The chain id the fixture's actions run under.
const CHAIN: i64 = 5000;

/// A pulsing effect whose script is [`RemoveProbe`].
const PULSE: i32 = 91_300;
/// Timed-effect ledger effects holding `BSF_MOVEMENT_LOCK`.
const LEDGER: i32 = 91_301;
const LEDGER_TWO: i32 = 91_302;
/// Registered nowhere: the unknown-id path.
const UNKNOWN: i32 = 91_399;
/// The effect id the seed's test chains 2023 and 2024 remove. The fixture
/// does not register it.
const SEED_CHAIN_EFFECT: i32 = 1635;

const PROBE: &str = "RemoveEffectProbe";

/// Counts `on_remove` on its target as +1 on `FOCUS` current, so a test reads
/// the run count from the entity itself and parallel tests share no state.
struct RemoveProbe;

impl EffectScript for RemoveProbe {
    fn on_apply(&self, _ctx: &mut EffectContext) {}
    fn on_remove(&self, ctx: &mut EffectContext) {
        if let Some(s) = ctx
            .space_mgr
            .get_entity_mut(ctx.target_id)
            .and_then(|e| e.stats.get_mut(FOCUS))
        {
            let (min, cur, max) = (s.min, s.cur, s.max);
            s.update(min, cur + 1, max);
        }
    }
}

static PROBE_SCRIPT: RemoveProbe = RemoveProbe;

/// The player (entity 1, player 42) and a rival player (entity 2, player 43),
/// both in the startup space, with the probe script installed and the four
/// effect defs registered.
fn make_mgr() -> SpaceManager {
    let mut mgr = make_space_mgr();
    for (id, player_id) in [(PLAYER, 42), (RIVAL, 43)] {
        mgr.create_entity(id, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(id) {
            e.is_player = true;
            e.player_id = Some(player_id);
            if let Some(s) = e.stats.get_mut(FOCUS) {
                s.update(0, 200, 1000);
            }
        }
    }
    mgr.install_effect_scripts(
        EffectScripts::build([(PROBE, &PROBE_SCRIPT as &'static dyn EffectScript)]).unwrap(),
    );
    mgr.effect_defs.insert(
        PULSE,
        EffectDef {
            effect_id: PULSE,
            ability_id: 91_200,
            script_name: Some(PROBE.to_string()),
            pulse_count: 4,
            pulse_duration: 2.0,
            ..Default::default()
        },
    );
    for effect_id in [LEDGER, LEDGER_TWO] {
        mgr.effect_defs.insert(
            effect_id,
            EffectDef {
                effect_id,
                ability_id: 91_200,
                ..Default::default()
            },
        );
    }
    mgr
}

/// A timed-effect ledger entry on `target` that holds the movement lock.
fn lock_spec(effect_id: i32) -> TimedEffectSpec {
    TimedEffectSpec {
        cast_id: None,
        effect_id,
        ability_id: 91_200,
        invoker_id: PLAYER,
        effect_flags: 0,
        moniker_ids: vec![],
        stats: vec![],
        absorb: Vec::new(),
        duration_secs: Some(3600.0),
        stacking: TimedStacking::PerSource,
        state_flags: BSF_MOVEMENT_LOCK,
        invoker_identity: Default::default(),
        invoker_name: None,
    }
}

fn resolved(action: Action) -> ResolvedActions {
    ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(CHAIN, action)],
        action_delays: Vec::new(),
    }
}

/// Run `Action::RemoveEffect` for `effect_id` on the player, as a chain would.
async fn run_remove(mgr: &mut SpaceManager, effect_id: i32, tx: &mpsc::Sender<CellToBaseMsg>) {
    let engine = ChainEngine::new();
    execute_actions(
        resolved(Action::RemoveEffect { effect_id }),
        PLAYER,
        42,
        tx,
        mgr,
        &engine,
    )
    .await;
}

/// Apply the pulsing [`PULSE`] effect to `target` through the real
/// `Action::ApplyEffect` path, which registers the instance.
async fn apply_pulse(mgr: &mut SpaceManager, target: u32, tx: &mpsc::Sender<CellToBaseMsg>) {
    let engine = ChainEngine::new();
    let player_id = if target == PLAYER { 42 } else { 43 };
    execute_actions(
        resolved(Action::ApplyEffect {
            effect_id: PULSE,
            duration_secs: None,
        }),
        target,
        player_id,
        tx,
        mgr,
        &engine,
    )
    .await;
}

/// Every `(method_index, args)` sent to `entity`'s own client.
fn sent_to(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity: u32) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if entity_id == entity => out.push((method_index, args)),
            _ => {}
        }
    }
    out
}

fn count(sent: &[(u16, Vec<u8>)], method: u16) -> usize {
    sent.iter().filter(|(m, _)| *m == method).count()
}

fn focus(mgr: &SpaceManager, entity: u32) -> i32 {
    mgr.get_entity(entity)
        .unwrap()
        .stats
        .get(FOCUS)
        .unwrap()
        .cur
}

fn holds_lock(mgr: &SpaceManager, entity: u32) -> bool {
    mgr.get_entity(entity)
        .unwrap()
        .holds_ledger_flag(BSF_MOVEMENT_LOCK)
}

fn ledger_has(mgr: &SpaceManager, entity: u32, effect_id: i32) -> bool {
    mgr.get_entity(entity)
        .unwrap()
        .stat_buffs
        .entries
        .iter()
        .any(|e| e.effect_id == effect_id)
}

/// `true` when a `level` event whose message contains `msg` carries
/// `event = ev`. The `event` field is what the log-contract checks key on;
/// `find_event` matches `reason`, so it cannot find these rows.
fn logged(capture: &LogCaptureGuard, level: Level, msg: &str, ev: &str) -> bool {
    capture
        .find_message(level, msg)
        .is_some_and(|c| c.has_field("event", ev))
}

/// Case 1. A pulsing instance is removed: its `on_remove` runs once, its icon
/// clear goes out once, and the instance is gone.
#[tokio::test]
async fn remove_effect_takes_a_pulse_runs_on_remove_once_and_clears_its_icon() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    apply_pulse(&mut mgr, PLAYER, &tx).await;
    assert_eq!(
        mgr.get_entity(PLAYER).unwrap().active_effects.len(),
        1,
        "fixture: the pulse registered"
    );
    let before = focus(&mgr, PLAYER);
    sent_to(&mut rx, PLAYER);

    let capture = LogCapture::install();
    run_remove(&mut mgr, PULSE, &tx).await;

    let target = mgr.get_entity(PLAYER).unwrap();
    assert!(target.active_effects.is_empty(), "the instance is gone");
    assert_eq!(
        focus(&mgr, PLAYER),
        before + 1,
        "on_remove ran exactly once"
    );
    assert!(
        target.stat_buffs.pending_timer_clears.is_empty(),
        "the queued clear was flushed"
    );
    let sent = sent_to(&mut rx, PLAYER);
    assert_eq!(count(&sent, ON_TIMER_UPDATE), 1, "one icon clear: {sent:?}");
    assert_eq!(
        count(&sent, ON_STAT_UPDATE),
        1,
        "the stat on_remove moved goes out once: {sent:?}"
    );
    assert!(logged(
        &capture,
        Level::INFO,
        "removed effect (server-initiated)",
        "content_remove_effect"
    ));
}

/// Case 2. A ledger entry is removed: the entry goes and the counted lock
/// drains, so `BSF_MOVEMENT_LOCK` is clear.
#[tokio::test]
async fn remove_effect_takes_a_ledger_entry_and_drains_its_counted_flag() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    mgr.apply_timed_effect(PLAYER, lock_spec(LEDGER), Instant::now());
    assert!(holds_lock(&mgr, PLAYER), "fixture: the lock is held");
    // Send the lock on, so its coming off is a change the client is owed.
    flush_stat_buff_timers(PLAYER, Instant::now(), &tx, &mut mgr).await;
    sent_to(&mut rx, PLAYER);

    run_remove(&mut mgr, LEDGER, &tx).await;

    let sent = sent_to(&mut rx, PLAYER);
    assert_eq!(
        count(&sent, ON_STATE_FIELD_UPDATE),
        1,
        "the lock coming off is sent once: {sent:?}"
    );
    let target = mgr.get_entity(PLAYER).unwrap();
    assert!(!ledger_has(&mgr, PLAYER, LEDGER), "the entry is gone");
    assert!(!holds_lock(&mgr, PLAYER), "the counted lock drained");
    assert!(!target.has_state_flag(BSF_MOVEMENT_LOCK));
}

/// Case 3. Two sources hold the same flag. Removing one leaves the flag set,
/// because the count, not the bit, decides; removing the second clears it.
#[tokio::test]
async fn removing_one_source_keeps_the_flag_the_other_still_holds() {
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(256);
    mgr.apply_timed_effect(PLAYER, lock_spec(LEDGER), Instant::now());
    mgr.apply_timed_effect(PLAYER, lock_spec(LEDGER_TWO), Instant::now());

    run_remove(&mut mgr, LEDGER, &tx).await;
    assert!(!ledger_has(&mgr, PLAYER, LEDGER));
    assert!(
        ledger_has(&mgr, PLAYER, LEDGER_TWO),
        "the other source stays"
    );
    assert!(
        holds_lock(&mgr, PLAYER),
        "the flag stays set for the second"
    );
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .has_state_flag(BSF_MOVEMENT_LOCK));

    run_remove(&mut mgr, LEDGER_TWO, &tx).await;
    assert!(!holds_lock(&mgr, PLAYER), "the last source drains the flag");
}

/// Case 4. Idempotent: the chain action run twice removes the entry once; the
/// second run sends nothing (no second `onStateFieldUpdate`) and logs the
/// no-op. The helper's return values are checked directly on a fresh entry.
#[tokio::test]
async fn a_second_removal_is_a_noop_and_sends_nothing() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    mgr.apply_timed_effect(PLAYER, lock_spec(LEDGER), Instant::now());

    run_remove(&mut mgr, LEDGER, &tx).await;
    assert!(
        !ledger_has(&mgr, PLAYER, LEDGER),
        "the first run removes it"
    );
    sent_to(&mut rx, PLAYER);

    let capture = LogCapture::install();
    run_remove(&mut mgr, LEDGER, &tx).await;
    let sent = sent_to(&mut rx, PLAYER);
    assert_eq!(count(&sent, ON_STATE_FIELD_UPDATE), 0, "no second update");
    assert!(sent.is_empty(), "nothing at all: {sent:?}");
    assert!(logged(
        &capture,
        Level::DEBUG,
        "effect not on the entity",
        "content_remove_effect_noop"
    ));

    mgr.apply_timed_effect(PLAYER, lock_spec(LEDGER), Instant::now());
    assert_eq!(remove_effect(LEDGER, PLAYER, CHAIN, &tx, &mut mgr).await, 1);
    assert_eq!(remove_effect(LEDGER, PLAYER, CHAIN, &tx, &mut mgr).await, 0);
}

/// Case 5. An effect never applied: the action sends nothing and logs the
/// no-op; the helper returns 0.
#[tokio::test]
async fn removing_an_absent_effect_returns_zero_and_sends_nothing() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    sent_to(&mut rx, PLAYER);

    let capture = LogCapture::install();
    run_remove(&mut mgr, PULSE, &tx).await;
    assert!(sent_to(&mut rx, PLAYER).is_empty());
    assert!(logged(
        &capture,
        Level::DEBUG,
        "effect not on the entity",
        "content_remove_effect_noop"
    ));
    assert_eq!(remove_effect(PULSE, PLAYER, CHAIN, &tx, &mut mgr).await, 0);
}

/// Case 6. An unknown effect id logs `content_remove_effect_unknown` at WARN
/// through the chain action, returns 0 and sends nothing.
#[tokio::test]
async fn an_unknown_effect_id_warns_and_is_a_noop() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    sent_to(&mut rx, PLAYER);

    let capture = LogCapture::install();
    run_remove(&mut mgr, UNKNOWN, &tx).await;
    assert!(logged(
        &capture,
        Level::WARN,
        "remove_effect names an effect with no server def",
        "content_remove_effect_unknown"
    ));
    assert!(sent_to(&mut rx, PLAYER).is_empty());
    assert_eq!(
        remove_effect(UNKNOWN, PLAYER, CHAIN, &tx, &mut mgr).await,
        0
    );
}

/// Case 7. Self-only: with the same effect on both players, removing the
/// chain's entity leaves the rival's pulse, lock and stats untouched.
#[tokio::test]
async fn removal_touches_only_the_chains_entity() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    for target in [PLAYER, RIVAL] {
        apply_pulse(&mut mgr, target, &tx).await;
        mgr.apply_timed_effect(target, lock_spec(LEDGER), Instant::now());
    }
    let rival_focus = focus(&mgr, RIVAL);
    sent_to(&mut rx, RIVAL);

    run_remove(&mut mgr, PULSE, &tx).await;
    run_remove(&mut mgr, LEDGER, &tx).await;

    assert!(mgr.get_entity(PLAYER).unwrap().active_effects.is_empty());
    assert!(!holds_lock(&mgr, PLAYER));
    let rival = mgr.get_entity(RIVAL).unwrap();
    assert_eq!(rival.active_effects.len(), 1, "the rival keeps its pulse");
    assert!(holds_lock(&mgr, RIVAL), "the rival keeps its lock");
    assert!(ledger_has(&mgr, RIVAL, LEDGER));
    assert_eq!(focus(&mgr, RIVAL), rival_focus, "no on_remove on the rival");
    assert!(
        sent_to(&mut rx, RIVAL).is_empty(),
        "nothing sent to the rival"
    );
}

/// Case 8. One removal of an effect that is both pulsing and locking sends
/// at most one `onStateFieldUpdate`: the ledger's flush covers the pulse's
/// clear too, so the pulse path adds no second flush.
#[tokio::test]
async fn one_removal_sends_at_most_one_state_field_update() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    apply_pulse(&mut mgr, PLAYER, &tx).await;
    mgr.apply_timed_effect(PLAYER, lock_spec(PULSE), Instant::now());
    flush_stat_buff_timers(PLAYER, Instant::now(), &tx, &mut mgr).await;
    sent_to(&mut rx, PLAYER);

    run_remove(&mut mgr, PULSE, &tx).await;

    assert!(mgr.get_entity(PLAYER).unwrap().active_effects.is_empty());
    assert!(!holds_lock(&mgr, PLAYER));
    let sent = sent_to(&mut rx, PLAYER);
    assert_eq!(
        count(&sent, ON_STATE_FIELD_UPDATE),
        1,
        "one update: {sent:?}"
    );
}

/// Case 9. The seed's test chains 2023 and 2024 remove effect
/// 1635, which this fixture does not register. The action must not panic,
/// must warn and must send nothing. The fixture runs the action directly,
/// not the seed chain rows.
#[tokio::test]
async fn the_seed_chains_effect_1635_is_a_harmless_noop() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    sent_to(&mut rx, PLAYER);

    let capture = LogCapture::install();
    run_remove(&mut mgr, SEED_CHAIN_EFFECT, &tx).await;
    assert!(logged(
        &capture,
        Level::WARN,
        "remove_effect names an effect with no server def",
        "content_remove_effect_unknown"
    ));
    assert!(sent_to(&mut rx, PLAYER).is_empty());
}
