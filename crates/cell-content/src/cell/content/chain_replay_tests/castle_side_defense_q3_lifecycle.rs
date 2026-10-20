//! Castle defense_q3 side story (mission 10312, *Hold the Bravo Route*),
//! CS-04b3c2: the lifecycle after the offer. A login rebinds the offer while
//! 10312 is not_active (10121) and changes nothing while it runs or after
//! completion. An abandon at 0..3 of 4 rebinds the offer (10122), and a
//! re-accept starts at 0 of 4. Refusals fire nothing. Every event goes through
//! the real dispatcher, so the executor arms run; the chain shapes are pinned
//! by [`super::castle_side_defense_q3_chains`] and the replays by
//! [`super::castle_side_defense_q3`]. The ids and helpers are in `harness`.

mod harness;

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, STATUS_ACTIVE};
use tokio::sync::mpsc;

use self::harness::*;
use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_dialog_choice, fire_dialog_open};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::missions::abandon_mission;
use crate::test_support::require_db_or_skip;

// ── The offer after a login ──────────────────────────────────────────────

/// **A login rebinds the offer only while 10312 is not_active (10121).** With
/// 10310 completed and no 10312, the login binds 1000010 at slot 425. While
/// 10312 runs at 0, 2 or 3 of 4, the login changes nothing, the completed set
/// keeps its entries and the remaining kill still counts. Completed binds
/// nothing. Revert proof: delete chain 10121 and the not_active login binds
/// nothing.
#[tokio::test]
async fn live_db_castle_side_defense_q3_relog_each_stage() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut not_active = staged(&pool, CASTLE, vec![completed_prereq()]).await;
    log_in(&engine, &mut not_active).await;
    assert_eq!(
        bound_at(&not_active, OFFER_SLOT),
        vec![OFFER_SET],
        "a not_active login must rebind the offer (10121)"
    );
    assert!(
        mission_of(&not_active).is_none(),
        "a not_active login must not create 10312"
    );

    for done in [0usize, 2, 3] {
        let mut mgr = staged(
            &pool,
            CASTLE,
            vec![completed_prereq(), active_mission(&OBJECTIVES[..done])],
        )
        .await;
        let before = snapshot(&mgr);
        log_in(&engine, &mut mgr).await;
        assert_eq!(
            snapshot(&mgr),
            before,
            "{done} of 4: a login while active must change nothing"
        );
        kill(&engine, &mut mgr, KILL_TAGS[done]).await;
        let completed = &mission_of(&mgr).unwrap().completed_objectives;
        assert!(
            completed.contains(&OBJECTIVES[done]),
            "{done} of 4: the remaining kill must still count; got {completed:?}"
        );
    }

    let mut completed = staged(&pool, CASTLE, vec![completed_prereq(), finished_mission()]).await;
    let before = snapshot(&completed);
    log_in(&engine, &mut completed).await;
    assert_eq!(
        snapshot(&completed),
        before,
        "a login after completion must bind nothing and move nothing"
    );
}

// ── The abandon ──────────────────────────────────────────────────────────

/// **An abandon at 0..3 of 4 rebinds the offer, and a re-accept starts over.**
/// The real abandon removes 10312, then `mission_abandoned` fires 10122, which
/// binds the offer again. The next click on the giver accepts at step 10366
/// with all four objectives active and none completed. Revert proof: delete
/// chain 10122 and the giver stays unbound after the abandon.
#[tokio::test]
async fn live_db_castle_side_defense_q3_abandon_rebinds_and_resets() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for done in 0..=3usize {
        let mut mgr = staged(
            &pool,
            CASTLE,
            vec![completed_prereq(), active_mission(&OBJECTIVES[..done])],
        )
        .await;
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{done} of 4: the abandon must remove 10312"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert!(
            mission_of(&mgr).is_none(),
            "{done} of 4: 10312 must be gone after the abandon"
        );
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            vec![OFFER_SET],
            "{done} of 4: mission_abandoned must rebind the offer (10122)"
        );

        let (handled, _out) = click_giver(&engine, &mut mgr).await;
        assert!(
            handled,
            "{done} of 4: the re-accept must be handled by 10123"
        );
        let mission = mission_of(&mgr).expect("the re-accept must create 10312");
        assert_eq!(mission.status, MISSION_ACTIVE, "{done} of 4: re-accepted");
        assert_eq!(
            mission.current_step_id,
            Some(STEP),
            "{done} of 4: the re-accept starts at step 10366"
        );
        assert!(
            mission.completed_objectives.is_empty(),
            "{done} of 4: the re-accept starts at 0 of 4; got {:?}",
            mission.completed_objectives
        );
        assert_eq!(
            mission.active_objectives.len(),
            OBJECTIVES.len(),
            "{done} of 4: all four objectives must be active again"
        );
        assert!(
            mission
                .active_objectives
                .iter()
                .all(|o| o.status == STATUS_ACTIVE),
            "{done} of 4: every objective must be active"
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **Each refusal fires nothing.** The giver click in the wrong world, with
/// 10310 missing or still running, with 10312 already active and with 10312
/// already completed sends no mission traffic, leaves 10312 and the offer as
/// they were, and is not handled by a chain. Revert proof: drop
/// `mission_status 10310 completed` from 10123 and the not-completed case
/// accepts.
#[tokio::test]
async fn live_db_castle_side_defense_q3_refusals() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: [(&str, &str, Vec<MissionInstance>); 5] = [
        ("wrong world", COMBAT_SIM, vec![completed_prereq()]),
        ("prerequisite missing", CASTLE, Vec::new()),
        ("prerequisite not completed", CASTLE, vec![open_prereq()]),
        (
            "already active",
            CASTLE,
            vec![completed_prereq(), active_mission(&[])],
        ),
        (
            "already completed",
            CASTLE,
            vec![completed_prereq(), finished_mission()],
        ),
    ];
    for (label, world, missions) in cases {
        let mut mgr = staged(&pool, world, missions).await;
        let before = snapshot(&mgr);
        let (handled, out) = click_giver(&engine, &mut mgr).await;
        assert!(!handled, "{label}: no chain may handle the click");
        assert!(
            !any_mission_traffic(&out),
            "{label}: the click must send no 10312 traffic; got {out:?}"
        );
        assert_eq!(snapshot(&mgr), before, "{label}: the click moved state");
    }
}

// ── Forged dialog events ─────────────────────────────────────────────────

/// **A forged close is inert.** No chain keys on dialog 10717 or 10718 or on
/// a dialog event under 10312, so a forged `button -1` or a stale
/// `initialResponse` sends no 10312 traffic and leaves the state alone, from
/// every stage. Revert proof: add a `dialog_choice` chain on 10312 and the
/// seed count fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_forged_close_is_inert() {
    let pool = require_db_or_skip!();
    let keyed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE (t.event_type IN ('dialog_choice', 'dialog_open') AND t.event_key IN ($1, $2)) \
            OR (c.scope_type = 'mission' AND c.scope_id = $3 \
                AND t.event_type IN ('dialog_choice', 'dialog_open', 'dialog_set_open'))",
    )
    .bind(OFFER_DIALOG.to_string())
    .bind(DONE_DIALOG.to_string())
    .bind(MISSION)
    .fetch_one(&pool)
    .await
    .expect("dialog trigger query must succeed");
    assert_eq!(
        keyed, 0,
        "no chain may key on dialogs {OFFER_DIALOG} or {DONE_DIALOG} or on a dialog event under {MISSION}"
    );

    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for missions in [
        vec![completed_prereq()],
        vec![completed_prereq(), active_mission(&[])],
        vec![completed_prereq(), active_mission(&OBJECTIVES[..3])],
        vec![completed_prereq(), finished_mission()],
    ] {
        let mut mgr = staged(&pool, CASTLE, missions).await;
        log_in(&engine, &mut mgr).await;
        let before = snapshot(&mgr);
        let (tx, mut rx) = mpsc::channel(256);
        for button in [-1, 0, 1] {
            fire_dialog_choice(ENTITY, PLAYER, OFFER_DIALOG, button, &engine, &tx, &mut mgr).await;
        }
        fire_dialog_open(ENTITY, PLAYER, OFFER_DIALOG, &engine, &tx, &mut mgr).await;
        let out = drain(&mut rx);
        assert!(
            !any_mission_traffic(&out),
            "{before:?}: a forged dialog event must send no 10312 traffic; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "{before:?}: a forged dialog event moved state"
        );
    }
}

// ── Sibling disjointness ─────────────────────────────────────────────────

/// **For each event and every reachable 10310/10312 state, at most one chain
/// of 10120-10129 resolves.** Every state of the prerequisite (absent, open,
/// completed) crossed with every 10312 state (absent, 0 to 3 of 4 active,
/// completed) is built and each event resolved against it. Revert proof: add
/// the trigger `entity_dead_tag Castle_BravoOfficer1` to chain 10126; a fresh
/// kill of Officer 1 then resolves 10125 and 10126 together.
#[tokio::test]
async fn live_db_castle_side_defense_q3_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let prereqs = [None, Some(open_prereq()), Some(completed_prereq())];
    let quests = [
        None,
        Some(active_mission(&[])),
        Some(active_mission(&OBJECTIVES[..1])),
        Some(active_mission(&OBJECTIVES[..2])),
        Some(active_mission(&OBJECTIVES[..3])),
        Some(finished_mission()),
    ];
    for prereq in &prereqs {
        for quest in &quests {
            let missions: Vec<MissionInstance> =
                prereq.iter().chain(quest.iter()).cloned().collect();
            let mgr = staged(&pool, CASTLE, missions).await;
            for (trigger_type, key) in event_keys() {
                let label = format!("{trigger_type:?} with {prereq:?} and {quest:?}");
                let fired = resolve(&engine, &mgr, trigger_type, key);
                assert!(
                    fired.len() <= 1,
                    "{label} resolved {fired:?}; sibling chains must be pairwise disjoint"
                );
            }
        }
    }
}

// ── Two players and the shared actors ────────────────────────────────────

/// **One kill credits only the killer, and no chain here despawns anything.**
/// Two players hold an active 10312 at 0 of 4 and a kill by the first player
/// completes its objective only. The second player's count stays at 0 and it
/// stays active. The static scan confirms no chain of 10120-10129 holds a
/// `DespawnEntity` action, so the shared officers and the giver survive every
/// player's run. Revert proof: add a `DespawnEntity` action to 10125 and the
/// scan fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_two_player_credit() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for id in ALL_CHAINS {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        assert!(
            !chain
                .actions
                .iter()
                .any(|a| matches!(a, Action::DespawnEntity { .. })),
            "chain {id} must not despawn any entity"
        );
    }

    let mut mgr = staged(&pool, CASTLE, vec![completed_prereq(), active_mission(&[])]).await;
    mgr.create_entity(ENTITY2, CASTLE, [0.0; 3], [0.0; 3])
        .unwrap();
    let other = mgr.get_entity_mut(ENTITY2).unwrap();
    other.is_player = true;
    other.player_id = Some(PLAYER2);
    other.archetype_id = Some(1);
    other.missions.add_mission(active_mission(&[]));

    let out = kill(&engine, &mut mgr, KILL_TAGS[0]).await;

    assert!(
        any_mission_traffic(&out),
        "the killer's 10125 must send a 10312 update"
    );
    assert_eq!(
        mission_of(&mgr).unwrap().completed_objectives,
        vec![OBJECTIVES[0]],
        "the killer's objective must complete"
    );
    let second = mgr
        .get_entity(ENTITY2)
        .unwrap()
        .missions
        .get_mission(MISSION);
    let second = second.expect("the second player keeps its 10312");
    assert!(
        second.completed_objectives.is_empty(),
        "the other player's count must stay at 0; got {:?}",
        second.completed_objectives
    );
    assert_eq!(
        second.status, MISSION_ACTIVE,
        "the other player stays active"
    );
}
