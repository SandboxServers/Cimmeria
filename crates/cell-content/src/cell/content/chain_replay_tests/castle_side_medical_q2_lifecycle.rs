//! Castle medical side story Q2 (mission 10301), CS-04a2c2: the lifecycle and
//! refusals of chains 10010-10017 that `castle_side_medical_q2.rs` (c1) does
//! not cover. Relog restores each step's chain, abandon rebinds the offer,
//! the Q2 chains refuse outside world 8 and a forged dialog event is inert.
//! The sibling check runs the Q1 chains (10000-10006) and the Q2 chains over
//! the same event keys in every 10300 x 10301 status pair, so a shared event
//! can never resolve two chains of different quests. Fixtures live in
//! `castle_side_medical_q2_stage.rs`. Skip cleanly when DATABASE_URL is unset.

use std::collections::BTreeSet;

use cimmeria_entity::missions::MissionInstance;
use tokio::sync::mpsc;

use super::super::{fire_dialog_choice, fire_dialog_open};
use super::castle_side_medical_q2_stage::*;
use crate::cell::content::fire_mission_abandoned;
use crate::cell::missions::abandon_mission;
use crate::test_support::require_db_or_skip;
// ── Sibling disjointness ─────────────────────────────────────────────────

/// **Every shared event resolves at most one chain across Q1 and Q2, and the
/// slot 177 binding never holds both maps.** For each event key and each of
/// the sixteen 10300 x 10301 status-and-step cells, the union of the fired
/// chains has at most one member; across all cells every Q2 chain resolves
/// somewhere. After a login in each cell, slot 177 never carries both Q1's
/// map 1000001 and Q2's 1000003. Revert proof: remove
/// `mission_status 10300 completed` from 10015 and the medic click at a 10300
/// `not_active` cell resolves both 10003 and 10015.
#[tokio::test]
async fn live_db_castle_side_medical_q2_siblings_disjoint_with_q1() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS.iter().chain(Q2_CHAINS.iter()).copied().collect();
    let engine = engine_with(&pool, &ids).await;
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    for q1 in 0..4 {
        for q2 in 0..4 {
            let mut mgr = staged(&pool, cell(q1, q2)).await;
            for event in event_keys() {
                let fired = fired_by(&engine, &mgr, &event);
                assert!(
                    fired.len() <= 1,
                    "{event:?} in Q1 state {q1}, Q2 state {q2} resolved {fired:?}; \
                     one event must resolve at most one chain across both quests"
                );
                seen.extend(fired);
            }
            log_in(&engine, &mut mgr, CASTLE).await;
            let slot = bound_at(&mgr, TURN_IN_SLOT);
            assert!(
                !(slot.contains(&Q1_SET) && slot.contains(&TURN_IN_SET)),
                "Q1 state {q1}, Q2 state {q2}: slot {TURN_IN_SLOT} carries both \
                 map {Q1_SET} and map {TURN_IN_SET}: {slot:?}"
            );
        }
    }
    let q2_all: BTreeSet<i64> = Q2_CHAINS.iter().map(|&id| id as i64).collect();
    assert!(
        q2_all.is_subset(&seen),
        "every Q2 chain must resolve in some state; missing {:?}",
        q2_all.difference(&seen).collect::<Vec<_>>()
    );
}

// ── Relog ────────────────────────────────────────────────────────────────

/// **Relog restores exactly the chain of each step.** Not started rebinds the
/// offer (10011); at A the login heals to B and binds the turn-in (10016); at
/// B the turn-in is bound (10017); completed binds nothing. Revert proof: drop
/// 10016 and the login at A leaves the mission at A with nothing bound.
#[tokio::test]
async fn live_db_castle_side_medical_q2_relog_each_state() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS.iter().chain(Q2_CHAINS.iter()).copied().collect();
    let engine = engine_with(&pool, &ids).await;

    // (label, the Q2 mission, step after login, offer bound, turn-in bound)
    let cases: [(
        &str,
        Option<MissionInstance>,
        Option<i32>,
        Vec<i32>,
        Vec<i32>,
    ); 4] = [
        ("not started", None, None, vec![OFFER_SET], vec![]),
        (
            "at A",
            Some(active_at(MISSION, STEP_A, &[OBJ_A])),
            Some(STEP_B),
            vec![],
            vec![TURN_IN_SET],
        ),
        (
            "at B",
            Some(active_at(MISSION, STEP_B, &[OBJ_B])),
            Some(STEP_B),
            vec![],
            vec![TURN_IN_SET],
        ),
        ("completed", Some(completed(MISSION)), None, vec![], vec![]),
    ];
    for (label, q2, step, offer, turn_in) in cases {
        let mut missions = vec![completed(Q1)];
        missions.extend(q2);
        let mut mgr = staged(&pool, missions).await;
        log_in(&engine, &mut mgr, CASTLE).await;
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            offer,
            "{label}: the prisoner offer after relog"
        );
        assert_eq!(
            bound_at(&mgr, TURN_IN_SLOT),
            turn_in,
            "{label}: the medic turn-in after relog"
        );
        assert_eq!(step_of(&mgr), step, "{label}: the step after relog");
    }
}

// ── Abandon ──────────────────────────────────────────────────────────────

/// **An abandon at A or at B rebinds the offer and unbinds the medic.** The
/// real abandon removes 10301, then `mission_abandoned` fires 10012: the
/// prisoner offer is bound again and the turn-in is gone. At B the turn-in is
/// bound first (by the login, 10017) so the unbind is observable. Revert
/// proof: drop 10012's remove action and the turn-in stays bound after the
/// abandon at B.
#[tokio::test]
async fn live_db_castle_side_medical_q2_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS.iter().chain(Q2_CHAINS.iter()).copied().collect();
    let engine = engine_with(&pool, &ids).await;

    for (label, step, objective, relog) in [
        ("at A", STEP_A, OBJ_A, false),
        ("at B", STEP_B, OBJ_B, true),
    ] {
        let mut mgr = staged(
            &pool,
            vec![completed(Q1), active_at(MISSION, step, &[objective])],
        )
        .await;
        if relog {
            log_in(&engine, &mut mgr, CASTLE).await;
            assert_eq!(
                bound_at(&mgr, TURN_IN_SLOT),
                vec![TURN_IN_SET],
                "{label}: the turn-in must be bound before the abandon"
            );
        }
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{label}: the abandon must remove 10301"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "{label}: 10301 must be gone after the abandon"
        );
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            vec![OFFER_SET],
            "{label}: mission_abandoned must rebind the prisoner offer (10012)"
        );
        assert!(
            bound_at(&mgr, TURN_IN_SLOT).is_empty(),
            "{label}: mission_abandoned must unbind the medic (10012)"
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **Every Q2 chain refuses outside world 8.** In CombatSim, with each Q1 and
/// Q2 status cell, no event resolves a Q2 chain (the world condition fails
/// first), and a login binds neither the offer nor the turn-in. Revert proof:
/// drop a chain's `world` condition and that chain resolves in CombatSim.
#[tokio::test]
async fn live_db_castle_side_medical_q2_wrong_world_refused() {
    let pool = require_db_or_skip!();
    let ids: Vec<i32> = Q1_CHAINS.iter().chain(Q2_CHAINS.iter()).copied().collect();
    let engine = engine_with(&pool, &ids).await;
    let q2_all: BTreeSet<i64> = Q2_CHAINS.iter().map(|&id| id as i64).collect();

    for q1 in 0..4 {
        for q2 in 0..4 {
            let mut mgr = staged_in(&pool, COMBAT_SIM, cell(q1, q2)).await;
            for event in event_keys() {
                let fired = fired_by(&engine, &mgr, &event);
                assert!(
                    fired.is_disjoint(&q2_all),
                    "{event:?} in CombatSim, Q1 state {q1}, Q2 state {q2} resolved Q2 chains {fired:?}"
                );
            }
            log_in(&engine, &mut mgr, COMBAT_SIM).await;
            assert!(
                bound_at(&mgr, OFFER_SLOT).is_empty() && bound_at(&mgr, TURN_IN_SLOT).is_empty(),
                "Q1 state {q1}, Q2 state {q2}: a login in CombatSim must bind nothing"
            );
        }
    }
}

/// **A forged dialog event of 10701 or 10702 moves nothing.** The seed keys no
/// chain on either dialog (no `dialog_choice` or `dialog_open` trigger with
/// those keys, and no dialog event under scope 10301), and the real dispatchers
/// confirm it: the close (`button -1`), button ordinals and the open an
/// `initialResponse` ends in send no traffic and leave 10301, the two bindings
/// and the dialogs on screen as they were. Revert proof: add a `dialog_choice`
/// trigger for 10701 to any Q2 chain and the seed count fails.
#[tokio::test]
async fn live_db_castle_side_medical_q2_forged_close_is_inert() {
    let pool = require_db_or_skip!();
    let keyed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE (t.event_type IN ('dialog_choice', 'dialog_open') AND t.event_key IN ($1, $2)) \
            OR (c.scope_type = 'mission' AND c.scope_id = $3 \
                AND t.event_type IN ('dialog_choice', 'dialog_open', 'dialog_set_open'))",
    )
    .bind(ACCEPT_DIALOG.to_string())
    .bind(TURN_IN_DIALOG.to_string())
    .bind(MISSION)
    .fetch_one(&pool)
    .await
    .expect("dialog trigger query must succeed");
    assert_eq!(
        keyed, 0,
        "no chain may key on dialogs {ACCEPT_DIALOG} or {TURN_IN_DIALOG} or on a dialog event under scope {MISSION}"
    );

    let ids: Vec<i32> = Q1_CHAINS.iter().chain(Q2_CHAINS.iter()).copied().collect();
    let engine = engine_with(&pool, &ids).await;
    for q2 in 0..4 {
        let mut mgr = staged(&pool, cell(3, q2)).await;
        log_in(&engine, &mut mgr, CASTLE).await;
        let before = (
            step_of(&mgr),
            bound_at(&mgr, OFFER_SLOT),
            bound_at(&mgr, TURN_IN_SLOT),
            offered(&mgr),
        );
        let (tx, mut rx) = mpsc::channel(256);
        for dialog in [ACCEPT_DIALOG, TURN_IN_DIALOG] {
            for button in [-1, 0, 1] {
                fire_dialog_choice(ENTITY, PLAYER, dialog, button, &engine, &tx, &mut mgr).await;
            }
            fire_dialog_open(ENTITY, PLAYER, dialog, &engine, &tx, &mut mgr).await;
        }
        assert!(
            drain(&mut rx).is_empty(),
            "Q2 state {q2}: forged dialog events sent traffic"
        );
        let after = (
            step_of(&mgr),
            bound_at(&mgr, OFFER_SLOT),
            bound_at(&mgr, TURN_IN_SLOT),
            offered(&mgr),
        );
        assert_eq!(
            after, before,
            "Q2 state {q2}: forged dialog events moved state"
        );
    }
}
