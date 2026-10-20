//! defense_q1 refusals (CS-04b1c2): the sentry click in the wrong world, without
//! Q1, while active or after completion, the Infirmary entry at the wrong step,
//! and a forged dialog close, button or open of dialog 10709.

use tracing::Level;

use super::super::super::{fire_dialog_choice, fire_dialog_open};
use super::*;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

/// The DEBUG `content.resolve` rows the capture holds for `chain_id`.
fn resolve_rows(capture: &LogCaptureGuard, chain_id: i64) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::DEBUG
                && c.target == "content.resolve"
                && c.has_field("chain_id", &chain_id.to_string())
        })
        .collect()
}

/// Assert `rows` is exactly one `condition_failed` row at `failed_index`.
fn assert_failed_at(rows: &[Captured], failed_index: i32, what: &str) {
    assert_eq!(rows.len(), 1, "{what}: {rows:#?}");
    assert!(rows[0].has_field("reason", "condition_failed"), "{rows:#?}");
    assert!(
        rows[0].has_field("failed_condition_index", &failed_index.to_string()),
        "{what}: condition {failed_index} must fail; {rows:#?}"
    );
}

/// **The sentry click accepts only in world 8 with Q1 completed and 10310
/// not started.** A positive control is handled by 10103. Then the same click
/// in CombatSim, with Q1 active, and with no Q1 at all is handled by nothing,
/// sends nothing, leaves 10310 unstarted, and logs a `condition_failed` row
/// for 10103 naming the failing condition (0 = world, 1 = `mission_status
/// 10300`). Revert proof: drop 10103's `world` condition and the CombatSim
/// case is handled.
#[tokio::test]
async fn live_db_castle_side_defense_q1_wrong_world_and_missing_prerequisite_refused() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut control = staged(&pool, CASTLE, stages(Stage::Done, Stage::Absent)).await;
    let (handled, out) = click_sentry(&engine, &mut control).await;
    assert!(
        handled,
        "control: chain 10103 must accept in world 8 after Q1"
    );
    assert!(
        has_update(&out, Some(STEP_A)),
        "control: the accept must persist 10310 at step A"
    );

    // (world, Q1 state, index of the condition that must fail)
    let refused = [
        (COMBAT_SIM, Stage::Done, 0),
        (CASTLE, Stage::AtA, 1),
        (CASTLE, Stage::Absent, 1),
    ];
    for (world, q1, failed_index) in refused {
        let capture = LogCapture::install();
        let mut mgr = staged(&pool, world, stages(q1, Stage::Absent)).await;
        let want_world = if world == CASTLE { 8 } else { 1 };
        assert_eq!(
            mgr.get_entity_world_id(ENTITY),
            Some(want_world),
            "{world}: the player must be staged in world {want_world}"
        );
        let (handled, out) = click_sentry(&engine, &mut mgr).await;
        assert!(
            !handled,
            "{world} with Q1 {q1:?}: no chain may handle the click"
        );
        assert!(
            !has_mission_or_dialog(&out),
            "{world} with Q1 {q1:?}: no mission or dialog traffic; got {out:?}"
        );
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "{world} with Q1 {q1:?}: 10310 must not start"
        );
        let rows = resolve_rows(&capture, ACCEPT_CHAIN);
        assert_failed_at(&rows, failed_index, &format!("{world} with Q1 {q1:?}"));
    }
}

/// **Already active, already completed and wrong step are refused.** The
/// click at A or B, or after completion, is handled by nothing (10103's
/// `not_active` gate, index 2). Entering the Infirmary at A is refused by
/// 10105's step gate (index 2): the completion needs step B. Each case sends
/// no mission traffic and leaves 10310 where it was. Revert proof: drop
/// 10105's `step_status 10355` condition and the entry at A completes 10310.
#[tokio::test]
async fn live_db_castle_side_defense_q1_already_active_completed_and_wrong_step_refused() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for sentry in [Stage::AtA, Stage::AtB, Stage::Done] {
        let capture = LogCapture::install();
        let mut mgr = staged(&pool, CASTLE, stages(Stage::Done, sentry)).await;
        let before = snapshot(&mgr);
        let (handled, out) = click_sentry(&engine, &mut mgr).await;
        assert!(!handled, "10310 {sentry:?}: the click must fall through");
        assert!(
            !has_mission_or_dialog(&out),
            "10310 {sentry:?}: no mission or dialog traffic; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "10310 {sentry:?}: the click moved state"
        );
        let rows = resolve_rows(&capture, ACCEPT_CHAIN);
        assert_failed_at(&rows, 2, &format!("10310 {sentry:?}"));
    }

    let capture = LogCapture::install();
    let mut at_a = staged(&pool, CASTLE, stages(Stage::Done, Stage::AtA)).await;
    let out = enter(&engine, &mut at_a, INFIRMARY).await;
    assert!(
        !has_update(&out, None),
        "the Infirmary at A must send no mission traffic; got {out:?}"
    );
    assert_eq!(step_of(&at_a), Some(STEP_A), "at A the step must not move");
    assert_eq!(
        status_of(&at_a, MISSION),
        Some(MISSION_ACTIVE),
        "at A the mission must stay active"
    );
    let rows = resolve_rows(&capture, INFIRMARY_CHAIN);
    assert_failed_at(&rows, 2, "the Infirmary at A (step_status 10355)");
}

/// **A forged close, button or open of dialog 10709 moves nothing.** No chain
/// keys on dialog 10709 or on any dialog event under scope 10310 (checked in
/// the seed), and the real dialog dispatchers confirm it in every state: the
/// close (`button -1`), two button ordinals and the open that an
/// `initialResponse` ends in send no traffic and leave 10310, the offer
/// binding and the dialogs on screen as they were. Revert proof: add a
/// `dialog_choice` trigger on 10709 and the seed count fails.
#[tokio::test]
async fn live_db_castle_side_defense_q1_forged_close_is_inert() {
    let pool = require_db_or_skip!();
    let keyed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE (t.event_type IN ('dialog_choice', 'dialog_open') AND t.event_key = $1) \
            OR (c.scope_type = 'mission' AND c.scope_id = $2 \
                AND t.event_type IN ('dialog_choice', 'dialog_open', 'dialog_set_open'))",
    )
    .bind(DIALOG.to_string())
    .bind(MISSION)
    .fetch_one(&pool)
    .await
    .expect("dialog trigger query must succeed");
    assert_eq!(
        keyed, 0,
        "no chain may key on dialog {DIALOG} or on a dialog event under scope {MISSION}"
    );

    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for sentry in [Stage::Absent, Stage::AtA, Stage::AtB, Stage::Done] {
        let mut mgr = staged(&pool, CASTLE, stages(Stage::Done, sentry)).await;
        if matches!(sentry, Stage::Absent) {
            log_in(&engine, &mut mgr).await;
        }
        let before = snapshot(&mgr);
        let (tx, mut rx) = mpsc::channel(256);
        for button in [-1, 0, 1] {
            fire_dialog_choice(ENTITY, PLAYER, DIALOG, button, &engine, &tx, &mut mgr).await;
        }
        fire_dialog_open(ENTITY, PLAYER, DIALOG, &engine, &tx, &mut mgr).await;
        let out = drain(&mut rx);
        assert!(
            out.is_empty(),
            "10310 {sentry:?}: forged dialog events must send nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "10310 {sentry:?}: forged dialog events moved state"
        );
    }
}
