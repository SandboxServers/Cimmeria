//! Mission 706 — "Power Behind the Throne" (Castle rebuild packet
//! CA08). Pins chains 1321-1323 in
//! `db/resources/Content/Seed/castle_706_708_chains.sql`.
//!
//! What each test would catch if the seed drifted:
//!
//! - `chain_1321_*` — the region key is matched with case-sensitive
//!   string equality against `point_sets.name`, so a typo'd
//!   `Castle.Throneroom` loads fine and then never fires, stranding the
//!   player on step 2411 with no way forward. The happy-path test fires
//!   a real `Castle.ThroneRoom` event; the negative pins the step gate
//!   so re-entering the room after advancing can't re-advance.
//! - `chain_1322_*` — the completion chain. The `mission_status 708 eq
//!   not_active` condition is doing two jobs (accept gate + tag
//!   disambiguation against chain 1344, which triggers on the same
//!   `Castle_AccessPanel` tag for mission 708's step 2415), so both
//!   negatives are load-bearing.
//! - `chain_1323_*` — interaction flags are in-memory only; without the
//!   restore the panel is scenery after a server restart.
//!
//! Resolve-only, deliberately: every action in this range is a mission
//! or interaction-flag verb whose executor arms are already pinned
//! elsewhere (`mission_688.rs` for the flag swap, `grant_xp.rs` /
//! `sgc_w1_move_entity.rs` for the execute-through pattern), and
//! `accept_mission` cannot be executed without a populated
//! `space_mgr.mission_defs` cache. Mission 708's `add_item` and
//! `start_minigame` chains ARE pushed through `execute_actions` — see
//! [`super::mission_708`].

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;
use crate::test_support::require_db_or_skip;

/// Quest-object glow — the only bit that makes template 147 (the Access
/// Panel, `interaction_type = 0`) clickable at all.
const GLOW: i64 = cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT;

async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains — a None here \
                 means the row is missing from castle_706_708_chains.sql or its \
                 trigger/action rows failed to convert"
            )
        });
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

fn actions_of(resolved: &ResolvedActions, chain_id: i64) -> Vec<&Action> {
    resolved
        .actions
        .iter()
        .filter_map(|(id, a)| if *id == chain_id { Some(a) } else { None })
        .collect()
}

/// Chain 1321 happy path: entering the Throne Room on step 2411 must
/// advance to 2412 and light the Access Panel, in that order.
#[tokio::test]
async fn chain_1321_advances_to_2412_and_lights_the_access_panel() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1321).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle.ThroneRoom"),
    );
    ctx.set_param(
        "mission_706_step_2411_status".to_string(),
        serde_json::json!("active"),
    );

    let resolved = fire(&engine, TriggerType::RegionEnter, &ctx);
    assert_no_deferred_actions(&resolved, 1321);
    let actions = actions_of(&resolved, 1321);

    assert_eq!(
        actions.len(),
        2,
        "chain 1321 must resolve exactly two actions (advance + glow); got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AdvanceStep {
                mission_id: 706,
                step_id: 2412
            }
        ),
        "chain 1321's first action must be AdvanceStep(706, 2412); got {:?}",
        actions[0],
    );
    assert!(
        matches!(
            actions[1],
            Action::SetInteractionType { entity_tag, operation, mask }
                if entity_tag == "Castle_AccessPanel" && operation == "|" && *mask == GLOW
        ),
        "chain 1321 must OR INT_MissionWorldObject onto Castle_AccessPanel; got {:?}",
        actions[1],
    );
}

/// Chain 1321 negative: walking back into the Throne Room after the step
/// advanced must not re-advance. `advance_step` is unconditional
/// (`missions/progression.rs:20-129`), so without the `eq active` gate a
/// second region entry would reset the mission to step 2412 and re-arm
/// objectives the player had already completed.
#[tokio::test]
async fn chain_1321_does_not_refire_once_past_step_2411() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1321).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle.ThroneRoom"),
    );
    ctx.set_param(
        "mission_706_step_2411_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );

    let resolved = fire(&engine, TriggerType::RegionEnter, &ctx);
    assert!(
        actions_of(&resolved, 1321).is_empty(),
        "chain 1321 must not resolve once step 2411 is behind the player; \
         got {:?}",
        resolved.actions,
    );
}

/// Chain 1322 happy path: using the panel on step 2412 plays 2584,
/// completes 706 and accepts 708 — each exactly once, in seed order.
///
/// The ordering assertion is the point. `complete_mission` before
/// `accept_mission` matters because `accept_mission` synchronously
/// re-enters the dispatcher to fire chain 1341
/// (`executor/mission.rs:104-107`); doing it the other way round would
/// run 708's setup while 706 was still the active mission.
#[tokio::test]
async fn chain_1322_completes_706_and_accepts_708_exactly_once() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1322).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_AccessPanel"),
    );
    ctx.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );
    // 708 has not been offered yet — this is both the accept gate and
    // the discriminator against chain 1344 (same tag, mission 708).
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("not_active"),
    );

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 1322);
    let actions = actions_of(&resolved, 1322);

    assert_eq!(
        actions.len(),
        3,
        "chain 1322 must resolve exactly three actions (dialog, complete, \
         accept); got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 2584 }),
        "chain 1322 must play dialog 2584 first; got {:?}",
        actions[0],
    );
    assert!(
        matches!(actions[1], Action::CompleteMission { mission_id: 706 }),
        "chain 1322 must complete 706 before accepting 708; got {:?}",
        actions[1],
    );
    assert!(
        matches!(actions[2], Action::AcceptMission { mission_id: 708 }),
        "chain 1322 must accept 708 last; got {:?}",
        actions[2],
    );

    // The glow must NOT be cleared here. Template 147 ships
    // `interaction_type = 0` (entity_templates.sql:25), the flag lives on
    // the SHARED entity (executor/world/mod.rs:19-64), and mission 708's
    // step-2415 panel route needs it immediately afterwards. A clear
    // would also strand the panel permanently if `accept_mission` hit
    // the offer guard's early return (executor/mission.rs:64-72) and
    // chain 1341 never ran. See the seed header, engine fact (7).
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::SetInteractionType { operation, .. } if operation == "~")),
        "chain 1322 must not clear any interaction bit — the Access Panel is \
         a shared, zero-baseline prop; got {actions:?}",
    );
}

/// Chain 1322 negative 1: the panel must do nothing on the wrong step.
/// Without the gate, a player who wandered into the Throne Room before
/// reaching step 2412 could complete 706 early.
#[tokio::test]
async fn chain_1322_does_not_fire_before_step_2412() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1322).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_AccessPanel"),
    );
    ctx.set_param(
        "mission_706_step_2411_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("not_active"),
    );

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert!(
        actions_of(&resolved, 1322).is_empty(),
        "chain 1322 must not fire while the player is still on step 2411; got {:?}",
        resolved.actions,
    );
}

/// Chain 1322 negative 2: once 708 is live the panel belongs to mission
/// 708's diagnosis route (chain 1344), not to 706's completion. This is
/// the condition that keeps a second panel click from re-completing 706
/// and re-accepting 708.
#[tokio::test]
async fn chain_1322_does_not_refire_once_708_is_active() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1322).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_AccessPanel"),
    );
    // Pathological state: 706 somehow still reports step 2412 active
    // while 708 is running. The mission gate must still refuse.
    ctx.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("active"),
    );

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert!(
        actions_of(&resolved, 1322).is_empty(),
        "chain 1322 must not resolve while mission 708 is active — the \
         `mission_status 708 eq not_active` row is the re-accept guard; got {:?}",
        resolved.actions,
    );
}

/// Chain 1323: relog on step 2412 repaints the panel glow.
#[tokio::test]
async fn chain_1323_restores_the_panel_glow_on_relog_at_step_2412() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1323).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
    ctx.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );

    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    let actions = actions_of(&resolved, 1323);
    assert_eq!(
        actions.len(),
        1,
        "chain 1323 must resolve exactly one action; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::SetInteractionType { entity_tag, operation, mask }
                if entity_tag == "Castle_AccessPanel" && operation == "|" && *mask == GLOW
        ),
        "chain 1323 must re-OR the panel glow; got {:?}",
        actions[0],
    );
}

/// Chain 1323 negative: a player who is not on step 2412 must not have
/// the panel lit for them on login. Also covers the world-name key —
/// loading into `Castle_CellBlock` must not fire a `Castle` chain.
#[tokio::test]
async fn chain_1323_does_not_restore_on_another_step_or_another_world() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1323).await;

    let mut wrong_step = ExecutionContext::new();
    wrong_step.set_param("world_name".to_string(), serde_json::json!("Castle"));
    wrong_step.set_param(
        "mission_706_step_2411_status".to_string(),
        serde_json::json!("active"),
    );
    assert!(
        actions_of(&fire(&engine, TriggerType::PlayerLoaded, &wrong_step), 1323).is_empty(),
        "chain 1323 must not repaint on a step that never lit the panel",
    );

    let mut wrong_world = ExecutionContext::new();
    wrong_world.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    wrong_world.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );
    assert!(
        actions_of(
            &fire(&engine, TriggerType::PlayerLoaded, &wrong_world),
            1323
        )
        .is_empty(),
        "chain 1323 is keyed on world `Castle`; a Cellblock load must not match",
    );
}

/// The 706 → 708 handover invariant, stated once with BOTH panel chains
/// in one engine: a single click on `Castle_AccessPanel` must never
/// resolve chain 1322 (706's completion) and chain 1344 (708's
/// diagnosis) together.
///
/// The per-chain negatives above and
/// `mission_708::diagnosis::chain_1344_does_not_fire_during_mission_706`
/// each fire one chain in an isolated engine, which approximates this but
/// never asserts it. The approximation has a real seam:
/// `Condition::MissionStatus` reads a missing key as `not_active`
/// (`conditions.rs:190-194`, `.unwrap_or("not_active")`), so a context
/// carrying only `mission_706_step_2412_status = active` plus
/// `mission_708_step_2415_status = active` — with `mission_708_status`
/// absent — satisfies both chains at once. Production's
/// `populate_mission_context` always writes both keys, so that state is
/// not live-reachable; the point is that only a both-registered fixture
/// can tell "mutually exclusive" from "never tested together".
///
/// Double-firing here is the worst outcome in either mission: 1322 would
/// complete 706 and accept 708 while 1344 displayed the 2415 diagnostic
/// dialog, leaving the player holding a dialog whose `dialog_choice`
/// chain (1345) then advances 708 out of a step it had only just entered.
#[tokio::test]
async fn one_panel_click_never_resolves_both_706_and_708_chains() {
    let pool = require_db_or_skip!();

    let mut engine = ChainEngine::new();
    for chain_id in [1322, 1344] {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }

    /// `(label, [(param, value)], expect_1322, expect_1344)`
    type State = (&'static str, Vec<(&'static str, &'static str)>, bool, bool);

    let states: Vec<State> = vec![
        (
            "mid-706 on step 2412, 708 not yet accepted",
            vec![
                ("mission_706_step_2412_status", "active"),
                ("mission_708_status", "not_active"),
            ],
            true,
            false,
        ),
        (
            "mid-708 on step 2415, 706 already complete",
            vec![
                ("mission_706_status", "completed"),
                ("mission_708_status", "active"),
                ("mission_708_step_2415_status", "active"),
            ],
            false,
            true,
        ),
        (
            "both missions finished — the panel is inert",
            vec![
                ("mission_706_status", "completed"),
                ("mission_708_status", "completed"),
            ],
            false,
            false,
        ),
    ];

    for (label, params, expect_1322, expect_1344) in states {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "entity_tag".to_string(),
            serde_json::json!("Castle_AccessPanel"),
        );
        for (key, value) in params {
            ctx.set_param(key.to_string(), serde_json::json!(value));
        }

        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        let from_1322 = actions_of(&resolved, 1322);
        let from_1344 = actions_of(&resolved, 1344);

        assert_eq!(
            !from_1322.is_empty(),
            expect_1322,
            "state `{label}`: chain 1322 resolution mismatch; got {from_1322:?}",
        );
        assert_eq!(
            !from_1344.is_empty(),
            expect_1344,
            "state `{label}`: chain 1344 resolution mismatch; got {from_1344:?}",
        );
        assert!(
            from_1322.is_empty() || from_1344.is_empty(),
            "state `{label}`: ONE panel click resolved BOTH the 706 completion \
             chain and the 708 diagnosis chain. 706 would complete and 708 \
             accept while the 2415 diagnostic dialog opened, and closing that \
             dialog advances 708 out of a step it just entered. 1322 gave \
             {from_1322:?}; 1344 gave {from_1344:?}",
        );
    }
}
