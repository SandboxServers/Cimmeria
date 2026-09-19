//! Mission 639, step 2144 ("Defend yourself from the drone!") — the
//! dual-objective take-cover/kill-the-drone gate (C05). Replaces the old
//! `cover_demo` proof-of-wiring chain (1035) with real gameplay: step
//! 2144 requires BOTH objective 2482 (kill) and 2484 (cover) before
//! advancing to step 2343 (use the Ambernol cure).
//!
//! Four chains implement the three-way split:
//!   - 1033: drone killed while cover pending → `CompleteObjective(2482)`
//!   - 1131: drone killed while cover already taken → `AdvanceStep(2343)`
//!   - 1132: cover taken while kill pending → `CompleteObjective(2484)` +
//!     `PlaySequence(10014)` (hide the TakeCoverIndicator)
//!   - 1133: cover taken while kill already done → `AdvanceStep(2343)` +
//!     `PlaySequence(10014)`
//!
//! `Action::CompleteObjective` is never used for the SECOND (last) of the
//! two required objectives — doing so would trip
//! `cell::missions::complete_objective`'s all-required-done auto-complete
//! and end mission 639 outright, skipping step 2343 (the same trap
//! `mission_688.rs`'s chain 1107/1109 tests document for step 2356).
//! `Action::AdvanceStep` is used instead for whichever event resolves the
//! pair, since `advance_step`'s own implementation completes the
//! remaining objective via the raw `MissionInstance::complete_objective`
//! method, bypassing the wrapper's auto-complete-mission check.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Loads all four cover/kill chains into one engine — used by the
/// sequence-level tests that need to prove only ONE of the four ever
/// resolves for a given event, not just that the targeted chain resolves
/// in isolation (which per-chain tests below already cover).
async fn load_all_four(pool: &sqlx::PgPool) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for id in [1033, 1131, 1132, 1133] {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

fn drone_death_event(
    step_2144_status: &str,
    obj_2484_status: &str,
) -> (ExecutionContext, TriggerEvent) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("ArmYourself_PrisonerRetrievalUnit"),
    );
    ctx.set_param(
        "mission_639_step_2144_status".to_string(),
        serde_json::json!(step_2144_status),
    );
    ctx.set_param(
        "mission_639_obj_2484_status".to_string(),
        serde_json::json!(obj_2484_status),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (ctx, event)
}

fn cover_entered_event(
    cover_set_id: i64,
    step_2144_status: &str,
    obj_2482_status: &str,
) -> (ExecutionContext, TriggerEvent) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(cover_set_id));
    ctx.set_param(
        "mission_639_step_2144_status".to_string(),
        serde_json::json!(step_2144_status),
    );
    ctx.set_param(
        "mission_639_obj_2482_status".to_string(),
        serde_json::json!(obj_2482_status),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCover,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (ctx, event)
}

// ── Chain 1033: drone killed, cover pending ─────────────────────────────

#[tokio::test]
async fn chain_1033_completes_kill_objective_when_cover_pending() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1033)
        .await
        .expect("DB query for chain 1033 must succeed")
        .expect("chain 1033 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = drone_death_event("active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1033 { Some(a) } else { None })
        .collect();

    assert_eq!(
        actions.len(),
        1,
        "chain 1033 must resolve exactly one action when cover is pending; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteObjective {
                mission_id: 639,
                objective_id: 2482
            }
        ),
        "chain 1033 must complete the kill objective (2482), not advance the step \
         (advancing here would be the auto-complete-mission trap if cover were also \
         complete, but here cover is still pending so it must never touch AdvanceStep \
         either); got {:?}",
        actions[0]
    );
}

#[tokio::test]
async fn chain_1033_does_not_fire_when_cover_already_taken() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1033)
        .await
        .expect("DB query for chain 1033 must succeed")
        .expect("chain 1033 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    // Cover already completed — chain 1131 should be the one that fires
    // instead (asserted separately below). Chain 1033's `neq completed`
    // gate must reject.
    let (ctx, event) = drone_death_event("active", "completed");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1033)
        .count();
    assert_eq!(
        n, 0,
        "chain 1033 must NOT fire once cover (2484) is already completed \
         (chain 1131 owns that branch); got {n} actions"
    );
}

#[tokio::test]
async fn chain_1033_does_not_fire_when_step_2144_inactive() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1033)
        .await
        .expect("DB query for chain 1033 must succeed")
        .expect("chain 1033 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = drone_death_event("not_active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1033)
        .count();
    assert_eq!(
        n, 0,
        "chain 1033 must NOT fire while step 2144 is not active; got {n} actions"
    );
}

#[tokio::test]
async fn chain_1033_does_not_refire_once_kill_already_completed() {
    // Self-completion guard (found in review): a second `entity_dead_tag`
    // event for the same tag (e.g. a respawn/relog edge) must not re-run
    // `CompleteObjective(2482)` once 2482 is already completed -- chain
    // 1033 must check its OWN target's status, not just the other
    // objective's.
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1033)
        .await
        .expect("DB query for chain 1033 must succeed")
        .expect("chain 1033 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (mut ctx, mut event) = drone_death_event("active", "active");
    ctx.set_param(
        "mission_639_obj_2482_status".to_string(),
        serde_json::json!("completed"),
    );
    event.params = ctx.params.clone();
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1033)
        .count();
    assert_eq!(
        n, 0,
        "chain 1033 must NOT re-fire once its own target (2482) is already \
         completed, even with cover still pending; got {n} actions"
    );
}

// ── Chain 1131: drone killed, cover already taken (second objective) ───

#[tokio::test]
async fn chain_1131_advances_step_when_kill_is_second() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1131)
        .await
        .expect("DB query for chain 1131 must succeed")
        .expect("chain 1131 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = drone_death_event("active", "completed");
    let resolved = engine.resolve_event(&event, &ctx);
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1131 { Some(a) } else { None })
        .collect();

    assert_eq!(
        actions.len(),
        1,
        "chain 1131 must resolve exactly one action when kill is the second \
         objective satisfied; got {actions:?}"
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 639,
                step_id: 2343
            }
        ),
        "chain 1131 must advance to step 2343 directly — NOT call \
         CompleteObjective(2482), which would trip the auto-complete-mission \
         trap now that both objectives are satisfied; got {:?}",
        actions[0]
    );
}

#[tokio::test]
async fn chain_1131_does_not_fire_when_cover_still_pending() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1131)
        .await
        .expect("DB query for chain 1131 must succeed")
        .expect("chain 1131 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = drone_death_event("active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1131)
        .count();
    assert_eq!(
        n, 0,
        "chain 1131 must NOT fire while cover (2484) is still pending \
         (chain 1033 owns that branch); got {n} actions"
    );
}

// ── Chain 1132: cover taken, kill pending ───────────────────────────────

#[tokio::test]
async fn chain_1132_completes_cover_objective_when_kill_pending() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1132)
        .await
        .expect("DB query for chain 1132 must succeed")
        .expect("chain 1132 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = cover_entered_event(1381, "active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1132 { Some(a) } else { None })
        .collect();

    assert_eq!(
        actions.len(),
        2,
        "chain 1132 must resolve CompleteObjective + PlaySequence(10014) when \
         kill is pending; got {actions:?}"
    );
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::CompleteObjective {
                mission_id: 639,
                objective_id: 2484
            }
        )),
        "chain 1132 must complete the cover objective (2484); got {actions:?}"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::PlaySequence { sequence_id: 10014 })),
        "chain 1132 must hide the TakeCoverIndicator (sequence 10014); got {actions:?}"
    );
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::AdvanceStep { .. })),
        "chain 1132 must NOT advance the step while kill is still pending; got {actions:?}"
    );
}

#[tokio::test]
async fn chain_1132_does_not_fire_when_kill_already_done() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1132)
        .await
        .expect("DB query for chain 1132 must succeed")
        .expect("chain 1132 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = cover_entered_event(1381, "active", "completed");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1132)
        .count();
    assert_eq!(
        n, 0,
        "chain 1132 must NOT fire once kill (2482) is already completed \
         (chain 1133 owns that branch); got {n} actions"
    );
}

#[tokio::test]
async fn chain_1132_does_not_refire_on_cover_reentry_once_already_completed() {
    // Self-completion guard (found in review): `player_entered_cover` fires
    // on every proximity enter/leave edge (once=false), and
    // `Action::PlaySequence` sends unconditionally with no dedup. A player
    // who leans out of cover and back in before killing the drone must not
    // re-trigger chain 1132 (and resend PlaySequence(10014)) once 2484 is
    // already completed -- chain 1132 must check its OWN target's status,
    // not just the kill objective's.
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1132)
        .await
        .expect("DB query for chain 1132 must succeed")
        .expect("chain 1132 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (mut ctx, mut event) = cover_entered_event(1381, "active", "active");
    ctx.set_param(
        "mission_639_obj_2484_status".to_string(),
        serde_json::json!("completed"),
    );
    event.params = ctx.params.clone();
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1132)
        .count();
    assert_eq!(
        n, 0,
        "chain 1132 must NOT re-fire (and must not resend PlaySequence(10014)) \
         on cover re-entry once its own target (2484) is already completed, \
         even with the kill still pending; got {n} actions"
    );
}

#[tokio::test]
async fn chain_1132_does_not_fire_for_a_different_cover_set() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1132)
        .await
        .expect("DB query for chain 1132 must succeed")
        .expect("chain 1132 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    // Some other cover set entirely (e.g. a corridor lean-point elsewhere
    // in the space) must not trigger the med-station desk's objective.
    let (ctx, event) = cover_entered_event(42, "active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1132)
        .count();
    assert_eq!(
        n, 0,
        "chain 1132 must only match cover_set_id 1381 (the med-station desk), \
         not an unrelated cover set; got {n} actions"
    );
}

// ── Chain 1133: cover taken, kill already done (second objective) ──────

#[tokio::test]
async fn chain_1133_advances_step_when_cover_is_second() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1133)
        .await
        .expect("DB query for chain 1133 must succeed")
        .expect("chain 1133 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = cover_entered_event(1381, "active", "completed");
    let resolved = engine.resolve_event(&event, &ctx);
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == 1133 { Some(a) } else { None })
        .collect();

    assert_eq!(
        actions.len(),
        2,
        "chain 1133 must resolve AdvanceStep + PlaySequence(10014) when cover is \
         the second objective satisfied; got {actions:?}"
    );
    assert!(
        actions.iter().any(|a| matches!(
            a,
            Action::AdvanceStep {
                mission_id: 639,
                step_id: 2343
            }
        )),
        "chain 1133 must advance to step 2343 directly — NOT call \
         CompleteObjective(2484), which would trip the auto-complete-mission \
         trap; got {actions:?}"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::PlaySequence { sequence_id: 10014 })),
        "chain 1133 must still hide the TakeCoverIndicator; got {actions:?}"
    );
}

#[tokio::test]
async fn chain_1133_does_not_fire_when_kill_still_pending() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1133)
        .await
        .expect("DB query for chain 1133 must succeed")
        .expect("chain 1133 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ctx, event) = cover_entered_event(1381, "active", "active");
    let resolved = engine.resolve_event(&event, &ctx);
    let n = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1133)
        .count();
    assert_eq!(
        n, 0,
        "chain 1133 must NOT fire while kill (2482) is still pending \
         (chain 1132 owns that branch); got {n} actions"
    );
}

// ── Sequence-level: both orderings, using all four chains together ─────

/// Cover first, then kill. Proves the three-way split end to end: taking
/// cover alone only ticks the cover objective (chain 1132), and only the
/// SECOND event (the kill) advances the step (chain 1131) — with chains
/// 1033/1133 never firing in this ordering.
#[tokio::test]
async fn cover_then_kill_advances_only_on_the_second_event() {
    let pool = require_db_or_skip!();
    let engine = load_all_four(&pool).await;

    // Step 1: player takes cover. Kill not yet done.
    let (ctx1, event1) = cover_entered_event(1381, "active", "active");
    let resolved1 = engine.resolve_event(&event1, &ctx1);
    let fired1: Vec<i64> = resolved1
        .actions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| [1033, 1131, 1132, 1133].contains(id))
        .collect();
    assert_eq!(
        fired1,
        // Chain 1132 carries two actions (complete_objective + the
        // indicator-hide play_sequence), so it contributes two entries
        // to the flat (chain_id, Action) list -- this is asserting
        // "only chain 1132 fired," not "exactly one action resolved."
        vec![1132, 1132],
        "only chain 1132 (complete cover objective) must fire on the cover-first \
         event; got chains {fired1:?}"
    );
    assert!(
        !resolved1
            .actions
            .iter()
            .any(|(_, a)| matches!(a, Action::AdvanceStep { .. })),
        "the cover-first event must not advance the step yet"
    );

    // Step 2: drone dies. Cover is now complete (as chain 1132 above just
    // marked it) — the caller is responsible for reflecting that in the
    // next event's context, mirroring how `populate_mission_context` would
    // re-derive it from the persisted mission state in production.
    let (ctx2, event2) = drone_death_event("active", "completed");
    let resolved2 = engine.resolve_event(&event2, &ctx2);
    let fired2: Vec<i64> = resolved2
        .actions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| [1033, 1131, 1132, 1133].contains(id))
        .collect();
    assert_eq!(
        fired2,
        vec![1131],
        "only chain 1131 (advance to 2343) must fire on the kill-second event; \
         got chains {fired2:?}"
    );
}

/// Kill first, then cover — the mirror ordering.
#[tokio::test]
async fn kill_then_cover_advances_only_on_the_second_event() {
    let pool = require_db_or_skip!();
    let engine = load_all_four(&pool).await;

    // Step 1: drone dies. Cover not yet taken.
    let (ctx1, event1) = drone_death_event("active", "active");
    let resolved1 = engine.resolve_event(&event1, &ctx1);
    let fired1: Vec<i64> = resolved1
        .actions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| [1033, 1131, 1132, 1133].contains(id))
        .collect();
    assert_eq!(
        fired1,
        vec![1033],
        "only chain 1033 (complete kill objective) must fire on the kill-first \
         event; got chains {fired1:?}"
    );

    // Step 2: player takes cover. Kill is now complete.
    let (ctx2, event2) = cover_entered_event(1381, "active", "completed");
    let resolved2 = engine.resolve_event(&event2, &ctx2);
    let fired2: Vec<i64> = resolved2
        .actions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| [1033, 1131, 1132, 1133].contains(id))
        .collect();
    assert_eq!(
        fired2,
        // Chain 1133 also carries two actions (advance_step + the
        // indicator-hide play_sequence) -- same shape as chain 1132
        // above.
        vec![1133, 1133],
        "only chain 1133 (advance to 2343) must fire on the cover-second event; \
         got chains {fired2:?}"
    );

    // The indicator-hide sequence must resolve exactly once across the
    // whole two-event sequence, not twice (it's carried by BOTH 1132 and
    // 1133, but those two are mutually exclusive on the same event).
    let hide_count = resolved1
        .actions
        .iter()
        .chain(resolved2.actions.iter())
        .filter(|(_, a)| matches!(a, Action::PlaySequence { sequence_id: 10014 }))
        .count();
    assert_eq!(
        hide_count, 1,
        "PlaySequence(10014) must resolve exactly once across the whole \
         kill-then-cover sequence; got {hide_count}"
    );
}
