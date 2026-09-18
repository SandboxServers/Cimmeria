//! End-to-end coverage for `content_actions.delay_ms > 0` scheduling
//! (C08a): `execute_actions` must run `delay_ms == 0` actions inline and
//! queue the rest, and `deferred_content_action_tick` must fire only the
//! entries whose deadline has passed, leaving the rest queued.
//!
//! Time is controlled the same way `cell::service::ticks::pending_holster`'s
//! tests do — no real sleeping. A "ready" entry is stamped with a `fire_at`
//! already in the past; a "not ready" entry is stamped far in the future.
//! `deferred_content_action_tick` reads `Instant::now()` internally, so
//! these tests lean on that call happening microseconds after the past
//! stamp was written, the same non-flaky assumption `pending_attack_tick`'s
//! own tests already rely on.

use super::*;
use crate::cell::space_manager::PendingContentAction;
use std::time::{Duration, Instant};

/// `execute_actions` must run every `delay_ms == 0` action synchronously,
/// in order, exactly as it did before C08a — this is the zero-delay
/// regression guard for the executor's own dispatch loop (see also the
/// per-family tests in sibling files, which all go through this same
/// `delay_ms == 0` path and would fail if the zero-delay case regressed).
/// A `delay_ms > 0` action in the SAME resolved list must NOT run
/// synchronously — it must be queued on `SpaceManager` instead.
#[tokio::test]
async fn execute_actions_runs_zero_delay_inline_and_queues_positive_delay() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: vec![0, 5_000],
        params: std::collections::HashMap::new(),
        actions: vec![
            (
                1161,
                Action::IncrementCounter {
                    counter_name: "immediate".to_string(),
                    amount: 1,
                },
            ),
            (
                1161,
                Action::IncrementCounter {
                    counter_name: "deferred".to_string(),
                    amount: 1,
                },
            ),
        ],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.counters.get("immediate"),
        Some(&1),
        "delay_ms == 0 action must run synchronously inside execute_actions"
    );
    assert!(
        !entity.counters.contains_key("deferred"),
        "delay_ms > 0 action must NOT run inline — it must be queued instead"
    );
    assert_eq!(
        mgr.pending_content_actions.get(&1).map(|q| q.len()),
        Some(1),
        "the delay_ms=5000 action must be queued against entity 1"
    );
}

/// `deferred_content_action_tick` fires only entries whose deadline has
/// elapsed and leaves the rest queued — the core "resume later without
/// blocking the tick" behavior C08a adds.
#[tokio::test]
async fn deferred_content_action_tick_fires_elapsed_and_skips_future() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let now = Instant::now();
    mgr.pending_content_actions.insert(
        1,
        vec![
            PendingContentAction {
                fire_at: now - Duration::from_secs(1),
                chain_id: 1161,
                action: Action::IncrementCounter {
                    counter_name: "ready".to_string(),
                    amount: 1,
                },
                player_id: 42,
                params: std::collections::HashMap::new(),
            },
            PendingContentAction {
                fire_at: now + Duration::from_secs(3600),
                chain_id: 1161,
                action: Action::IncrementCounter {
                    counter_name: "not_ready".to_string(),
                    amount: 1,
                },
                player_id: 42,
                params: std::collections::HashMap::new(),
            },
        ],
    );

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    deferred_content_action_tick(&tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.counters.get("ready"),
        Some(&1),
        "an action whose fire_at is in the past must fire on this tick"
    );
    assert!(
        !entity.counters.contains_key("not_ready"),
        "an action whose fire_at is still in the future must NOT fire"
    );
    assert_eq!(
        mgr.pending_content_actions.get(&1).map(|q| q.len()),
        Some(1),
        "the not-yet-ready entry must remain queued after the tick"
    );
}

/// Disconnect/leave-space before a deferred action's delay elapses must
/// drop the action — not fire it later. `SpaceManager::destroy_entity`
/// already scrubs `pending_content_actions` (see
/// `space_manager::deferred_content_actions` for that guard); this test
/// proves the end-to-end consequence: the tick genuinely executes nothing
/// for a torn-down entity, because there is nothing left to drain.
#[tokio::test]
async fn destroyed_entity_never_fires_its_deferred_action() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    mgr.pending_content_actions.insert(
        1,
        vec![PendingContentAction {
            fire_at: Instant::now() - Duration::from_secs(1),
            chain_id: 1161,
            action: Action::IncrementCounter {
                counter_name: "should_never_fire".to_string(),
                amount: 1,
            },
            player_id: 42,
            params: std::collections::HashMap::new(),
        }],
    );

    // Entity leaves before the tick ever runs.
    mgr.destroy_entity(1);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    deferred_content_action_tick(&tx, &mut mgr, &engine).await;

    assert!(
        mgr.pending_content_actions.is_empty(),
        "destroy_entity must have already scrubbed the queue"
    );
    // No panic and no observable side effect: the entity is gone, so
    // there's nothing to assert a counter against — the absence of a
    // panic in execute_one against a missing entity is itself the guard.
}
