//! Mission 1521 (Castle symbiote tank, Option B) step graph, CS-03c2:
//! tank clicks 1374/1376, the Livewire victory 1375, the dialog close 1377
//! and the relog path, all against the seeded rows of
//! `castle_symbiote_chains.sql`.
//!
//! The victory chain 1375 has no trigger and carries conditions, so it is
//! driven through `fire_guarded_minigame_chain_by_id`, the seam that
//! evaluates them. The routing in `base_messages/minigame.rs` that picks
//! the guarded seam for a conditioned chain is CS-MGF2's; until it lands the
//! live victory path still calls the unguarded `fire_chain_by_id`, which
//! ignores the 1375 conditions. This file therefore proves the chain and the
//! guarded seam, not the end-to-end win, and says so in each victory test.

use cimmeria_content_engine::chain::ChainEngine;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::{
    fire_dialog_choice, fire_enter_region, fire_guarded_minigame_chain_by_id, fire_interact_tag,
    fire_player_loaded,
};
use super::mission_1521_entry::{
    completed_704, engine_with, mission_at, mission_of, staged, CASTLE, ENTITY, MISSION, PLAYER,
    STEP_FIRST, STEP_TANK, TANK_ENTITY, TANK_TAG,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::load_monologue_dialog_ids;
use crate::test_support::require_db_or_skip;

const STEP_VICTORY: i32 = 4457;
const STEP_CLOSE: i32 = 4458;
const ALL_CHAINS: [i32; 6] = [1369, 1370, 1374, 1375, 1376, 1377];
/// Wire index of `onDialogDisplay` (as `debug_hub_mail_clerk.rs`).
const ON_DIALOG_DISPLAY: u16 = 105;

/// What one dispatch produced: the Livewire starts and the dialog ids shown.
#[derive(Debug, Default, PartialEq, Eq)]
struct Sent {
    minigames: usize,
    dialogs: Vec<i32>,
    other: usize,
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Sent {
    let mut sent = Sent::default();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::StartMinigame { .. } => sent.minigames += 1,
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if method_index == ON_DIALOG_DISPLAY => {
                sent.dialogs
                    .push(i32::from_le_bytes(args[4..8].try_into().unwrap()));
            }
            _ => sent.other += 1,
        }
    }
    sent
}

/// A Jaffa in `world` with 1521 at `step`, holding that step's objective.
async fn at_step(pool: &PgPool, world: &str, step: i32) -> SpaceManager {
    let objective = match step {
        STEP_FIRST => 5175,
        STEP_TANK => 5176,
        STEP_VICTORY => 5177,
        _ => 5178,
    };
    let mut mgr = staged(
        pool,
        world,
        Some(8),
        vec![
            completed_704(),
            mission_at(MISSION, Some(step), &[objective]),
        ],
    )
    .await;
    // 4990 is a player monologue; the executor needs the set to bind the
    // player rather than hunt for an NPC.
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool).await.expect("monologues");
    mgr
}

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr, MISSION).and_then(|m| m.current_step_id)
}

fn credited(mgr: &SpaceManager, objective: i32) -> bool {
    mission_of(mgr, MISSION).is_some_and(|m| m.completed_objectives.contains(&objective))
}

async fn click(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Sent) {
    let (tx, mut rx) = mpsc::channel(64);
    let handled = fire_interact_tag(ENTITY, PLAYER, TANK_TAG, TANK_ENTITY, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

async fn victory(engine: &ChainEngine, mgr: &mut SpaceManager) -> Sent {
    let (tx, mut rx) = mpsc::channel(64);
    fire_guarded_minigame_chain_by_id(1375, ENTITY, engine, &tx, mgr).await;
    drain(&mut rx)
}

async fn close_4991(engine: &ChainEngine, mgr: &mut SpaceManager) -> Sent {
    let (tx, mut rx) = mpsc::channel(64);
    fire_dialog_choice(ENTITY, PLAYER, 4991, -1, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// **1374 and 1376 share one trigger key and are pairwise disjoint.** With
/// both registered, a click at 4456 fires only the Livewire starter and a
/// click at 4457 only the advance; 4455 and 4458 fire neither.
#[tokio::test]
async fn live_db_1521_tank_chains_are_disjoint_on_the_shared_trigger() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1374, 1376]).await;

    let mut mgr = at_step(&pool, CASTLE, STEP_TANK).await;
    let (handled, sent) = click(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!((sent.minigames, sent.dialogs.len()), (1, 0), "{sent:?}");
    assert_eq!(step_of(&mgr), Some(STEP_TANK), "the starter never advances");

    let mut mgr = at_step(&pool, CASTLE, STEP_VICTORY).await;
    let (handled, sent) = click(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!(sent.minigames, 0, "1374 must not fire at 4457: {sent:?}");
    assert_eq!(sent.dialogs, vec![4991]);

    for step in [STEP_FIRST, STEP_CLOSE] {
        let mut mgr = at_step(&pool, CASTLE, step).await;
        let (handled, sent) = click(&engine, &mut mgr).await;
        assert!(!handled, "no tank chain may fire at step {step}");
        assert_eq!(sent, Sent::default());
        assert_eq!(step_of(&mgr), Some(step));
    }
}

/// **1375 at 4456 credits 5176, moves to 4457 and shows monologue 4990.**
/// Guarded seam only; see the module note on CS-MGF2.
#[tokio::test]
async fn live_db_1521_victory_credits_5176_advances_to_4457_and_shows_4990() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1375]).await;
    let mut mgr = at_step(&pool, CASTLE, STEP_TANK).await;
    let sent = victory(&engine, &mut mgr).await;
    assert_eq!(step_of(&mgr), Some(STEP_VICTORY));
    assert!(credited(&mgr, 5176), "advance_step must credit 5176");
    assert_eq!(sent.dialogs, vec![4990], "{sent:?}");
}

/// **1375 emits nothing at any other step, in another world, once the
/// mission is completed, or for a character without it.** These are the
/// behavioural half of the CS-03c1 seed guard.
#[tokio::test]
async fn live_db_1521_victory_at_a_stale_step_emits_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1375]).await;

    for step in [STEP_FIRST, STEP_VICTORY, STEP_CLOSE] {
        let mut mgr = at_step(&pool, CASTLE, step).await;
        let sent = victory(&engine, &mut mgr).await;
        assert_eq!(sent, Sent::default(), "victory at step {step}");
        assert_eq!(step_of(&mgr), Some(step), "step {step} must not move");
    }

    let mut other_world = at_step(&pool, "Castle_CellBlock", STEP_TANK).await;
    assert_eq!(
        victory(&engine, &mut other_world).await,
        Sent::default(),
        "wrong world"
    );
    assert_eq!(step_of(&other_world), Some(STEP_TANK));

    let mut done = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), {
            let mut m = mission_at(MISSION, None, &[]);
            m.status = cimmeria_entity::missions::MISSION_COMPLETED;
            m
        }],
    )
    .await;
    assert_eq!(victory(&engine, &mut done).await, Sent::default());

    let mut never = staged(&pool, CASTLE, Some(8), vec![completed_704()]).await;
    assert_eq!(victory(&engine, &mut never).await, Sent::default());
    assert!(mission_of(&never, MISSION).is_none());
}

/// A replayed win after the advance is a no-op: the first call moves the
/// step, the second finds 4457 and stops.
#[tokio::test]
async fn live_db_1521_victory_replayed_twice_advances_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1375]).await;
    let mut mgr = at_step(&pool, CASTLE, STEP_TANK).await;
    assert_eq!(victory(&engine, &mut mgr).await.dialogs, vec![4990]);
    assert_eq!(victory(&engine, &mut mgr).await, Sent::default());
    assert_eq!(step_of(&mgr), Some(STEP_VICTORY));
}

/// **1376 at 4457 credits 5177, moves to 4458 and opens 4991.**
#[tokio::test]
async fn live_db_1521_second_click_credits_5177_advances_to_4458_and_opens_4991() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1376]).await;
    let mut mgr = at_step(&pool, CASTLE, STEP_VICTORY).await;
    let (handled, sent) = click(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!(step_of(&mgr), Some(STEP_CLOSE));
    assert!(credited(&mgr, 5177), "advance_step must credit 5177");
    assert_eq!(sent.dialogs, vec![4991], "{sent:?}");
}

/// **1377 on the 4991 close credits 5178 and completes 1521; the second
/// event does nothing.** Also quiet at 4457, where 4991 is not yet open.
#[tokio::test]
async fn live_db_1521_dialog_close_completes_the_mission_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1377]).await;

    let mut early = at_step(&pool, CASTLE, STEP_VICTORY).await;
    close_4991(&engine, &mut early).await;
    assert_eq!(step_of(&early), Some(STEP_VICTORY), "4991 close at 4457");
    assert_eq!(
        mission_of(&early, MISSION).map(|m| m.status),
        Some(cimmeria_entity::missions::MISSION_ACTIVE)
    );

    let mut mgr = at_step(&pool, CASTLE, STEP_CLOSE).await;
    close_4991(&engine, &mut mgr).await;
    let mission = mission_of(&mgr, MISSION).expect("mission stays on the record");
    assert_eq!(mission.status, cimmeria_entity::missions::MISSION_COMPLETED);
    assert!(
        mission.completed_objectives.contains(&5178),
        "complete_mission must credit 5178; got {:?}",
        mission.completed_objectives
    );
    let credits_before = mission.completed_objectives.clone();

    let again = close_4991(&engine, &mut mgr).await;
    assert_eq!(again, Sent::default(), "re-fire must emit nothing");
    assert_eq!(
        mission_of(&mgr, MISSION).unwrap().completed_objectives,
        credits_before
    );
}

/// **Offer to completion through the real dispatchers, crediting every
/// objective.** CS-03S requires the end state to hold 5175-5178: 5175 from the
/// 1370 advance, 5176 and 5177 from the 1375/1376 advances, and 5178 only from
/// the 1377 `complete_mission`. The single-step tests stage one objective, so
/// only this walk can see a credit that a step transition drops.
#[tokio::test]
async fn live_db_1521_full_walk_credits_every_objective_and_completes() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, CASTLE, Some(8), vec![completed_704()]).await;
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(&pool).await.expect("monologues");

    let (tx, _rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, "Castle.Infirmary", &engine, &tx, &mut mgr).await;
    assert_eq!(step_of(&mgr), Some(STEP_TANK), "offer lands on 4456");
    assert!(credited(&mgr, 5175), "1370's advance credits 5175");

    let (handled, sent) = click(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!(sent.minigames, 1, "first click starts Livewire: {sent:?}");
    assert_eq!(step_of(&mgr), Some(STEP_TANK));

    let sent = victory(&engine, &mut mgr).await;
    assert_eq!(sent.dialogs, vec![4990], "{sent:?}");
    assert_eq!(step_of(&mgr), Some(STEP_VICTORY));

    let (handled, sent) = click(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!(sent.dialogs, vec![4991], "{sent:?}");
    assert_eq!(step_of(&mgr), Some(STEP_CLOSE));

    close_4991(&engine, &mut mgr).await;
    let mission = mission_of(&mgr, MISSION).expect("mission stays on the record");
    assert_eq!(mission.status, cimmeria_entity::missions::MISSION_COMPLETED);
    assert_eq!(mission.current_step_id, None, "no step left active");
    for objective in [5175, 5176, 5177, 5178] {
        assert!(
            mission.completed_objectives.contains(&objective),
            "objective {objective} must be credited; got {:?}",
            mission.completed_objectives
        );
    }
}

/// **Relog at 4456, 4457 and 4458.** `player_loaded` runs every 1521 chain
/// and must change nothing (no restore chain exists); the click path then
/// resumes. At 4458 there is no click chain, so a relog between the second
/// click and the 4991 close leaves the mission with no way forward: the
/// tank does nothing and 4991 is not re-offered. This pins today's behavior
/// and is reported as a gap; fixing it needs a restore chain (a new seed
/// row), which is not CS-03c2's call.
#[tokio::test]
async fn live_db_1521_relog_resumes_the_click_path() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for step in [STEP_TANK, STEP_VICTORY, STEP_CLOSE] {
        let mut mgr = at_step(&pool, CASTLE, step).await;
        let (tx, mut rx) = mpsc::channel(64);
        fire_player_loaded(ENTITY, PLAYER, CASTLE, &engine, &tx, &mut mgr).await;
        assert_eq!(drain(&mut rx), Sent::default(), "relog at {step}");
        assert_eq!(step_of(&mgr), Some(step), "relog must not move {step}");

        let (handled, sent) = click(&engine, &mut mgr).await;
        match step {
            STEP_TANK => assert_eq!((handled, sent.minigames), (true, 1)),
            STEP_VICTORY => {
                assert_eq!((handled, sent.dialogs.clone()), (true, vec![4991]));
                assert_eq!(step_of(&mgr), Some(STEP_CLOSE));
            }
            _ => {
                assert!(!handled, "KNOWN GAP: no click chain at 4458");
                assert_eq!(sent, Sent::default());
            }
        }
    }
}
