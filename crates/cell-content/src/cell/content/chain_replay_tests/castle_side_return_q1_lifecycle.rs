//! Castle return_q1 side story (mission 10320), CS-04c1c2: the lifecycle and
//! the refusals of chains 10130-10133, replayed through the real dispatchers.
//!
//! * the relog restores exactly the Marsh bind its state calls for
//!   (`live_db_castle_side_return_c2_relog_each_state`);
//! * an abandon unbinds Marsh, and a new Harset crossing accepts 10320 again
//!   (`live_db_castle_side_return_c2_abandon_rebinds`);
//! * the Marsh bind is never live outside world 8
//!   (`live_db_castle_side_return_c2_no_bind_outside_world_8`), and a crossing from any
//!   world but 57, or without 708 or with 10320 already started, fires nothing
//!   (`live_db_castle_side_return_c2_wrong_world_and_missing_prereq`);
//! * a forged close, button or open of dialog 10721 moves nothing
//!   (`live_db_castle_side_return_c2_forged_close_is_inert`);
//! * the chains are disjoint from their siblings on the same keys and carry
//!   the world gates the seal freezes (`siblings`).
//!
//! The ids and helpers are in `harness`.

mod harness;
mod siblings;

use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED};
use tokio::sync::mpsc;
use tracing::Level;

use self::harness::*;
use super::super::{fire_dialog_choice, fire_dialog_open};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::missions::abandon_mission;
use crate::test_support::{require_db_or_skip, LogCapture};

// ── Relog ────────────────────────────────────────────────────────────────

/// **A relog restores exactly the bind its state calls for.** At step A the
/// login rebinds Marsh (10131); a completed 10320 and a not-started 10320 bind
/// nothing and leave the mission as it was. Revert proof: remove chain 10131
/// and the step-A login binds nothing, so the first assertion fails.
#[tokio::test]
async fn live_db_castle_side_return_c2_relog_each_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;

    let mut at_a = stage(&pool, CASTLE, vec![completed_708(), return_mission()]).await;
    log_in(&engine, &mut at_a, CASTLE).await;
    assert_eq!(
        bound_at(&at_a, BIND_SLOT),
        vec![BIND_SET],
        "a login at step A must rebind Marsh (10131)"
    );
    assert_eq!(
        snapshot(&at_a),
        (Some(MISSION_ACTIVE), Some(STEP_A), vec![BIND_SET]),
        "the login at step A must leave 10320 active at A"
    );

    let mut done = stage(&pool, CASTLE, vec![completed_708(), finished_return()]).await;
    log_in(&engine, &mut done, CASTLE).await;
    assert_eq!(
        snapshot(&done),
        (Some(MISSION_COMPLETED), None, Vec::new()),
        "a login after the report must bind nothing and leave 10320 completed"
    );

    let mut not_started = stage(&pool, CASTLE, vec![completed_708()]).await;
    log_in(&engine, &mut not_started, CASTLE).await;
    assert!(
        mission_of(&not_started).is_none(),
        "a login with 10320 not started must not create it"
    );
    assert!(
        bound_at(&not_started, BIND_SLOT).is_empty(),
        "a login with 10320 not started must bind nothing"
    );
}

// ── Abandon ──────────────────────────────────────────────────────────────

/// **An abandon at A unbinds Marsh, and the next Harset crossing accepts
/// 10320 again.** The real abandon removes 10320 first, then
/// `mission_abandoned` fires 10133, which removes the bind. Revert proof:
/// remove chain 10133 and the bind survives the abandon, so the unbind
/// assertion fails.
#[tokio::test]
async fn live_db_castle_side_return_c2_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;

    let mut mgr = stage(&pool, CASTLE, vec![completed_708(), return_mission()]).await;
    bind_marsh(&mut mgr);
    assert_eq!(bound_at(&mgr, BIND_SLOT), vec![BIND_SET]);
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
        "the abandon must remove 10320"
    );
    fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
    assert!(
        mission_of(&mgr).is_none(),
        "10320 must be gone after the abandon"
    );
    assert!(
        bound_at(&mgr, BIND_SLOT).is_empty(),
        "mission_abandoned must unbind Marsh (10133)"
    );

    let mut back = stage(&pool, HARSET, vec![completed_708()]).await;
    let first = cross_to_castle(&engine, &mut back).await;
    assert!(
        update_at(&first, Some(STEP_A)).is_some(),
        "a new Harset crossing must accept 10320 again at step A; got {first:?}"
    );
    assert_eq!(
        mission_of(&back).unwrap().status,
        MISSION_ACTIVE,
        "the re-accept must leave 10320 active"
    );
}

// ── Bind outside world 8 ─────────────────────────────────────────────────

/// **Marsh is never bound, unbound or clicked outside world 8.** The four
/// worlds that hold a Marsh spawn (12, 68, Dakara_E1 = 61) or a Harset
/// crossing origin (57) get a login that claims `Castle` (so the trigger key
/// matches and the chain's own `world` condition is the only guard) with 10320
/// at step A; none binds slot 10. An abandon there leaves a bind in place
/// (10133 is world-gated too), and the click resolves nothing. Revert proof:
/// drop `world 8` from 10131; the login in world 68 binds Marsh and the first
/// assertion fails. Drop it from 10133 instead and the abandon removes the
/// bind.
#[tokio::test]
async fn live_db_castle_side_return_c2_no_bind_outside_world_8() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;

    for (world, world_id) in [
        (CELLBLOCK, 12),
        (CMD_CENTER, 68),
        (DAKARA, 61),
        (HARSET, 57),
    ] {
        let mut mgr = stage(&pool, world, vec![completed_708(), return_mission()]).await;
        assert_eq!(
            mgr.get_entity_world_id(ENTITY),
            Some(world_id),
            "{world}: the player must be staged in world {world_id}"
        );
        log_in(&engine, &mut mgr, CASTLE).await;
        assert!(
            bound_at(&mgr, BIND_SLOT).is_empty(),
            "{world}: the login must not bind Marsh; got {:?}",
            bound_at(&mgr, BIND_SLOT)
        );
        bind_marsh(&mut mgr);
        let (tx, _rx) = mpsc::channel(256);
        assert!(abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await);
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert_eq!(
            bound_at(&mgr, BIND_SLOT),
            vec![BIND_SET],
            "{world}: an abandon outside world 8 must not unbind (10133)"
        );
        mgr.get_entity_mut(ENTITY)
            .unwrap()
            .missions
            .add_mission(return_mission());
        let (handled, out) = click_marsh(&engine, &mut mgr).await;
        assert!(!handled, "{world}: the Marsh click must resolve nothing");
        assert!(
            out.is_empty(),
            "{world}: the Marsh click must send nothing; got {out:?}"
        );
        assert_eq!(
            mission_of(&mgr).map(|m| m.status),
            Some(MISSION_ACTIVE),
            "{world}: the refused click must leave 10320 active"
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **The crossing accepts only from world 57 with 708 done and 10320 not
/// started; every other case fires nothing and logs `condition_failed` for
/// 10130 at the failing condition.** The positive control is Harset with 708
/// completed; the refusals are the other worlds (12, 68, 62, 8), 708 absent,
/// and 10320 active or completed. Revert proof: drop `mission_status 708
/// completed` from 10130; the 708-absent case accepts and the test fails.
#[tokio::test]
async fn live_db_castle_side_return_c2_wrong_world_and_missing_prereq() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;

    let capture = LogCapture::install();
    let mut control = stage(&pool, HARSET, vec![completed_708()]).await;
    let accepted = cross_to_castle(&engine, &mut control).await;
    assert!(
        update_at(&accepted, Some(STEP_A)).is_some(),
        "control: Harset with 708 completed must accept 10320"
    );
    drop(capture);

    // (origin world, 10320 state seeded, index of the condition that fails)
    let refused: [(&str, Vec<MissionInstance>, i32); 7] = [
        (CELLBLOCK, vec![completed_708()], 0),
        (CMD_CENTER, vec![completed_708()], 0),
        (DAKARA, vec![completed_708()], 0),
        (CASTLE, vec![completed_708()], 0),
        (HARSET, Vec::new(), 1),
        (HARSET, vec![completed_708(), return_mission()], 2),
        (HARSET, vec![completed_708(), finished_return()], 2),
    ];
    for (origin, missions, failed_index) in refused {
        let capture = LogCapture::install();
        let mut mgr = stage(&pool, origin, missions).await;
        let before = snapshot(&mgr);
        let out = cross_to_castle(&engine, &mut mgr).await;
        assert!(
            out.is_empty(),
            "{origin}: the crossing must fire nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "{origin}: the refused crossing must not move 10320 or the bind"
        );
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::DEBUG
                    && c.target == "content.resolve"
                    && c.has_field("chain_id", &ACCEPT_CHAIN.to_string())
            })
            .collect();
        assert_eq!(rows.len(), 1, "{origin}: {rows:#?}");
        assert!(rows[0].has_field("reason", "condition_failed"), "{rows:#?}");
        assert!(
            rows[0].has_field("failed_condition_index", &failed_index.to_string()),
            "{origin}: condition {failed_index} must be the one that fails; {rows:#?}"
        );
    }
}

// ── Forged close ─────────────────────────────────────────────────────────

/// **A forged close, button or open of dialog 10721 moves nothing.** No chain
/// keys on dialog 10721 or on a dialog event under scope 10320 (checked in
/// the seed), and the real dispatchers confirm it in every return_q1 state:
/// the close (`button -1`), two button ordinals and the open send no traffic
/// and leave 10320 and the Marsh bind as they were. Revert proof: add a
/// `dialog_choice` trigger for 10721 to any 10130-10133 chain and the seed
/// count fails.
#[tokio::test]
async fn live_db_castle_side_return_c2_forged_close_is_inert() {
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

    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;
    for (label, missions) in [
        ("R1 not started", vec![completed_708()]),
        ("R1 at A", vec![completed_708(), return_mission()]),
        ("R1 completed", vec![completed_708(), finished_return()]),
    ] {
        let mut mgr = stage(&pool, CASTLE, missions).await;
        log_in(&engine, &mut mgr, CASTLE).await;
        let before = snapshot(&mgr);
        let (tx, mut rx) = mpsc::channel(256);
        for button in [-1, 0, 1] {
            fire_dialog_choice(ENTITY, PLAYER, DIALOG, button, &engine, &tx, &mut mgr).await;
        }
        fire_dialog_open(ENTITY, PLAYER, DIALOG, &engine, &tx, &mut mgr).await;
        let out = drain(&mut rx);
        assert!(
            out.is_empty(),
            "{label}: forged dialog events must send nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "{label}: forged dialog events moved state"
        );
    }
}
