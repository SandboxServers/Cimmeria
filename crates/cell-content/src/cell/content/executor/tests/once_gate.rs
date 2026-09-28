//! `content_triggers.once` (#802): the executor's fire-once gate.
//!
//! The pure `apply_once_gate` shapes first, then the gate as
//! `execute_actions` runs it: per entity, delayed actions, and a once-chain
//! whose conditions fail staying armed. Deleting the `gate_for_entity` call
//! in `execute_actions` fails every `execute_actions`-level test here.

use std::collections::HashSet;

use cimmeria_content_engine::chain::Chain;
use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{Trigger, TriggerEvent, TriggerType};

use super::super::once_gate::apply_once_gate;
use super::*;

const ONCE: i64 = 1008;
const PLAIN: i64 = 1009;

fn bump(name: &str) -> Action {
    Action::IncrementCounter {
        counter_name: name.to_string(),
        amount: 1,
    }
}

fn chain(id: i64, once: bool, conditions: Vec<Condition>, delays: Vec<i32>) -> Chain {
    Chain {
        id,
        name: format!("once_gate_{id}"),
        enabled: true,
        trigger: Trigger::OnCustomEvent {
            event_name: "go".to_string(),
        },
        conditions,
        actions: vec![bump(&format!("c{id}"))],
        action_delays: delays,
        priority: 0,
        once,
    }
}

fn engine_with(chains: Vec<Chain>) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for c in chains {
        engine.register_chain(c);
    }
    engine
}

fn go() -> TriggerEvent {
    TriggerEvent {
        trigger_type: TriggerType::CustomEvent,
        source_entity: None,
        target_entity: None,
        params: [("event_name".to_string(), serde_json::json!("go"))]
            .into_iter()
            .collect(),
    }
}

fn counter(mgr: &SpaceManager, eid: u32, name: &str) -> i32 {
    mgr.get_entity(eid)
        .and_then(|e| e.counters.get(name).copied())
        .unwrap_or(0)
}

// ── apply_once_gate (pure) ──────────────────────────────────────────────

#[test]
fn a_first_fire_is_kept_and_recorded_and_a_second_is_dropped() {
    let engine = engine_with(vec![chain(ONCE, true, vec![], vec![])]);
    let mut spent = HashSet::new();

    let first = apply_once_gate(vec![(ONCE, bump("a"))], vec![], &engine, &mut spent);
    assert_eq!(first.actions.len(), 1, "first fire runs");
    assert!(first.dropped_chain_ids.is_empty());
    assert!(spent.contains(&ONCE), "and is recorded");

    let second = apply_once_gate(vec![(ONCE, bump("a"))], vec![], &engine, &mut spent);
    assert!(second.actions.is_empty(), "second fire is dropped");
    assert_eq!(second.dropped_chain_ids, vec![ONCE]);
}

#[test]
fn a_chain_that_is_not_once_is_never_dropped_or_recorded() {
    let engine = engine_with(vec![chain(PLAIN, false, vec![], vec![])]);
    let mut spent = HashSet::new();
    for _ in 0..3 {
        let g = apply_once_gate(vec![(PLAIN, bump("p"))], vec![], &engine, &mut spent);
        assert_eq!(g.actions.len(), 1);
    }
    assert!(spent.is_empty());
}

/// A spent once-chain in the middle of a mixed list: its actions go, and the
/// delays of the survivors stay index-aligned.
#[test]
fn dropping_a_middle_chain_keeps_the_delays_aligned() {
    let engine = engine_with(vec![
        chain(PLAIN, false, vec![], vec![]),
        chain(ONCE, true, vec![], vec![]),
    ]);
    let mut spent: HashSet<i64> = [ONCE].into_iter().collect();

    let g = apply_once_gate(
        vec![
            (PLAIN, bump("p1")),
            (ONCE, bump("o1")),
            (ONCE, bump("o2")),
            (PLAIN, bump("p2")),
        ],
        vec![10, 20, 30, 40],
        &engine,
        &mut spent,
    );
    assert_eq!(
        g.actions.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![PLAIN, PLAIN]
    );
    assert_eq!(g.action_delays, vec![10, 40], "p2 keeps its own delay");
    assert_eq!(
        g.dropped_chain_ids,
        vec![ONCE],
        "listed once, not per action"
    );
}

// ── execute_actions ─────────────────────────────────────────────────────

async fn fire(engine: &ChainEngine, ctx: &ExecutionContext, eid: u32, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(8);
    let resolved = engine.resolve_event(&go(), ctx);
    execute_actions(resolved, eid, 42, &tx, mgr, engine).await;
}

/// Once per entity: the same player cannot fire it twice, a second player
/// still fires it once.
#[tokio::test]
async fn a_once_chain_fires_once_per_entity() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let engine = engine_with(vec![chain(ONCE, true, vec![], vec![])]);
    let ctx = ExecutionContext::new();

    fire(&engine, &ctx, 1, &mut mgr).await;
    fire(&engine, &ctx, 1, &mut mgr).await;
    assert_eq!(
        counter(&mgr, 1, "c1008"),
        1,
        "player 1: once, then disarmed"
    );

    fire(&engine, &ctx, 2, &mut mgr).await;
    assert_eq!(counter(&mgr, 2, "c1008"), 1, "player 2 has its own arm");
}

/// Chains without the flag behave exactly as before: every event fires.
#[tokio::test]
async fn a_plain_chain_still_fires_every_time() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let engine = engine_with(vec![chain(PLAIN, false, vec![], vec![])]);
    let ctx = ExecutionContext::new();

    fire(&engine, &ctx, 1, &mut mgr).await;
    fire(&engine, &ctx, 1, &mut mgr).await;
    assert_eq!(counter(&mgr, 1, "c1009"), 2);
}

/// A matched trigger whose conditions fail does not reach the executor, so
/// it does not disarm the chain (unlike the 2009 `Event.fire`, which
/// unsubscribed before running the callback).
#[tokio::test]
async fn a_once_chain_whose_conditions_fail_stays_armed() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let gate = Condition::Counter {
        counter_name: "ready".to_string(),
        operator: ComparisonOp::Gte,
        value: 1,
    };
    let engine = engine_with(vec![chain(ONCE, true, vec![gate], vec![])]);

    let closed = ExecutionContext::new();
    fire(&engine, &closed, 1, &mut mgr).await;
    assert_eq!(counter(&mgr, 1, "c1008"), 0, "gate closed: nothing ran");
    assert!(
        mgr.get_entity(1).unwrap().fired_once_chains.is_empty(),
        "a failed condition must not spend the chain"
    );

    let mut open = ExecutionContext::new();
    open.set_param("counter_ready".to_string(), serde_json::json!(1));
    fire(&engine, &open, 1, &mut mgr).await;
    fire(&engine, &open, 1, &mut mgr).await;
    assert_eq!(
        counter(&mgr, 1, "c1008"),
        1,
        "fires on the first open event only"
    );
}

/// A delayed action: the first fire schedules it, the second schedules
/// nothing.
#[tokio::test]
async fn a_delayed_once_action_is_scheduled_only_on_the_first_fire() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let engine = engine_with(vec![chain(ONCE, true, vec![], vec![5_000])]);
    let ctx = ExecutionContext::new();

    fire(&engine, &ctx, 1, &mut mgr).await;
    assert_eq!(
        mgr.pending_content_actions.get(&1).map(|q| q.len()),
        Some(1)
    );
    fire(&engine, &ctx, 1, &mut mgr).await;
    assert_eq!(
        mgr.pending_content_actions.get(&1).map(|q| q.len()),
        Some(1),
        "the second fire must not queue the delayed action again"
    );
}
