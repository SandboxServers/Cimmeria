//! `set_aggression` auto-aggro tests — Idle NPCs with `aggression > 0`
//! transitioning to Fighting against opposing-faction witnesses.

use super::make_aggression_fixture;
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

/// `aggression > 0` on an Idle NPC with an opposing-faction player in AoI
/// transitions the NPC to Fighting on the next tick with the player on
/// the threat list. Bug shape: the previous `set_aggression` wrote a
/// property nothing read, so the drone sat idle forever.
#[tokio::test]
async fn idle_npc_with_aggression_aggros_opposing_player() {
    let mut mgr = make_aggression_fixture(200_001, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_001) {
        npc.aggression = 1;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200_001).unwrap();
    assert_eq!(
        npc.ai_state(),
        AiState::Fighting,
        "aggression=1 with opposing-faction witness must transition to Fighting",
    );
    assert!(
        npc.threat_list.contains_key(&1),
        "player must be on NPC threat list after auto-aggro",
    );
}

/// `aggression == 0` (default) keeps the NPC Idle even with a hostile
/// player in AoI. The outer-tick filter — not an inner defensive check —
/// is what enforces this baseline; the test goes through `npc_ai_tick`
/// to exercise the actual production path.
#[tokio::test]
async fn idle_npc_without_aggression_stays_idle() {
    let mut mgr = make_aggression_fixture(200_002, 10, 1, [5.0, 0.0, 0.0]);
    assert_eq!(mgr.get_entity(200_002).unwrap().aggression, 0);
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200_002).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert!(npc.threat_list.is_empty());
}

/// `aggression > 0` but the witness shares the NPC's faction → no aggro.
/// Pins that the faction-equality check is in the right direction (skip
/// same-faction, target opposing).
#[tokio::test]
async fn aggression_skips_same_faction_witnesses() {
    let mut mgr = make_aggression_fixture(200_003, 0, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_003) {
        npc.aggression = 1;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let npc = mgr.get_entity(200_003).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle, "same faction must not aggro");
    assert!(npc.threat_list.is_empty());
}

/// NA00 review: the Idle scan is the one real caller of
/// `AggroCause::Proximity`. Driven through the dispatcher, its entry into
/// Fighting must say `cause=proximity` on the aggro row and
/// `reason=auto_aggro` on the transition row.
#[tokio::test]
async fn idle_auto_aggro_logs_the_proximity_cause() {
    let mut mgr = make_aggression_fixture(200_001, 10, 1, [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(200_001).unwrap().aggression = 1;
    let (tx, _rx) = mpsc::channel(16);
    let logs = crate::test_support::LogCapture::install();

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let all = logs.all();
    let acquired = all
        .iter()
        .find(|c| c.target == "npc_ai.aggro")
        .expect("auto-aggro must log npc_ai.aggro");
    assert!(acquired.has_field("cause", "proximity"), "{acquired:?}");
    let transition = all
        .iter()
        .find(|c| c.target == "npc_ai.transition" && c.has_field("to", "fighting"))
        .expect("and an Idle -> Fighting transition");
    assert!(
        transition.has_field("reason", "auto_aggro"),
        "{transition:?}"
    );
}
