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
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

use super::{
    actions_of, assert_no_deferred_actions, count_flag_ops, dialog_ctx, engine_for, fire, step_ctx,
    BANG, GLOW, TAURI,
};
use crate::test_support::require_db_or_skip;

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
        3,
        "chain 1343 must resolve exactly three actions; got {actions:?}",
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
    assert_eq!(
        count_flag_ops(&actions, "Castle_SurrenderGuard", "~", BANG),
        1,
        "chain 1343 must clear the guard's '!' cue; got {actions:?}",
    );

    // The panel's glow must survive. It is the only bit making a
    // zero-baseline shared prop clickable and another player may be
    // mid-706 or mid-2415 on it.
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
        3,
        "chain 1345 must resolve exactly three actions; got {actions:?}",
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
        1,
        "chain 1345 must clear the guard's '!' too — whichever route was taken, \
         the guard has nothing further to say at 2416; got {actions:?}",
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
