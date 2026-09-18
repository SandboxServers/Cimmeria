//! Step 2415 — "Discover why the Stargate will not dial out."
//! Chains 1341-1345.
//!
//! Two routes, either of which satisfies the step: interrogate the
//! surrendered guard (optional objective 2794, dialog 5003) or run the
//! Access Panel diagnostic (optional objective 2795, dialog 5004). Both
//! objectives are `is_optional = true` with required hidden objective
//! 2796 still active, so completing one cannot end the mission early —
//! `advance_step` closes the step instead.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE};
use tokio::sync::mpsc;

use super::super::super::executor::execute_actions;
use super::{
    actions_of, assert_no_deferred_actions, count_flag_ops, dialog_ctx, engine_for, fire,
    make_castle_space_mgr, step_ctx, BANG, GLOW, TAURI,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7185;
const PLAYER_ID: i32 = 7186;

/// Stage a player mid-708 on step 2415 with the step's REAL objective
/// set, taken from `mission_objectives.sql:6619/6621/6625`:
///
/// - 2796 — required (`is_optional = false`) and hidden,
/// - 2794 — optional, the guard-interrogation route,
/// - 2795 — optional, the Access Panel route.
///
/// Those flags are the whole point of the two executed guards below.
/// `complete_objective`'s auto-complete branch filters on `!optional`
/// (`missions/progression.rs:176-180`); today 2796 keeps the step from
/// ever being "all required complete", so ticking 2794 or 2795 cannot
/// end mission 708. A resolve-only test cannot see that either way,
/// because the auto-complete lives in the executor.
///
/// Scope caveat, stated so nobody inherits an overclaim: these flags are
/// HARD-CODED, not read back from `mission_objectives.sql`. So the
/// guards catch a chain in `castle_706_708_chains.sql` growing an extra
/// `complete_objective` row, and they catch a regression in the
/// executor's all-required check. An `is_optional` flip in the
/// objectives seed itself would NOT fail them — the fixture would go on
/// asserting the old flags. Mirroring such a flip into this fixture is a
/// manual step; see the worknote's Known gaps.
fn stage_player_on_step_2415(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, "Castle", [806.0, 55.0, 517.0], [0.0; 3])
        .expect("Castle startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(TAURI);
    p.missions.add_mission(MissionInstance::new(
        708,
        2415,
        vec![
            MissionObjective {
                objective_id: 2796,
                status: STATUS_ACTIVE,
                hidden: true,
                optional: false,
            },
            MissionObjective {
                objective_id: 2794,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: true,
            },
            MissionObjective {
                objective_id: 2795,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: true,
            },
        ],
    ));
    mgr.connect_entity(PLAYER_EID);
}

/// Execute one diagnosis route end to end and assert mission 708 is
/// still ACTIVE on step 2416 with `objective_id` recorded complete.
///
/// Shared by both routes because the hazard is identical — only the
/// dialog and the objective differ.
async fn assert_route_advances_without_completing(
    pool: &sqlx::PgPool,
    chain_id: i32,
    dialog_id: i32,
    objective_id: i32,
) {
    let engine = engine_for(pool, chain_id).await;

    let mut mgr = make_castle_space_mgr();
    stage_player_on_step_2415(&mut mgr);
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();

    let resolved = fire(
        &engine,
        TriggerType::DialogChoice,
        &dialog_ctx(dialog_id, 2415),
    );
    assert!(
        !resolved.actions.is_empty(),
        "chain {chain_id} must resolve — an empty list would make every \
         assertion below vacuously true",
    );
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mission = mgr
        .get_entity(PLAYER_EID)
        .and_then(|e| e.missions.get_mission(708))
        .cloned()
        .expect("mission 708 must still be tracked on the player");

    assert_eq!(
        mission.status, MISSION_ACTIVE,
        "mission 708 must still be ACTIVE after the {chain_id} route. A \
         `completed` here means required hidden objective 2796 was flipped to \
         `is_optional = true` and `complete_objective`'s all-required check now \
         fires on {objective_id}, ending 708 at step 2415.",
    );
    assert_eq!(
        mission.current_step_id,
        Some(2416),
        "the player must be on step 2416 (Retrieve the Control Crystal) after \
         the {chain_id} route",
    );
    assert!(
        mission.completed_objectives.contains(&objective_id),
        "objective {objective_id} must be recorded completed; got {:?}",
        mission.completed_objectives,
    );

    // Drain so a full channel can't mask a send failure in a later run.
    while rx.try_recv().is_ok() {}
}

/// Executed guard for the guard-interrogation route (chain 1343).
#[tokio::test]
async fn guard_route_advances_without_completing_the_mission() {
    let pool = require_db_or_skip!();
    assert_route_advances_without_completing(&pool, 1343, 5003, 2794).await;
}

/// Executed guard for the Access Panel route (chain 1345). Mirrors
/// [`guard_route_advances_without_completing_the_mission`] — both
/// objectives are optional, so either one alone would end the mission if
/// 2796 stopped being required.
#[tokio::test]
async fn panel_route_advances_without_completing_the_mission() {
    let pool = require_db_or_skip!();
    assert_route_advances_without_completing(&pool, 1345, 5004, 2795).await;
}

/// Chain 1341: accepting 708 lights both diagnosis affordances at once.
/// Both are needed simultaneously because the two routes are
/// alternatives, not a sequence.
#[tokio::test]
async fn chain_1341_marks_guard_and_panel_on_708_accept() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1341).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(708));

    let resolved = fire(&engine, TriggerType::MissionAccepted, &ctx);
    assert_no_deferred_actions(&resolved, 1341);
    let actions = actions_of(&resolved, 1341);

    assert_eq!(
        actions.len(),
        2,
        "chain 1341 must light exactly the guard and the panel; got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_SurrenderGuard", "|", BANG),
        1,
        "chain 1341 must OR the '!' cue onto Castle_SurrenderGuard; got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_AccessPanel", "|", GLOW),
        1,
        "chain 1341 must OR the quest glow onto Castle_AccessPanel — mission \
         706's completion chain hands the panel over without clearing it, and \
         this re-assert is what covers a player who reaches 708 any other way; \
         got {actions:?}",
    );
}

/// Chain 1341 negative: accepting a different mission must not paint
/// 708's cues. The trigger key is the mission id, so this pins the
/// `event_key = '708'` row.
#[tokio::test]
async fn chain_1341_does_not_fire_for_another_mission_accept() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1341).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(706));

    assert!(
        actions_of(&fire(&engine, TriggerType::MissionAccepted, &ctx), 1341).is_empty(),
        "chain 1341 is keyed on mission 708; accepting 706 must not paint its cues",
    );
}

/// Chain 1342: talking to the guard on step 2415 plays 5003 and nothing
/// else. The objective is NOT completed here — the player has to read
/// the interrogation, and closing it fires chain 1343.
#[tokio::test]
async fn chain_1342_displays_5003_on_guard_interact_at_2415() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1342).await;

    let mut ctx = step_ctx(2415, TAURI);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_SurrenderGuard"),
    );

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 1342);
    let actions = actions_of(&resolved, 1342);
    assert_eq!(
        actions.len(),
        1,
        "chain 1342 must resolve exactly one action; got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 5003 }),
        "chain 1342 must display dialog 5003; got {:?}",
        actions[0],
    );
}

/// Chain 1342 negative: the guard has nothing to say once the step has
/// moved on. Without the gate, re-interrogating at step 2416 would
/// re-display 5003 and re-arm chain 1343's `dialog_choice`, which would
/// then fail its own step gate — a dead dialog loop rather than a hard
/// break, but still wrong.
#[tokio::test]
async fn chain_1342_does_not_display_5003_after_2415() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1342).await;

    let mut ctx = step_ctx(2416, TAURI);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_SurrenderGuard"),
    );

    assert!(
        actions_of(&fire(&engine, TriggerType::InteractTag, &ctx), 1342).is_empty(),
        "chain 1342 must not re-display 5003 once the player is past step 2415",
    );
}

/// Chain 1343: closing 5003 ticks the surrender objective, advances, and
/// clears the guard's "!".
///
/// The ORDER of the first two actions is load-bearing.
/// `mission.complete_objective(2794)` resolves against the CURRENT
/// step's objective list, so running it after `advance_step` would look
/// for 2794 among step 2416's objectives, miss, and early-return at
/// `missions/progression.rs:155-157` — the client would never see the
/// objective tick.
#[tokio::test]
async fn chain_1343_completes_2794_then_advances_to_2416() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1343).await;

    let ctx = dialog_ctx(5003, 2415);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_no_deferred_actions(&resolved, 1343);
    let actions = actions_of(&resolved, 1343);

    assert_eq!(
        actions.len(),
        2,
        "chain 1343 must resolve exactly two actions; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteObjective {
                mission_id: 708,
                objective_id: 2794
            }
        ),
        "chain 1343 must complete objective 2794 FIRST — after an advance_step \
         it would resolve against the wrong step's objective list and no-op; \
         got {:?}",
        actions[0],
    );
    assert!(
        matches!(
            actions[1],
            Action::AdvanceStep {
                mission_id: 708,
                step_id: 2416
            }
        ),
        "chain 1343's second action must be AdvanceStep(708, 2416); got {:?}",
        actions[1],
    );
    // Nothing on this route clears a cue. Both the guard and the panel
    // sit at template baseline 0 (`Castle_SurrenderGuard`'s template is
    // CA05's and unknown; `Castle_AccessPanel` is template 147), so
    // clearing either one's last bit would strip its only affordance
    // from every other player still on 706 step 2412 or 708 step 2415.
    // Zero-baseline rule, seed engine fact (7).
    assert_eq!(
        count_flag_ops(&actions, "Castle_SurrenderGuard", "~", BANG),
        0,
        "chain 1343 must NOT clear the guard's '!' — the guard's template \
         baseline is unknown until CA05 lands, and a clear to flags 0 makes a \
         shared NPC unclickable for a bystander; got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_AccessPanel", "~", GLOW),
        0,
        "chain 1343 must NOT clear the shared Access Panel glow; got {actions:?}",
    );

    // Neither report objective may be touched here. Completing 5185 or
    // 5186 out of step order is the failure that ends 708 at the wrong
    // point.
    assert!(
        !actions.iter().any(|a| matches!(
            a,
            Action::CompleteObjective {
                objective_id: 5185 | 5186 | 2795,
                ..
            }
        )),
        "chain 1343 must complete only objective 2794; got {actions:?}",
    );
}

/// Chain 1343 negative: a forged or stale close of 5003 outside step
/// 2415 must resolve nothing. `advance_step` is unconditional, so
/// without the gate a replayed choice would drag the mission back to
/// 2416 from wherever it had reached.
#[tokio::test]
async fn chain_1343_does_not_fire_outside_step_2415() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1343).await;

    for step in [2416, 2417, 2418, 4462, 4469] {
        let ctx = dialog_ctx(5003, step);
        assert!(
            actions_of(&fire(&engine, TriggerType::DialogChoice, &ctx), 1343).is_empty(),
            "chain 1343 must not resolve while the player is on step {step}",
        );
    }
}

/// Chain 1344: the panel diagnostic route. Same tag as mission 706's
/// completion chain 1322 — the two are separated only by their step and
/// mission gates, so this pins that 1344 answers the 708 side.
#[tokio::test]
async fn chain_1344_displays_5004_on_panel_interact_at_2415() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1344).await;

    let mut ctx = step_ctx(2415, TAURI);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_AccessPanel"),
    );

    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 1344);
    let actions = actions_of(&resolved, 1344);
    assert_eq!(
        actions.len(),
        1,
        "chain 1344 must resolve exactly one action; got {actions:?}",
    );
    assert!(
        matches!(actions[0], Action::DisplayDialog { dialog_id: 5004 }),
        "chain 1344 must display dialog 5004; got {:?}",
        actions[0],
    );
}

/// Chain 1344 negative: the panel must not offer the 708 diagnostic
/// while the player is still finishing mission 706 on step 2412.
#[tokio::test]
async fn chain_1344_does_not_fire_during_mission_706() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1344).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_AccessPanel"),
    );
    ctx.set_param(
        "mission_706_step_2412_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_708_status".to_string(),
        serde_json::json!("not_active"),
    );

    assert!(
        actions_of(&fire(&engine, TriggerType::InteractTag, &ctx), 1344).is_empty(),
        "chain 1344 must not resolve before 708 reaches step 2415 — mission \
         706's chain 1322 owns the panel until then",
    );
}

/// Chain 1345: the panel route's completion half. Mirror of 1343 with
/// objective 2795.
#[tokio::test]
async fn chain_1345_completes_2795_then_advances_to_2416() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1345).await;

    let ctx = dialog_ctx(5004, 2415);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_no_deferred_actions(&resolved, 1345);
    let actions = actions_of(&resolved, 1345);

    assert_eq!(
        actions.len(),
        2,
        "chain 1345 must resolve exactly two actions; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::CompleteObjective {
                mission_id: 708,
                objective_id: 2795
            }
        ),
        "chain 1345 must complete objective 2795 first; got {:?}",
        actions[0],
    );
    assert!(
        matches!(
            actions[1],
            Action::AdvanceStep {
                mission_id: 708,
                step_id: 2416
            }
        ),
        "chain 1345's second action must be AdvanceStep(708, 2416); got {:?}",
        actions[1],
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_SurrenderGuard", "~", BANG),
        0,
        "chain 1345 must NOT clear the guard's '!' either — see chain 1343; \
         got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_AccessPanel", "~", GLOW),
        0,
        "chain 1345 must NOT clear the shared Access Panel glow; got {actions:?}",
    );
}

/// The two routes must be mutually exclusive on their trigger, not just
/// their conditions: closing 5003 must never satisfy the panel chain and
/// vice versa. If both fired, the player would complete both optional
/// objectives and hit `advance_step` twice — the second advancing out of
/// 2416 immediately and skipping the crystal entirely.
#[tokio::test]
async fn the_two_diagnosis_routes_never_answer_each_others_dialog() {
    let pool = require_db_or_skip!();
    let guard_chain = engine_for(&pool, 1343).await;
    let panel_chain = engine_for(&pool, 1345).await;

    let ctx_5003 = dialog_ctx(5003, 2415);
    let ctx_5004 = dialog_ctx(5004, 2415);

    assert!(
        actions_of(
            &fire(&panel_chain, TriggerType::DialogChoice, &ctx_5003),
            1345
        )
        .is_empty(),
        "chain 1345 (panel route) must not answer dialog 5003",
    );
    assert!(
        actions_of(
            &fire(&guard_chain, TriggerType::DialogChoice, &ctx_5004),
            1343
        )
        .is_empty(),
        "chain 1343 (guard route) must not answer dialog 5004",
    );
}
