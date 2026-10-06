//! M1569 "Ordinance" class rewards and signatures: chains 3041-3044, one
//! per Human class, on `mission_completed 1569`.
//!
//! Nothing accepts or completes M1569 yet: its route belongs to the SGU
//! route campaign (OD-CS10). These tests therefore fire the completion the
//! way the executor's `complete_mission` arm does, which is the seam that
//! campaign will reach the chains through.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::{AbilityGrantKind, Action};
use cimmeria_content_engine::chain::ResolvedActions;

use super::super::super::event_dispatch::fire_mission_completed;
use super::super::super::executor::execute_actions;
use super::{
    drain, engine_with, give_mission, label, mission_status, sgc_mgr, Sent, ARCHAEOLOGIST,
    COMMANDO, COMPLETED, NON_HUMANS, PLAYER_EID, PLAYER_ID, SCIENTIST, SOLDIER,
};
use crate::test_support::require_db_or_skip;

const REWARD_CHAINS: [i32; 4] = [3041, 3042, 3043, 3044];

/// The Gear matrix's M1569 rows and the Abilities matrix's signatures
/// (`docs/analysis/class-start-v6/README.md`): archetype, items in grant
/// order, signature ability.
const REWARDS: [(i32, &[i32], i32); 4] = [
    (SOLDIER, &[3260, 7373], 598),
    (COMMANDO, &[3347, 3359, 3372, 3387, 3401, 3325], 646),
    (SCIENTIST, &[4444, 7373], 948),
    (ARCHAEOLOGIST, &[6843, 7373], 802),
];

/// What the base must be asked for: each item once, in the item's own
/// default container (the bare test space maps none, so that is `INV_Main`,
/// 1), then the signature with its provenance and its own archetype gate.
fn expected(archetype: i32, items: &[i32], signature: i32) -> Vec<Sent> {
    items
        .iter()
        .map(|&item| Sent::Item(item, 1, 1))
        .chain([Sent::Abilities(
            vec![signature],
            AbilityGrantKind::Signature,
            Some(1569),
            vec![archetype],
        )])
        .collect()
}

/// **Guard: each Human class gets its own M1569 gear and signature, and
/// only its own.** All four chains are registered, so a missing or wrong
/// `archetype eq N` shows up as another class's items in the list. The
/// signature must carry `source_kind` signature, `source_id` 1569 and the
/// class's archetype (OD-CS06; the loader refuses a signature without
/// `archetypes`).
#[tokio::test]
async fn live_db_m1569_completion_gives_each_human_class_its_gear_and_signature() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &REWARD_CHAINS).await;

    for (archetype, items, signature) in REWARDS {
        let mut mgr = sgc_mgr(Some(archetype));
        let (tx, mut rx) = mpsc::channel(64);

        fire_mission_completed(PLAYER_EID, PLAYER_ID, 1569, &engine, &tx, &mut mgr).await;

        assert_eq!(
            drain(&mut rx),
            expected(archetype, items, signature),
            "M1569 rewards for archetype {archetype}",
        );
    }
}

/// The Asgard (holding state, blocked B1-B3), the Free Jaffa (geared at the
/// Dakara start), the Praxis archetypes and a player with no archetype get
/// nothing from M1569: `eq` never matches the -1 a missing archetype reads.
#[tokio::test]
async fn live_db_m1569_completion_gives_non_humans_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &REWARD_CHAINS).await;

    for archetype in NON_HUMANS.iter().map(|&a| Some(a)).chain([None]) {
        let mut mgr = sgc_mgr(archetype);
        let (tx, mut rx) = mpsc::channel(64);

        fire_mission_completed(PLAYER_EID, PLAYER_ID, 1569, &engine, &tx, &mut mgr).await;

        assert_eq!(
            drain(&mut rx),
            vec![],
            "M1569 rewards for {}",
            label(archetype)
        );
    }
}

/// The chains listen for 1569 and nothing else: a Soldier completing M1559
/// gets no Ordinance reward.
#[tokio::test]
async fn live_db_m1569_rewards_do_not_fire_on_another_mission() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &REWARD_CHAINS).await;
    let mut mgr = sgc_mgr(Some(SOLDIER));
    let (tx, mut rx) = mpsc::channel(64);

    fire_mission_completed(PLAYER_EID, PLAYER_ID, 1559, &engine, &tx, &mut mgr).await;

    assert_eq!(drain(&mut rx), vec![]);
}

/// **Guard: the route campaign's seam.** Whatever chain later completes
/// M1569 will do it with a `complete_mission 1569` action. Run that action
/// for a Soldier who holds the mission: the rewards follow from the
/// completion with no other wiring, and a second `complete_mission` (the
/// mission is no longer active) grants nothing more.
#[tokio::test]
async fn live_db_complete_mission_1569_delivers_the_rewards_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &REWARD_CHAINS).await;
    let mut mgr = sgc_mgr(Some(SOLDIER));
    give_mission(&mut mgr, 1569, 4640);
    let (tx, mut rx) = mpsc::channel(64);
    // A stand-in for the route campaign's chain; the id names no seeded chain.
    let complete = || {
        let mut resolved = ResolvedActions::default();
        resolved
            .actions
            .push((999_001, Action::CompleteMission { mission_id: 1569 }));
        resolved.action_delays.push(0);
        resolved
    };

    execute_actions(complete(), PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;
    assert_eq!(mission_status(&mgr, 1569), COMPLETED);
    assert_eq!(drain(&mut rx), expected(SOLDIER, &[3260, 7373], 598));

    execute_actions(complete(), PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;
    assert_eq!(
        drain(&mut rx),
        vec![],
        "completing an already completed M1569 grants nothing",
    );
}
