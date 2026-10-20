//! Castle repeatable_q1 Supply Run (mission 10330), CS-05b1c1: the rest of the
//! mission flow over chains 10230-10237. The load and the accept are in
//! `castle_side_repeatable_q1`, whose harness this file shares.
//!
//! * the Infirmary entry advances A to B and binds the turn-in, only at A;
//! * the turn-in click completes the mission and the nested chain rebinds
//!   the offer;
//! * each login step has one owner, an abandon rebinds the offer, the wrong
//!   world and a missing 704 refuse the click, and no event key resolves two
//!   chains.
//!
//! Every event goes through the real dispatchers, so a dispatcher that skips
//! `populate_world_context` or the mission context fails here, not only a
//! missing condition row.

use std::collections::BTreeSet;

use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{
    MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE,
};
use tokio::sync::mpsc;
use tracing::Level;

use super::super::mission_context::{populate_mission_context, populate_world_context};
use super::super::{fire_enter_region, fire_mission_abandoned};
use super::castle_side_repeatable_q1::{
    bound_at, click_officer, completed_704, engine_with, log_in, offered, staged, staged_in,
    supply_mission, ALL_CHAINS, CASTLE, ENTITY, MISSION, OBJ_A, OBJ_B, OFFER_SET, OFFICER_TAG,
    ON_DIALOG_DISPLAY, PLAYER, PREREQ, SLOT, STEP_A, STEP_B, TURN_IN_DIALOG, TURN_IN_SET,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{require_db_or_skip, LogCapture};

const INFIRMARY: &str = "Castle.Infirmary";
/// `resources.worlds` row 1; any world other than 8 refuses the officer.
const COMBAT_SIM: &str = "CombatSim";
/// The officer-click accept chain, the only one that accepts.
const ACCEPT_CHAIN: i64 = 10234;

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

/// What a lifecycle event may change on 10330: status, step and repeats.
fn snapshot(mgr: &SpaceManager) -> (Option<(i8, Option<i32>, i32)>, Vec<i32>, Vec<i32>) {
    (
        mission_of(mgr).map(|m| (m.status, m.current_step_id, m.repeats)),
        bound_at(mgr, SLOT),
        offered(mgr),
    )
}

async fn enter_infirmary(
    engine: &cimmeria_content_engine::chain::ChainEngine,
    mgr: &mut SpaceManager,
) {
    let (tx, _rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, INFIRMARY, engine, &tx, mgr).await;
}

/// Position of the `onDialogDisplay` call for `dialog_id` in `out`.
fn dialog_at(out: &[CellToBaseMsg], dialog_id: i32) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == ENTITY
                    && *method_index == ON_DIALOG_DISPLAY
                    && args.len() >= 8
                    && i32::from_le_bytes(args[4..8].try_into().unwrap()) == dialog_id
        )
    })
}

// ── The Infirmary ────────────────────────────────────────────────────────

/// **The Infirmary entry advances A to B and binds the turn-in, at A only.**
/// At A the entry fires 10235: step B active, the turn-in bound at slot 424.
/// At B, not_active or completed the gate fails and nothing moves. Revert
/// proof: remove 10235's `step_status 10384 active` condition and the entry at
/// B binds the turn-in again.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_region_advances_only_at_a() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut at_a = staged(
        &pool,
        vec![completed_704(), supply_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    enter_infirmary(&engine, &mut at_a).await;
    let mission = mission_of(&at_a).expect("10330 must stay on the record");
    assert_eq!(
        mission.status, MISSION_ACTIVE,
        "the advance keeps 10330 active"
    );
    assert_eq!(
        mission.current_step_id,
        Some(STEP_B),
        "the entry at A must advance to B"
    );
    assert_eq!(
        bound_at(&at_a, SLOT),
        vec![TURN_IN_SET],
        "the entry at A must bind the turn-in"
    );

    let cases: [(&str, Vec<MissionInstance>); 3] = [
        (
            "at B",
            vec![completed_704(), supply_mission(STEP_B, &[OBJ_B])],
        ),
        ("not_active", vec![completed_704()]),
        ("completed", vec![completed_704(), finished_run(1)]),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, missions).await;
        let before = snapshot(&mgr);
        enter_infirmary(&engine, &mut mgr).await;
        assert_eq!(
            snapshot(&mgr),
            before,
            "entering the Infirmary {label} must change nothing"
        );
    }
}

// ── The turn-in ──────────────────────────────────────────────────────────

/// **The click at B shows 10730, completes 10330 and rebinds the offer.**
/// 10236 removes the turn-in, shows the dialog, then completes: `repeats`
/// becomes 1 and the nested 10233 binds the offer for the next run. The
/// dialog goes out before the completion update. The click at A is handled by
/// no chain and changes nothing. Revert proof: remove 10236's `step_status
/// 10385 active` condition and the click at A completes the mission.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_turn_in_completes_and_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut at_b = staged(
        &pool,
        vec![completed_704(), supply_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    log_in(&engine, &mut at_b).await;
    assert_eq!(
        bound_at(&at_b, SLOT),
        vec![TURN_IN_SET],
        "the turn-in must be bound before the click"
    );
    let (handled, out) = click_officer(&engine, &mut at_b).await;
    assert!(handled, "chain 10236 must handle the click at B");
    let dialog = dialog_at(&out, TURN_IN_DIALOG).expect("10730 must be shown");
    let complete = out
        .iter()
        .position(|m| {
            matches!(
                m,
                CellToBaseMsg::MissionUpdate {
                    mission_id: MISSION,
                    status: MISSION_COMPLETED,
                    repeats: 1,
                    ..
                }
            )
        })
        .expect("the completion must persist 10330 with repeats 1");
    assert!(
        dialog < complete,
        "traffic order must be dialog, then completion; got {dialog}, {complete}"
    );
    let mission = mission_of(&at_b).expect("a completed run stays on the record");
    assert_eq!(mission.status, MISSION_COMPLETED, "10330 must complete");
    assert_eq!(mission.repeats, 1, "the completion must count one run");
    assert_eq!(
        bound_at(&at_b, SLOT),
        vec![OFFER_SET],
        "the turn-in must be gone and the offer rebound by 10233"
    );
    assert_eq!(offered(&at_b), vec![TURN_IN_DIALOG], "10730 must be shown");

    let mut at_a = staged(
        &pool,
        vec![completed_704(), supply_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    let before = snapshot(&at_a);
    let (handled, out) = click_officer(&engine, &mut at_a).await;
    assert!(!handled, "no chain may handle the click at A");
    assert!(
        out.is_empty(),
        "the click at A must send nothing; got {out:?}"
    );
    assert_eq!(
        snapshot(&at_a),
        before,
        "the click at A must change nothing"
    );
}

// ── The login ────────────────────────────────────────────────────────────

/// **Each login step has one owner.** Not active or finished (cap not reached)
/// the login rebinds the offer (10231) and nothing else; at A it binds
/// nothing; at B it rebinds the turn-in (10237) and not the offer. Revert
/// proof: delete chain 10237 and the login at B binds nothing.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_relog_each_step() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let cases: [(&str, Vec<MissionInstance>, Vec<i32>); 4] = [
        ("not_active", vec![completed_704()], vec![OFFER_SET]),
        (
            "finished once",
            vec![completed_704(), finished_run(1)],
            vec![OFFER_SET],
        ),
        (
            "at A",
            vec![completed_704(), supply_mission(STEP_A, &[OBJ_A])],
            vec![],
        ),
        (
            "at B",
            vec![completed_704(), supply_mission(STEP_B, &[OBJ_B])],
            vec![TURN_IN_SET],
        ),
    ];
    for (label, missions, want) in cases {
        let mut mgr = staged(&pool, missions).await;
        let before = mission_of(&mgr).map(|m| (m.status, m.current_step_id, m.repeats));
        log_in(&engine, &mut mgr).await;
        assert_eq!(
            bound_at(&mgr, SLOT),
            want,
            "a login {label} must leave exactly this binding at slot 424"
        );
        assert_eq!(
            mission_of(&mgr).map(|m| (m.status, m.current_step_id, m.repeats)),
            before,
            "a login {label} must not move 10330"
        );
        assert!(offered(&mgr).is_empty(), "a login {label} shows no dialog");
    }
}

// ── The abandon ──────────────────────────────────────────────────────────

/// **An abandon at A or at B swaps to the offer and keeps `repeats`.** The
/// real abandon removes the active attempt (a record with `repeats` stays as
/// NOT_ACTIVE), then `mission_abandoned` fires 10232: the turn-in comes off
/// and the offer goes on. Revert proof: delete chain 10232 and no offer is
/// bound.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for (label, step, objective, repeats) in [
        ("at A", STEP_A, OBJ_A, 0),
        ("at B", STEP_B, OBJ_B, 0),
        ("at B on run 2", STEP_B, OBJ_B, 1),
    ] {
        let mut attempt = supply_mission(step, &[objective]);
        attempt.repeats = repeats;
        let mut mgr = staged(&pool, vec![completed_704(), attempt]).await;
        log_in(&engine, &mut mgr).await;
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{label}: the abandon must end the attempt"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert_eq!(
            bound_at(&mgr, SLOT),
            vec![OFFER_SET],
            "{label}: mission_abandoned must leave only the offer (10232)"
        );
        match (repeats, mission_of(&mgr)) {
            (0, None) => {}
            (0, Some(m)) => panic!("{label}: no run was spent, yet 10330 stays as {m:?}"),
            (n, Some(m)) => {
                assert_eq!(m.repeats, n, "{label}: the abandon must keep repeats");
                assert_eq!(m.status, MISSION_NOT_ACTIVE, "{label}: not_active");
            }
            (n, None) => panic!("{label}: repeats {n} must survive the abandon"),
        }
    }
}

// ── Wrong world, missing 704 ─────────────────────────────────────────────

/// **The officer click accepts only in world 8 with 704 completed.** A
/// positive control (world 8, 704 completed, 10330 not active) is handled by
/// 10234, so the refusals cannot pass vacuously. The same click in world 1, in
/// the Castle with 704 active and with no 704 is handled by nothing, sends
/// nothing, starts nothing, and logs a `condition_failed` row for chain 10234
/// naming the failing condition (0 = world, 1 = `mission_status 704`). Revert
/// proof: drop 10234's `world` condition and the world-1 case accepts.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_wrong_world_and_missing_704_refused() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut mgr = staged(&pool, vec![completed_704()]).await;
    let (handled, out) = click_officer(&engine, &mut mgr).await;
    assert!(
        handled,
        "control: chain 10234 must accept in world 8 after 704"
    );
    assert!(
        out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )),
        "control: the accept must persist 10330"
    );

    let active_704 = MissionInstance::new(PREREQ, 0, Vec::new());
    // (world, 704 as held, index of the condition that must fail)
    let refused: [(&str, Vec<MissionInstance>, usize); 3] = [
        (COMBAT_SIM, vec![completed_704()], 0),
        (CASTLE, vec![active_704], 1),
        (CASTLE, vec![], 1),
    ];
    for (world, missions, failed_index) in refused {
        let capture = LogCapture::install();
        let mut mgr = staged_in(&pool, world, missions).await;
        let want_world = if world == CASTLE { 8 } else { 1 };
        assert_eq!(
            mgr.get_entity_world_id(ENTITY),
            Some(want_world),
            "{world}: the player must be staged in world {want_world}"
        );
        let (handled, out) = click_officer(&engine, &mut mgr).await;
        assert!(!handled, "{world}: no chain may handle the officer click");
        assert!(
            !out.iter().any(|m| matches!(
                m,
                CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
            )),
            "{world}: no mission or dialog traffic; got {out:?}"
        );
        assert!(mission_of(&mgr).is_none(), "{world}: 10330 must not start");
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::DEBUG
                    && c.target == "content.resolve"
                    && c.has_field("chain_id", &ACCEPT_CHAIN.to_string())
            })
            .collect();
        assert_eq!(rows.len(), 1, "{world}: {rows:#?}");
        assert!(rows[0].has_field("reason", "condition_failed"), "{rows:#?}");
        assert!(
            rows[0].has_field("failed_condition_index", &failed_index.to_string()),
            "{world}: condition {failed_index} must be the one that fails; {rows:#?}"
        );
    }
}

// ── Sibling disjointness ─────────────────────────────────────────────────

/// The six event keys the eight chains listen on, as the dispatchers build
/// them: the trigger type and the one param that carries the key.
fn event_keys() -> [(TriggerType, Option<(&'static str, serde_json::Value)>); 6] {
    [
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", PREREQ.into())),
        ),
        (TriggerType::PlayerLoaded, None),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", OFFICER_TAG.into())),
        ),
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::RegionEnter,
            Some(("region_key", INFIRMARY.into())),
        ),
    ]
}

/// **For each event key and every reachable 704/10330 state in the Castle, at
/// most one of chains 10230-10237 resolves; across all of them every chain
/// resolves somewhere.** The contexts are built as the dispatchers build them
/// (world, mission state, archetype, the key param). The second half stops a
/// loader that drops a chain from passing the first half vacuously. Revert
/// proof: remove 10231's `mission_status 10330 neq active` condition and
/// 10231 and 10237 both resolve on `player_loaded` at step B.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    for prereq in ["absent", "active", "completed"] {
        for run in ["absent", "at A", "at B", "finished"] {
            let mut held = Vec::new();
            match prereq {
                "active" => held.push(MissionInstance::new(PREREQ, 0, Vec::new())),
                "completed" => held.push(completed_704()),
                _ => {}
            }
            match run {
                "at A" => held.push(supply_mission(STEP_A, &[OBJ_A])),
                "at B" => held.push(supply_mission(STEP_B, &[OBJ_B])),
                "finished" => held.push(finished_run(1)),
                _ => {}
            }
            let mgr = staged(&pool, held).await;
            for (trigger_type, key) in event_keys() {
                let mut ctx =
                    ExecutionContext::new().with_source(cimmeria_common::EntityId(ENTITY as i32));
                if let Some((name, value)) = key {
                    ctx.set_param(name.to_string(), value);
                }
                populate_world_context(ENTITY, &mgr, &mut ctx);
                populate_mission_context(mgr.get_entity(ENTITY).unwrap(), &mut ctx);
                ctx.set_param("archetype".to_string(), serde_json::json!(1));
                let event = TriggerEvent {
                    trigger_type: trigger_type.clone(),
                    source_entity: Some(cimmeria_common::EntityId(ENTITY as i32)),
                    target_entity: None,
                    params: ctx.params.clone(),
                };
                let fired: BTreeSet<i64> = engine
                    .resolve_event(&event, &ctx)
                    .actions
                    .iter()
                    .map(|(id, _)| *id)
                    .collect();
                assert!(
                    fired.len() <= 1,
                    "{trigger_type:?} with 704 {prereq}, 10330 {run} resolved {fired:?}; \
                     sibling chains must be pairwise disjoint on the 10330 state"
                );
                seen.extend(fired);
            }
        }
    }
    let all: BTreeSet<i64> = ALL_CHAINS.iter().map(|&id| id as i64).collect();
    assert_eq!(
        seen, all,
        "every Supply Run chain must resolve in some reachable state"
    );
}
