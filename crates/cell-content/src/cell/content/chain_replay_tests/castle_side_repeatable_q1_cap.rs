//! Castle repeatable_q1 Supply Run (mission 10330), CS-05b1c2: the cap and
//! the persistence of `repeats` over chains 10230-10237. The load, the accept,
//! the flow and the login are in `castle_side_repeatable_q1` and
//! `castle_side_repeatable_q1_flow`, whose harness this file shares.
//!
//! * three full runs complete, and the fourth accept is refused with
//!   `repeat_cap_reached` while the refused click still shows 10729 and
//!   removes the offer;
//! * `repeats` survives a relog and an abandon of an in-flight attempt, and a
//!   completed record cannot be abandoned;
//! * the cap belongs to the character, not the account;
//! * an unreadable saved state refuses the accept and persists nothing.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE,
};
use tokio::sync::mpsc;

use super::super::{fire_enter_region, fire_interact_tag, fire_mission_abandoned};
use super::castle_side_repeatable_q1::{
    bound_at, click_officer, completed_704, drain, engine_with, log_in, staged, supply_mission,
    ALL_CHAINS, CASTLE, ENTITY, MISSION, OBJ_A, OBJ_B, OFFER_SET, OFFICER, OFFICER_TAG, PLAYER,
    SLOT, STEP_A, STEP_B,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::{abandon_mission, accept_mission, state_lock};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{require_db_or_skip, LogCapture};

const INFIRMARY: &str = "Castle.Infirmary";
/// The offer dialog the click shows; the same id as `castle_side_repeatable_q1`.
const OFFER_DIALOG: i32 = 10729;
/// `num_repeats = 2` in the seed, so the cap admits N + 1 = 3 completions.
const CAP_RUNS: i32 = 3;
/// A second character on the same account, for the per-character cap.
const SECOND: u32 = 2;
const SECOND_PLAYER: i32 = 43;

// ── Harness ──────────────────────────────────────────────────────────────

/// 10330 as a finished run: COMPLETED, no current step, `repeats` runs done.
fn finished_run(repeats: i32) -> MissionInstance {
    let mut mission = supply_mission(STEP_B, &[OBJ_B]);
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission.repeats = repeats;
    mission
}

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

/// `(status, repeats)` of 10330, or `None` when the record is absent.
fn status_and_repeats(mgr: &SpaceManager) -> Option<(i8, i32)> {
    mission_of(mgr).map(|m| (m.status, m.repeats))
}

/// Position of the `onDialogDisplay` call for `dialog_id` in `out`.
fn dialog_at(out: &[CellToBaseMsg], dialog_id: i32) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == ENTITY
                    && *method_index == super::castle_side_repeatable_q1::ON_DIALOG_DISPLAY
                    && args.len() >= 8
                    && i32::from_le_bytes(args[4..8].try_into().unwrap()) == dialog_id
        )
    })
}

/// True when `out` persists 10330 (a `MissionUpdate` for it).
fn persists_supply(out: &[CellToBaseMsg]) -> bool {
    out.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )
    })
}

async fn enter_infirmary(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, INFIRMARY, engine, &tx, mgr).await;
}

/// Play one run from the offer: the officer click accepts, the Infirmary
/// entry advances to B, and the turn-in click completes the run.
async fn play_run(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (handled, _) = click_officer(engine, mgr).await;
    assert!(handled, "the offer click must accept a run");
    enter_infirmary(engine, mgr).await;
    let (handled, _) = click_officer(engine, mgr).await;
    assert!(handled, "the turn-in click must complete the run");
}

// ── The cap ──────────────────────────────────────────────────────────────

/// **Three full runs complete; the fourth accept is refused.** Each run
/// leaves `repeats` one higher. The fourth click matches 10234 but the offer
/// guard refuses the accept (`repeat_cap_reached`): the status stays
/// COMPLETED, no `mission_accepted` event fires and no `MissionUpdate` for
/// 10330 is sent. Revert proof: delete the `MISSION_COMPLETED if repeats >
/// num_repeats` arm at `lifecycle.rs:95`; the fourth accept succeeds and this
/// test fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_three_runs_then_refused() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704()]).await;
    log_in(&engine, &mut mgr).await;

    for run in 1..=CAP_RUNS {
        play_run(&engine, &mut mgr).await;
        assert_eq!(
            status_and_repeats(&mgr),
            Some((MISSION_COMPLETED, run)),
            "run {run} must complete and count one run"
        );
    }

    let capture = LogCapture::install();
    let (handled, out) = click_officer(&engine, &mut mgr).await;
    assert!(handled, "the refused click still matches 10234");
    assert!(
        !persists_supply(&out),
        "a refused accept must not persist 10330; got {out:?}"
    );
    assert_eq!(
        status_and_repeats(&mgr),
        Some((MISSION_COMPLETED, CAP_RUNS)),
        "the refused accept must leave the record COMPLETED with {CAP_RUNS} runs"
    );
    let rows = capture.all();
    assert!(
        rows.iter()
            .any(|c| c.has_field("reason", "repeat_cap_reached")),
        "the refusal must log repeat_cap_reached; got {rows:#?}"
    );
    assert!(
        !rows
            .iter()
            .any(|c| c.has_field("event", "mission_accepted")),
        "no mission_accepted event may fire on a refused accept"
    );
}

/// **The refused click still shows 10729 and removes the offer.** The login
/// after the cap offers 10330 (10231 cannot see `repeats`), and the click
/// that is refused continues its action list: the offer binding comes off and
/// the neutral dialog shows. Revert proof: delete the `display_dialog` action
/// from 10234 (the dialog assertion fails), or its `remove_dialog_set` action
/// (the unbind assertion fails).
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_refused_click_still_shows_dialog_and_unbinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704(), finished_run(CAP_RUNS)]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, SLOT),
        vec![OFFER_SET],
        "the login after the cap must still offer 10330"
    );

    let (handled, out) = click_officer(&engine, &mut mgr).await;
    assert!(handled, "the refused click still matches 10234");
    assert!(
        dialog_at(&out, OFFER_DIALOG).is_some(),
        "dialog 10729 must show after the refused accept; got {out:?}"
    );
    assert!(
        bound_at(&mgr, SLOT).is_empty(),
        "the offer binding must be removed by the refused click"
    );
    assert!(!persists_supply(&out), "a refused accept persists nothing");
    assert_eq!(
        status_and_repeats(&mgr),
        Some((MISSION_COMPLETED, CAP_RUNS)),
        "the refused click must not move 10330"
    );
}

// ── Persistence ──────────────────────────────────────────────────────────

/// **`repeats` survives a relog and an abandon, and a completed record cannot
/// be abandoned.** A relog keeps the count. An abandon of an in-flight attempt
/// keeps `repeats` (the attempt does not count toward the cap) and rebinds the
/// offer. An abandon of the COMPLETED record at the cap is refused and the
/// record is unchanged. Revert proof: remove the `status != MISSION_ACTIVE`
/// check at `entity/src/missions.rs:253`; abandoning the COMPLETED record
/// overwrites it and this test fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_repeats_survive_relog_and_abandon() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut relogged = staged(&pool, vec![completed_704(), finished_run(2)]).await;
    log_in(&engine, &mut relogged).await;
    assert_eq!(
        status_and_repeats(&relogged),
        Some((MISSION_COMPLETED, 2)),
        "a relog must keep the completed record and its repeats"
    );

    let mut attempt = supply_mission(STEP_A, &[OBJ_A]);
    attempt.repeats = 2;
    let mut in_flight = staged(&pool, vec![completed_704(), attempt]).await;
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, &mut in_flight).await,
        "the in-flight attempt must abandon"
    );
    fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut in_flight).await;
    assert_eq!(
        status_and_repeats(&in_flight),
        Some((MISSION_NOT_ACTIVE, 2)),
        "an abandoned attempt keeps repeats and does not consume a run"
    );
    assert_eq!(
        bound_at(&in_flight, SLOT),
        vec![OFFER_SET],
        "the abandon must rebind the offer"
    );

    let mut finished = staged(&pool, vec![completed_704(), finished_run(CAP_RUNS)]).await;
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        !abandon_mission(ENTITY, MISSION, &tx, &mut finished).await,
        "a COMPLETED record must not be abandoned"
    );
    assert_eq!(
        status_and_repeats(&finished),
        Some((MISSION_COMPLETED, CAP_RUNS)),
        "the completed record at the cap must survive an abandon"
    );
}

/// **The cap is per character.** Character 1 is at the cap; character 2 on the
/// same account has no record and accepts 10330 at once. Then character 1's
/// click is still refused and character 2's record is unaffected by it.
/// Revert proof: key the repeat counter by account instead of character; the
/// second character is refused and this test fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_two_characters_independent() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704(), finished_run(CAP_RUNS)]).await;
    mgr.create_entity(SECOND, CASTLE, [0.0; 3], [0.0; 3])
        .unwrap();
    {
        let second = mgr.get_entity_mut(SECOND).unwrap();
        second.is_player = true;
        second.player_id = Some(SECOND_PLAYER);
        second.archetype_id = Some(1);
        second.missions.add_mission(completed_704());
    }

    let (tx, _rx) = mpsc::channel(256);
    let handled = fire_interact_tag(
        SECOND,
        SECOND_PLAYER,
        OFFICER_TAG,
        OFFICER,
        &engine,
        &tx,
        &mut mgr,
    )
    .await;
    assert!(handled, "the second character's click must accept");
    let second = mgr
        .get_entity(SECOND)
        .unwrap()
        .missions
        .get_mission(MISSION);
    assert_eq!(
        second.map(|m| (m.status, m.repeats)),
        Some((MISSION_ACTIVE, 0)),
        "the second character must accept its first run with repeats 0"
    );

    let (handled, out) = click_officer(&engine, &mut mgr).await;
    assert!(handled, "character 1's click still matches 10234");
    assert!(
        !persists_supply(&out),
        "character 1 at the cap must stay refused"
    );
    assert_eq!(
        status_and_repeats(&mgr),
        Some((MISSION_COMPLETED, CAP_RUNS)),
        "character 1 must stay at the cap"
    );
}

// ── Unreadable state ─────────────────────────────────────────────────────

/// **An unreadable saved state cannot reset the cap.** With the tracker locked
/// (`state_lock::mark_state_unknown`), the accept is refused before the offer
/// guard and nothing is persisted. The record is absent, so without the lock
/// the accept would proceed with `repeats` 0. The click path is also gated in
/// the chain executor and at the persist site, so the test calls
/// `accept_mission` directly as well. Revert proof: remove the
/// `refuse_if_state_unknown` call at `lifecycle.rs:51`; the direct accept
/// succeeds on the empty tracker and creates the record, and this test fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_cap_unreadable_state_cannot_reset() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704()]).await;
    state_lock::mark_state_unknown(ENTITY, &mut mgr);

    let (_handled, out) = click_officer(&engine, &mut mgr).await;
    assert!(
        !persists_supply(&out),
        "a locked tracker must persist nothing for 10330; got {out:?}"
    );
    assert!(
        mission_of(&mgr).is_none_or(|m| m.status != MISSION_ACTIVE),
        "a locked tracker must not start 10330"
    );

    // The primitive itself, below the chain gate and the persist gate, which
    // would each hide a missing guard in the click above.
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        !accept_mission(ENTITY, MISSION, STEP_A, Vec::new(), &tx, &mut mgr).await,
        "accept_mission must refuse while the saved state is unknown"
    );
    assert!(
        mission_of(&mgr).is_none(),
        "a refused accept must not create the record"
    );
    assert!(
        !persists_supply(&drain(&mut rx)),
        "a refused accept must persist nothing"
    );
}
