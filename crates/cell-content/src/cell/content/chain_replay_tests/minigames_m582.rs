//! Mission 582 step 2307, Converse with Marsh (MG-61), seeded in
//! [`db/resources/Content/Seed/minigames_m582_chains.sql`] (launcher 24000,
//! victory 24001) and enabled in `mission_steps.sql` (step 2307).
//!
//! Mission 582 cannot be offered yet (its giver has no spawn, M10), so these
//! tests set the mission and step active directly. The launcher shares the
//! `CmdCenter_Marsh` tag with Marsh's chains 6501 and 6527, and it must step
//! aside while missions 1360 (step 4038) and 1361 (step 4694) are on their
//! Marsh steps.
//!
//! Four assertions, as in [`super::livewire_pairs`]: the launcher resolves
//! exactly one `StartMinigame(ConverseBasicHumanoid, …)` under its step gate;
//! it resolves nothing off the step or under the Marsh guard; the executor
//! emits one `CellToBaseMsg::StartMinigame`; the victory chain's action list
//! matches the seed. A fifth pins the step-enable half of the change.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::livewire_pairs::execute_and_drain_starts;
use crate::test_support::require_db_or_skip;

const LAUNCHER_ID: i32 = 24000;
const VICTORY_ID: i64 = 24001;
/// The world Marsh stands in (`Harset_CmdCenter`).
const MARSH_WORLD_ID: i32 = 68;
const MARSH_TAG: &str = "CmdCenter_Marsh";

/// Mission 582 active at step 2307, with Marsh's other two missions off their
/// Marsh steps, so the launcher's own gate is the only thing that varies.
fn converse_gate() -> Vec<(&'static str, &'static str)> {
    vec![
        ("mission_582_status", "active"),
        ("mission_582_step_2307_status", "active"),
        ("mission_1360_step_4038_status", "not_active"),
        ("mission_1361_step_4694_status", "not_active"),
    ]
}

/// Load `chain_id`, fire `CmdCenter_Marsh` in world 68 with the supplied
/// params, and return the actions the chain resolved.
async fn resolve_marsh(
    pool: &PgPool,
    chain_id: i32,
    params: &[(&str, &str)],
) -> Vec<(i64, Action)> {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(MARSH_WORLD_ID);
    ctx.set_param("entity_tag".to_string(), serde_json::json!(MARSH_TAG));
    for (key, value) in params {
        ctx.set_param(key.to_string(), serde_json::json!(value));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx).actions
}

/// Count the actions chain `chain_id` resolved, whatever they are.
fn fired_by(actions: &[(i64, Action)], chain_id: i32) -> usize {
    actions
        .iter()
        .filter(|(id, _)| *id == chain_id as i64)
        .count()
}

/// Interacting with Marsh while step 2307 is active shows prompt 2433 and
/// starts ConverseBasicHumanoid at difficulty 1 with victory chain 24001.
#[tokio::test]
async fn live_db_chain_24000_starts_converse_and_reaches_base() {
    let pool = require_db_or_skip!();
    let actions = resolve_marsh(&pool, LAUNCHER_ID, &converse_gate()).await;

    let starts = actions
        .iter()
        .filter(|(id, action)| {
            *id == LAUNCHER_ID as i64
                && matches!(
                    action,
                    Action::StartMinigame { minigame_type, on_victory_chains, .. }
                    if minigame_type == "ConverseBasicHumanoid"
                        && on_victory_chains == &vec![VICTORY_ID]
                )
        })
        .count();
    assert_eq!(
        starts, 1,
        "chain {LAUNCHER_ID} must resolve exactly one \
         StartMinigame(ConverseBasicHumanoid, on_victory_chains=[{VICTORY_ID}]) \
         under its step gate; got {actions:?}",
    );

    let prompts = actions
        .iter()
        .filter(|(id, action)| {
            *id == LAUNCHER_ID as i64 && matches!(action, Action::DisplayDialog { dialog_id: 2433 })
        })
        .count();
    assert_eq!(
        prompts, 1,
        "chain {LAUNCHER_ID} must show prompt dialog 2433 once; got {actions:?}",
    );

    let sends = execute_and_drain_starts(actions).await;
    assert_eq!(
        sends.len(),
        1,
        "chain {LAUNCHER_ID} must emit exactly one CellToBaseMsg::StartMinigame; \
         got {sends:?} (zero means the executor has no Action::StartMinigame arm)",
    );
    assert_eq!(
        (sends[0].1.as_str(), sends[0].2, sends[0].3.clone()),
        ("ConverseBasicHumanoid", 1, vec![VICTORY_ID]),
        "StartMinigame must carry the authored game name, difficulty 1 and the \
         authored victory chain",
    );
}

/// The launcher must resolve nothing once step 2307 is completed, and nothing
/// while Marsh's mission 1360 (step 4038) or 1361 (step 4694) is active on
/// its own Marsh step. Either would steal the click from those chains.
#[tokio::test]
async fn live_db_chain_24000_resolves_nothing_off_step_or_under_marsh_guard() {
    let pool = require_db_or_skip!();

    let past_step = [
        ("mission_582_status", "active"),
        ("mission_582_step_2307_status", "completed"),
        ("mission_582_step_3274_status", "active"),
        ("mission_1360_step_4038_status", "not_active"),
        ("mission_1361_step_4694_status", "not_active"),
    ];
    let actions = resolve_marsh(&pool, LAUNCHER_ID, &past_step).await;
    assert_eq!(
        fired_by(&actions, LAUNCHER_ID),
        0,
        "chain {LAUNCHER_ID} must NOT fire once step 2307 is completed; got {actions:?}",
    );

    let guard_1360 = [
        ("mission_582_status", "active"),
        ("mission_582_step_2307_status", "active"),
        ("mission_1360_step_4038_status", "active"),
        ("mission_1361_step_4694_status", "not_active"),
    ];
    let actions = resolve_marsh(&pool, LAUNCHER_ID, &guard_1360).await;
    assert_eq!(
        fired_by(&actions, LAUNCHER_ID),
        0,
        "chain {LAUNCHER_ID} must NOT fire while mission 1360 step 4038 is active \
         (it would steal Marsh's click); got {actions:?}",
    );

    let guard_1361 = [
        ("mission_582_status", "active"),
        ("mission_582_step_2307_status", "active"),
        ("mission_1360_step_4038_status", "not_active"),
        ("mission_1361_step_4694_status", "active"),
    ];
    let actions = resolve_marsh(&pool, LAUNCHER_ID, &guard_1361).await;
    assert_eq!(
        fired_by(&actions, LAUNCHER_ID),
        0,
        "chain {LAUNCHER_ID} must NOT fire while mission 1361 step 4694 is active \
         (it would steal Marsh's click); got {actions:?}",
    );
}

/// Chain 24001 (Converse victory) must complete objective 2675 and then
/// advance mission 582 to step 3274, in that order.
#[tokio::test]
async fn live_db_chain_24001_victory_completes_2675_then_advances_to_3274() {
    let pool = require_db_or_skip!();
    let actions = load_single_chain_for_test(&pool, VICTORY_ID as i32)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {VICTORY_ID} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {VICTORY_ID} must exist in seeded content_chains"))
        .actions;

    assert_eq!(
        actions.len(),
        2,
        "chain {VICTORY_ID} must resolve exactly 2 actions (complete objective + \
         advance); got {actions:?}",
    );
    assert!(
        matches!(
            &actions[0],
            Action::CompleteObjective {
                mission_id: 582,
                objective_id: 2675
            }
        ),
        "chain {VICTORY_ID}'s first action must complete objective 2675 of mission \
         582; got {:?}",
        actions[0],
    );
    assert!(
        matches!(
            &actions[1],
            Action::AdvanceStep {
                mission_id: 582,
                step_id: 3274
            }
        ),
        "chain {VICTORY_ID}'s second action must advance mission 582 to step 3274; \
         got {:?}",
        actions[1],
    );
}

/// The seeded step 2307 row is enabled. Without this the step is never
/// reachable, whatever the chain says.
#[tokio::test]
async fn live_db_seeded_step_2307_of_mission_582_is_enabled() {
    let pool = require_db_or_skip!();
    let enabled: bool = sqlx::query_scalar(
        "SELECT step_enabled FROM resources.mission_steps \
         WHERE step_id = $1 AND mission_id = $2",
    )
    .bind(2307_i32)
    .bind(582_i32)
    .fetch_one(&pool)
    .await
    .expect("mission_steps must hold step 2307 of mission 582");

    assert!(
        enabled,
        "step 2307 of mission 582 must have step_enabled = true (MG-61); the \
         Converse launcher can never run on a disabled step",
    );
}
