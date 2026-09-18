//! Missions 681-686 (Hallway Controller chain: Mess Hall, Hallway01-05)
//! — chains 1085-1094 in `castle_cellblock_chains.sql`. See audit.md
//! row 15 (C02) and `work-packets.md`'s C02 scope.
//!
//! Two shapes repeat across this family:
//!
//! - **Single-guard hallways** (682/683/684/685 -> chains
//!   1088/1089/1090/1091): one guard kill directly completes the
//!   mission and accepts the next one. No counter needed.
//! - **Two-guard rooms** (681 Mess Hall, 686 Hallway05 -> chains
//!   1085/1086/1087 and 1092/1093/1094): each guard kill runs an
//!   *increment* chain that bumps a named counter; a separate
//!   *completion* chain (with an OR trigger on both guard tags) fires
//!   the same event and checks `counter gte target - 1`. Because
//!   `resolve_event` evaluates ALL matching chains' conditions against
//!   the PRE-increment counter value (conditions resolve before any
//!   chain's actions execute — see `ChainEngine::resolve_event`), the
//!   completion chain's threshold is seeded one below the true target:
//!   on the second kill the counter is still 1 (from the first kill's
//!   increment), so `gte 1` passes. Mirrors the `mission_687.rs`
//!   chain_1103 pattern (barracks 3-guard counter, `gte 2`) exactly —
//!   `mission_687.rs` is this suite's counter-pattern fixture per
//!   `work-packets.md`.
//!
//! The increment and completion chains for a given room fire on the
//! SAME trigger event (either guard's death, once the counter clears
//! the threshold) and therefore compete for ordering within one
//! `resolve_event` call. `ChainEngine::register_chain` sorts each
//! trigger's chain bucket by priority (descending, stable). Commit
//! `a51a10d` bumped the increment chains (1085/1086/1092/1093) from
//! priority 0 to priority 1, keeping the completion chains
//! (1087/1094) at priority 0, specifically so the increment's
//! `increment_counter` always resolves before the completion's
//! `reset_counter` on the same event — equal priority left the order
//! undefined and could leak a non-zero counter into the next mission's
//! tracking. The tests at the bottom of this file pin that ordering
//! and include a control that reproduces the pre-fix bug shape.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{Trigger, TriggerEvent, TriggerType};

use super::super::engine_loader::{load_chain_expansions_for_test, load_single_chain_for_test};
use crate::test_support::require_db_or_skip;

// ── Single-guard hallway chains (1088-1091) ─────────────────────────────

/// Shared shape for chains 1088/1089/1090/1091: killing the room's one
/// tagged guard while `mission_id` is active must resolve exactly one
/// `CompleteMission(mission_id)` and one `AcceptMission(next_mission_id)`.
async fn assert_single_guard_completes_and_accepts(
    chain_id: i64,
    guard_tag: &str,
    mission_id: i32,
    next_mission_id: i32,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));
    ctx.set_param(
        format!("mission_{mission_id}_status"),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id
                && matches!(action, Action::CompleteMission { mission_id: m } if *m == mission_id)
        })
        .count();
    assert_eq!(
        completes, 1,
        "chain {chain_id} must resolve exactly one CompleteMission({mission_id}) \
         on {guard_tag} death while {mission_id} is active; got {completes}. \
         Resolved: {:?}",
        resolved.actions,
    );

    let accepts = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id
                && matches!(action, Action::AcceptMission { mission_id: m } if *m == next_mission_id)
        })
        .count();
    assert_eq!(
        accepts, 1,
        "chain {chain_id} must resolve exactly one AcceptMission({next_mission_id}) \
         alongside completing {mission_id}; got {accepts}. Resolved: {:?}",
        resolved.actions,
    );
}

/// Negative pin shared by 1088/1089/1090/1091: the guard's death must
/// not complete/accept anything while the gating mission isn't active
/// (e.g. the player hasn't reached this hallway yet, or already passed
/// it and the mission is `completed`).
async fn assert_single_guard_does_not_fire_when_mission_not_active(chain_id: i64, guard_tag: &str) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));
    // mission_status defaults to "not_active" when the param is unset.

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .count();
    assert_eq!(
        chain_actions, 0,
        "chain {chain_id} must NOT fire while its gating mission isn't \
         active; got {chain_actions} actions",
    );
}

#[tokio::test]
async fn chain_1088_kills_hallway01_guard_completes_682_accepts_683() {
    assert_single_guard_completes_and_accepts(1088, "Hallway01_Guard", 682, 683).await;
}

#[tokio::test]
async fn chain_1088_does_not_fire_when_682_not_active() {
    assert_single_guard_does_not_fire_when_mission_not_active(1088, "Hallway01_Guard").await;
}

#[tokio::test]
async fn chain_1089_kills_hallway02_guard_completes_683_accepts_684() {
    assert_single_guard_completes_and_accepts(1089, "Hallway02_Guard", 683, 684).await;
}

#[tokio::test]
async fn chain_1089_does_not_fire_when_683_not_active() {
    assert_single_guard_does_not_fire_when_mission_not_active(1089, "Hallway02_Guard").await;
}

#[tokio::test]
async fn chain_1090_kills_hallway03_guard_completes_684_accepts_685() {
    assert_single_guard_completes_and_accepts(1090, "Hallway03_Guard", 684, 685).await;
}

#[tokio::test]
async fn chain_1090_does_not_fire_when_684_not_active() {
    assert_single_guard_does_not_fire_when_mission_not_active(1090, "Hallway03_Guard").await;
}

#[tokio::test]
async fn chain_1091_kills_hallway04_guard_completes_685_accepts_686() {
    assert_single_guard_completes_and_accepts(1091, "Hallway04_Guard", 685, 686).await;
}

#[tokio::test]
async fn chain_1091_does_not_fire_when_685_not_active() {
    assert_single_guard_does_not_fire_when_mission_not_active(1091, "Hallway04_Guard").await;
}

// ── Two-guard counter chains: increment half (1085/1086, 1092/1093) ────

/// Shared shape for the increment chains: killing one of the room's two
/// tagged guards while `mission_id` is active bumps `counter_name` by 1
/// and does nothing else.
async fn assert_increment_chain_fires(
    chain_id: i64,
    guard_tag: &str,
    mission_id: i32,
    counter_name: &str,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));
    ctx.set_param(
        format!("mission_{mission_id}_status"),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let increments = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id
                && matches!(
                    action,
                    Action::IncrementCounter { counter_name: n, amount: 1 } if n == counter_name
                )
        })
        .count();
    assert_eq!(
        increments, 1,
        "chain {chain_id} must resolve exactly one IncrementCounter({counter_name}, \
         amount=1) on {guard_tag} death while {mission_id} is active; got \
         {increments}. Resolved: {:?}",
        resolved.actions,
    );
}

/// Negative pin shared by the increment chains: no increment while the
/// gating mission isn't active.
async fn assert_increment_chain_does_not_fire_when_mission_not_active(
    chain_id: i64,
    guard_tag: &str,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .count();
    assert_eq!(
        chain_actions, 0,
        "chain {chain_id} must NOT increment while its gating mission \
         isn't active; got {chain_actions} actions",
    );
}

#[tokio::test]
async fn chain_1085_kills_messhall_guard1_increments_messhall_kills() {
    assert_increment_chain_fires(1085, "MessHall_Guard1", 681, "messhall_kills").await;
}

#[tokio::test]
async fn chain_1085_does_not_fire_when_681_not_active() {
    assert_increment_chain_does_not_fire_when_mission_not_active(1085, "MessHall_Guard1").await;
}

#[tokio::test]
async fn chain_1086_kills_messhall_guard2_increments_messhall_kills() {
    assert_increment_chain_fires(1086, "MessHall_Guard2", 681, "messhall_kills").await;
}

#[tokio::test]
async fn chain_1086_does_not_fire_when_681_not_active() {
    assert_increment_chain_does_not_fire_when_mission_not_active(1086, "MessHall_Guard2").await;
}

#[tokio::test]
async fn chain_1092_kills_hallway05_guard1_increments_hallway05_kills() {
    assert_increment_chain_fires(1092, "Hallway05_Guard1", 686, "hallway05_kills").await;
}

#[tokio::test]
async fn chain_1092_does_not_fire_when_686_not_active() {
    assert_increment_chain_does_not_fire_when_mission_not_active(1092, "Hallway05_Guard1").await;
}

#[tokio::test]
async fn chain_1093_kills_hallway05_guard2_increments_hallway05_kills() {
    assert_increment_chain_fires(1093, "Hallway05_Guard2", 686, "hallway05_kills").await;
}

#[tokio::test]
async fn chain_1093_does_not_fire_when_686_not_active() {
    assert_increment_chain_does_not_fire_when_mission_not_active(1093, "Hallway05_Guard2").await;
}

// ── Two-guard counter chains: completion half (1087, 1094) ─────────────
//
// Loaded with `load_chain_expansions_for_test` because each completion
// chain has two trigger rows (one per guard tag) — a single-expansion
// load would silently miss the OR shape and only test one guard's
// death path (the exact loader bug the seed comments call out: "the
// loader silently dropped the 2nd row").

/// Shared shape for the completion chains: on the SECOND guard kill,
/// the pre-increment counter value is `target - 1` (the first kill's
/// increment already ran), so `counter gte target - 1` passes and the
/// chain resolves `CompleteMission`, `AcceptMission`, and
/// `ResetCounter`.
async fn assert_completion_chain_fires_on_threshold_kill(
    chain_id: i32,
    expected_expansions: usize,
    guard_tag: &str,
    mission_id: i32,
    next_mission_id: i32,
    counter_name: &str,
    pre_increment_counter_value: i64,
) {
    let pool = require_db_or_skip!();
    let expansions = load_chain_expansions_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"));
    assert_eq!(
        expansions.len(),
        expected_expansions,
        "chain {chain_id} must materialize {expected_expansions} in-memory \
         Chain(s), one per OR-trigger guard tag; got {}",
        expansions.len(),
    );

    let mut engine = ChainEngine::new();
    for chain in expansions {
        engine.register_chain(chain);
    }

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));
    ctx.set_param(
        format!("mission_{mission_id}_status"),
        serde_json::json!("active"),
    );
    ctx.set_param(
        format!("counter_{counter_name}"),
        serde_json::json!(pre_increment_counter_value),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id as i64
                && matches!(action, Action::CompleteMission { mission_id: m } if *m == mission_id)
        })
        .count();
    assert_eq!(
        completes, 1,
        "chain {chain_id} must resolve exactly one CompleteMission({mission_id}) \
         when counter_{counter_name} >= target - 1 on {guard_tag} death; got \
         {completes}. Resolved: {:?}",
        resolved.actions,
    );

    let accepts = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id as i64
                && matches!(action, Action::AcceptMission { mission_id: m } if *m == next_mission_id)
        })
        .count();
    assert_eq!(
        accepts, 1,
        "chain {chain_id} must resolve exactly one AcceptMission({next_mission_id}); \
         got {accepts}. Resolved: {:?}",
        resolved.actions,
    );

    let resets = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id as i64
                && matches!(action, Action::ResetCounter { counter_name: n } if n == counter_name)
        })
        .count();
    assert_eq!(
        resets, 1,
        "chain {chain_id} must resolve exactly one ResetCounter({counter_name}); \
         got {resets}. Resolved: {:?}",
        resolved.actions,
    );
}

/// Negative pin shared by the completion chains: on the FIRST guard
/// kill, the counter is still 0 (nothing has incremented it yet at
/// resolve time), so `gte target - 1` (>= 1) fails and the completion
/// chain must NOT fire. This is the load-bearing regression guard
/// against lowering the threshold (which would complete the mission on
/// the first kill instead of the second).
async fn assert_completion_chain_does_not_fire_on_first_kill(
    chain_id: i32,
    guard_tag: &str,
    mission_id: i32,
    counter_name: &str,
) {
    let pool = require_db_or_skip!();
    let expansions = load_chain_expansions_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"));

    let mut engine = ChainEngine::new();
    for chain in expansions {
        engine.register_chain(chain);
    }

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(guard_tag));
    ctx.set_param(
        format!("mission_{mission_id}_status"),
        serde_json::json!("active"),
    );
    ctx.set_param(format!("counter_{counter_name}"), serde_json::json!(0));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let completes = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == chain_id as i64
                && matches!(action, Action::CompleteMission { mission_id: m } if *m == mission_id)
        })
        .count();
    assert_eq!(
        completes, 0,
        "chain {chain_id} must NOT resolve CompleteMission on the first \
         guard kill (counter at 0 < gte threshold); got {completes} \
         actions. Resolved: {:?}",
        resolved.actions,
    );
}

#[tokio::test]
async fn chain_1087_completes_681_and_accepts_682_on_second_messhall_kill() {
    assert_completion_chain_fires_on_threshold_kill(
        1087,
        2,
        "MessHall_Guard2",
        681,
        682,
        "messhall_kills",
        1,
    )
    .await;
}

#[tokio::test]
async fn chain_1087_does_not_complete_681_on_first_messhall_kill() {
    assert_completion_chain_does_not_fire_on_first_kill(
        1087,
        "MessHall_Guard1",
        681,
        "messhall_kills",
    )
    .await;
}

#[tokio::test]
async fn chain_1094_completes_686_and_accepts_687_on_second_hallway05_kill() {
    assert_completion_chain_fires_on_threshold_kill(
        1094,
        2,
        "Hallway05_Guard2",
        686,
        687,
        "hallway05_kills",
        1,
    )
    .await;
}

#[tokio::test]
async fn chain_1094_does_not_complete_686_on_first_hallway05_kill() {
    assert_completion_chain_does_not_fire_on_first_kill(
        1094,
        "Hallway05_Guard1",
        686,
        "hallway05_kills",
    )
    .await;
}

// ── Priority-ordering invariant (a51a10d) ───────────────────────────────

/// Find the single-expansion `Chain` matching an `OnEntityDeath` trigger
/// for `entity_tag`, panicking with a descriptive message if none of
/// `expansions` matches. Used to pick out one guard-tag branch of a
/// multi-trigger completion chain for the ordering tests below, which
/// need to register exactly one increment chain against exactly one
/// completion-chain expansion (not all of them) to keep the resolved
/// action list small enough to reason about by index.
fn find_entity_death_expansion(
    expansions: Vec<cimmeria_content_engine::chain::Chain>,
    entity_tag: &str,
) -> cimmeria_content_engine::chain::Chain {
    expansions
        .into_iter()
        .find(|c| {
            matches!(
                &c.trigger,
                Trigger::OnEntityDeath { entity_tag: Some(tag), .. } if tag == entity_tag
            )
        })
        .unwrap_or_else(|| panic!("no OnEntityDeath expansion found for tag {entity_tag}"))
}

/// Positive ordering pin: on the second Hallway05 guard kill, increment
/// chain 1093 (seeded priority 1) must resolve its `increment_counter`
/// action BEFORE completion chain 1094's `reset_counter` action.
///
/// `ChainEngine::register_chain` re-sorts its trigger bucket by
/// priority (descending, stable) on every call. This test deliberately
/// registers the completion chain FIRST and the increment chain
/// SECOND — the opposite of numeric chain-id order — to prove the
/// final resolved order comes from `priority`, not registration order.
/// (The live DB loader happens to load chains `ORDER BY chain_id`, but
/// nothing in `ChainEngine` depends on that — see
/// `crates/services/src/cell/content/engine_loader.rs`.)
#[tokio::test]
async fn hallway05_increment_resolves_before_completion_reset_regardless_of_registration_order() {
    let pool = require_db_or_skip!();
    let increment_chain = load_single_chain_for_test(&pool, 1093)
        .await
        .expect("DB query for chain 1093 must succeed")
        .expect("chain 1093 must exist in seeded content_chains");
    let completion_expansions = load_chain_expansions_for_test(&pool, 1094)
        .await
        .expect("DB query for chain 1094 must succeed");
    let completion_guard2 = find_entity_death_expansion(completion_expansions, "Hallway05_Guard2");

    // Pin the priorities the ordering guarantee depends on, so a
    // regression shows up here with a clear message instead of as a
    // confusing index mismatch below.
    assert_eq!(
        increment_chain.priority, 1,
        "chain 1093 must be seeded at priority 1 (a51a10d) — without \
         this, the ordering assertion below is not meaningful"
    );
    assert_eq!(
        completion_guard2.priority, 0,
        "chain 1094 must be seeded at priority 0"
    );

    let mut engine = ChainEngine::new();
    engine.register_chain(completion_guard2); // adverse order: completion first
    engine.register_chain(increment_chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Hallway05_Guard2"),
    );
    ctx.set_param(
        "mission_686_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("counter_hallway05_kills".to_string(), serde_json::json!(1));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let increment_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1093 && matches!(a, Action::IncrementCounter { .. }))
        .expect("chain 1093's IncrementCounter action must resolve");
    let reset_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1094 && matches!(a, Action::ResetCounter { .. }))
        .expect("chain 1094's ResetCounter action must resolve");

    assert!(
        increment_idx < reset_idx,
        "chain 1093's increment_counter (priority 1) must resolve before \
         chain 1094's reset_counter (priority 0), even though the \
         completion chain was registered first; got increment at index \
         {increment_idx}, reset at index {reset_idx} in {:?}. A failure \
         here means the seed priorities for 1092-1094 have regressed \
         toward equal — see the a51a10d control test below for what \
         that looks like.",
        resolved.actions,
    );
}

/// FAILING-CONTROL for the ordering invariant above. Reproduces the
/// exact `a51a10d` bug shape to prove the positive test above is
/// capable of catching a regression, rather than merely asserting
/// today's happy path.
///
/// Loads the SAME real chains (1093 increment, 1094's Hallway05_Guard2
/// completion expansion — identical actions/conditions to production,
/// not a hand-built synthetic fixture) and manually overrides both
/// `.priority` fields to 0, simulating a revert of the a51a10d fix
/// (which bumped 1085/1086/1092/1093 from priority 0 to 1). Registered
/// in the same adverse order as the positive test (completion first).
///
/// With equal priority, `sort_by_key(Reverse(priority))` is a stable
/// sort, so it preserves registration order instead of correcting it:
/// the completion chain's `reset_counter` now resolves BEFORE the
/// increment chain's `increment_counter`. That is precisely the commit
/// message's described bug: "Equal priority left ordering undefined
/// and could leak a non-zero counter into the next mission" — the
/// counter is reset to 0, then immediately re-incremented to 1 by the
/// chain that should have run first, leaving a stale non-zero count
/// that bleeds into the next room's kill tracking.
#[tokio::test]
async fn equal_priority_control_reproduces_a51a10d_bug_shape_if_seed_priorities_regress() {
    let pool = require_db_or_skip!();
    let mut increment_chain = load_single_chain_for_test(&pool, 1093)
        .await
        .expect("DB query for chain 1093 must succeed")
        .expect("chain 1093 must exist in seeded content_chains");
    let completion_expansions = load_chain_expansions_for_test(&pool, 1094)
        .await
        .expect("DB query for chain 1094 must succeed");
    let mut completion_guard2 =
        find_entity_death_expansion(completion_expansions, "Hallway05_Guard2");

    // Simulate the pre-a51a10d seed: both chains at equal priority.
    increment_chain.priority = 0;
    completion_guard2.priority = 0;

    let mut engine = ChainEngine::new();
    engine.register_chain(completion_guard2); // same adverse order as the positive test
    engine.register_chain(increment_chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Hallway05_Guard2"),
    );
    ctx.set_param(
        "mission_686_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param("counter_hallway05_kills".to_string(), serde_json::json!(1));

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let increment_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1093 && matches!(a, Action::IncrementCounter { .. }))
        .expect("chain 1093's IncrementCounter action must resolve");
    let reset_idx = resolved
        .actions
        .iter()
        .position(|(id, a)| *id == 1094 && matches!(a, Action::ResetCounter { .. }))
        .expect("chain 1094's ResetCounter action must resolve");

    assert!(
        reset_idx < increment_idx,
        "control: with priorities forced equal (simulating a revert of \
         a51a10d) and the completion chain registered first, \
         reset_counter must resolve BEFORE increment_counter — \
         reproducing the exact leaked-counter bug shape the commit \
         fixed. This proves the positive ordering test above would fail \
         (and catch the regression) if the seed priorities ever \
         collapsed back to equal. Got reset at index {reset_idx}, \
         increment at index {increment_idx} in {:?}",
        resolved.actions,
    );
}
