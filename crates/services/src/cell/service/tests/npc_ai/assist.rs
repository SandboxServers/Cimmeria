//! Same-room assist (NA14, D-NA04) on a meshless space: who is pulled in,
//! who is not, and that an assister never recruits further. The real
//! navmesh cases (the MessHall pair, the Hallway guards) are in
//! [`super::assist_castle`].
//!
//! Layout: the victim `A` at the origin, neighbours along +x, the player far
//! enough away (40 u) that nobody proximity-aggroes it on their own, so
//! every engagement past `A`'s is an assist.

use cimmeria_entity::cell_entity::{AiState, MobAggression};
use tokio::sync::mpsc;

use crate::cell::combat::{generate_threat, AggroCause, HOSTILE_FACTION};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{LogCapture, LogCaptureGuard};

const A: u32 = 200_101;
const B: u32 = 200_102;
const C: u32 = 200_103;
const PLAYER: u32 = 7;
const FAR: [f32; 3] = [-40.0, 0.0, 0.0];

fn fixture(player_pos: [f32; 3], neighbours: &[(u32, [f32; 3])]) -> SpaceManager {
    let mut mgr = super::make_aggression_fixture(A, HOSTILE_FACTION, PLAYER, player_pos);
    for &(id, pos) in neighbours {
        mgr.spawn_npc(id, "Castle", pos, [0.0; 3]).unwrap();
        let npc = mgr.get_entity_mut(id).unwrap();
        npc.faction = HOSTILE_FACTION;
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
    }
    mgr
}

/// The player shoots `A`.
fn shoot_a(mgr: &mut SpaceManager) {
    let _ = generate_threat(mgr, PLAYER, A, 10.0, AggroCause::Damage);
}

fn engaged(mgr: &SpaceManager, id: u32) -> bool {
    let npc = mgr.get_entity(id).unwrap();
    npc.ai_state() == AiState::Fighting && npc.threat_list.contains_key(&PLAYER)
}

fn untouched(mgr: &SpaceManager, id: u32) -> bool {
    mgr.get_entity(id).unwrap().threat_list.is_empty()
}

fn acquired_causes(logs: &LogCaptureGuard, npc: u32) -> Vec<String> {
    logs.all()
        .into_iter()
        .filter(|c| {
            c.target == "npc_ai.aggro"
                && c.has_field("event", "acquired")
                && c.has_field("npc_id", &npc.to_string())
        })
        .filter_map(|c| c.fields.get("cause").cloned())
        .collect()
}

fn assist_rejected(logs: &LogCaptureGuard, npc: u32, reason: &str) -> bool {
    logs.all().iter().any(|c| {
        c.target == "npc_ai.aggro_scan"
            && c.has_field("event", "assist_rejected")
            && c.has_field("npc_id", &npc.to_string())
            && c.has_field("reason", reason)
    })
}

async fn tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// A hostile same-faction neighbour 7 u away joins on the same target, and
/// says so: `acquired cause=assist`, transition `reason=assist`.
#[tokio::test]
async fn neighbour_within_the_assist_radius_joins_the_fight() {
    let mut mgr = fixture(FAR, &[(B, [7.0, 0.0, 0.0])]);
    let logs = LogCapture::install();
    shoot_a(&mut mgr);

    assert!(engaged(&mgr, A));
    assert!(engaged(&mgr, B), "B must assist A");
    assert_eq!(acquired_causes(&logs, B), vec!["assist".to_string()]);
    assert_eq!(acquired_causes(&logs, A), vec!["damage".to_string()]);
    let transition = logs
        .all()
        .into_iter()
        .find(|c| c.target == "npc_ai.transition" && c.has_field("npc_id", &B.to_string()))
        .expect("B's Fighting entry logs a transition");
    assert!(transition.has_field("reason", "assist"), "{transition:?}");
    assert!(logs.all().iter().any(|c| c.target == "npc_ai.aggro_scan"
        && c.has_field("event", "assist_joined")
        && c.has_field("victim_id", &A.to_string())));
    // The player's threat set names both mobs, so combat tracking covers the
    // assister too.
    let threatened = &mgr.get_entity(PLAYER).unwrap().threatened_mobs;
    assert!(
        threatened.contains(&A) && threatened.contains(&B),
        "{threatened:?}"
    );

    // One AI tick later B is still on the player: the seed is real threat.
    tick(&mut mgr).await;
    assert!(engaged(&mgr, B));
}

/// No chaining: `C` is 7 u from the assister `B` but 14 u from the victim
/// `A`. `B` joins; `C` does not, although it would if assist recruited.
#[tokio::test]
async fn an_assister_does_not_recruit_a_further_npc() {
    let mut mgr = fixture(FAR, &[(B, [7.0, 0.0, 0.0]), (C, [14.0, 0.0, 0.0])]);
    let logs = LogCapture::install();
    shoot_a(&mut mgr);

    assert!(engaged(&mgr, B));
    assert!(untouched(&mgr, C), "C must not be recruited through B");
    assert_eq!(mgr.get_entity(C).unwrap().ai_state(), AiState::Idle);
    assert!(assist_rejected(&logs, C, "out_of_radius"));

    tick(&mut mgr).await;
    assert!(untouched(&mgr, C), "nor on the next tick");
}

/// A chain-armed spawn seeded NEUTRAL (spawns 10 and 20) is never pulled:
/// its chain starts its fight.
#[tokio::test]
async fn a_neutral_chain_armed_spawn_is_never_pulled() {
    let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
    mgr.get_entity_mut(B).unwrap().aggro.override_level = Some(MobAggression::Neutral);
    let logs = LogCapture::install();
    shoot_a(&mut mgr);

    assert!(engaged(&mgr, A));
    assert!(untouched(&mgr, B));
    assert_eq!(mgr.get_entity(B).unwrap().ai_state(), AiState::Idle);
    assert!(assist_rejected(&logs, B, "not_hostile"));
}

/// A Leashing NPC (walking home) is never pulled, and neither is one that is
/// already in another fight or investigating.
#[tokio::test]
async fn a_busy_neighbour_is_never_pulled() {
    for state in [
        AiState::Leashing,
        AiState::Investigating,
        AiState::Follow,
        AiState::Dead,
    ] {
        let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
        crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(B).unwrap(), state);
        let logs = LogCapture::install();
        shoot_a(&mut mgr);

        assert!(untouched(&mgr, B), "{state:?}");
        assert_eq!(mgr.get_entity(B).unwrap().ai_state(), state);
        let reason = if state == AiState::Dead {
            "dead"
        } else {
            "not_idle"
        };
        assert!(assist_rejected(&logs, B, reason), "{state:?}");
    }
}

/// Patrolling and wandering neighbours drop their route and join.
#[tokio::test]
async fn patrolling_and_wandering_neighbours_join() {
    for state in [AiState::Patrol, AiState::Wander] {
        let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
        crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(B).unwrap(), state);
        shoot_a(&mut mgr);
        assert!(engaged(&mgr, B), "{state:?}");
    }
}

/// Only the victim's own faction assists, and only on its floor and inside
/// the assister's own template radius.
#[tokio::test]
async fn faction_band_and_template_radius_gate_the_assist() {
    // Another faction, HOSTILE through its override: not considered at all.
    let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
    {
        let b = mgr.get_entity_mut(B).unwrap();
        b.faction = 1;
        b.aggro.override_level = Some(MobAggression::Hostile);
    }
    let logs = LogCapture::install();
    shoot_a(&mut mgr);
    assert!(untouched(&mgr, B));
    assert!(!logs
        .all()
        .iter()
        .any(|c| c.has_field("event", "assist_rejected")));
    drop(logs);

    // 6 u up: another storey.
    let mut mgr = fixture(FAR, &[(B, [3.0, 6.0, 0.0])]);
    let logs = LogCapture::install();
    shoot_a(&mut mgr);
    assert!(untouched(&mgr, B));
    assert!(assist_rejected(&logs, B, "out_of_vertical_band"));
    drop(logs);

    // 7 u away, but B's template radius is 5.
    let mut mgr = fixture(FAR, &[(B, [7.0, 0.0, 0.0])]);
    mgr.get_entity_mut(B).unwrap().aggro.assist_radius_override = Some(5.0);
    let logs = LogCapture::install();
    shoot_a(&mut mgr);
    assert!(untouched(&mgr, B));
    assert!(assist_rejected(&logs, B, "out_of_radius"));
}

/// A content chain's `generate_threat` keeps a scripted fight scripted: it
/// does not recruit.
#[tokio::test]
async fn content_threat_does_not_recruit() {
    let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
    let _ = generate_threat(&mut mgr, PLAYER, A, 1000.0, AggroCause::ContentThreat);
    assert!(engaged(&mgr, A));
    assert!(untouched(&mgr, B));
}

/// Proximity aggro recruits too. The player is 15 u from `A` (inside its
/// 18 u aggro radius) and 23 u from `B` (outside it), so only an assist can
/// put `B` in the fight on this tick.
#[tokio::test]
async fn proximity_aggro_recruits_the_neighbour() {
    let mut mgr = fixture([-15.0, 0.0, 0.0], &[(B, [8.0, 0.0, 0.0])]);
    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert!(engaged(&mgr, A));
    assert_eq!(acquired_causes(&logs, A), vec!["proximity".to_string()]);
    assert!(engaged(&mgr, B), "B must assist A's proximity aggro");
    assert_eq!(acquired_causes(&logs, B), vec!["assist".to_string()]);
}

/// A GM with `.aggro off` who shoots a mob fights that mob only.
#[tokio::test]
async fn a_gm_with_aggro_off_pulls_no_assisters() {
    let mut mgr = fixture(FAR, &[(B, [5.0, 0.0, 0.0])]);
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.access_level = 2; // GameMaster
    }
    mgr.gm_aggro_off.insert(PLAYER as i32);

    let logs = LogCapture::install();
    shoot_a(&mut mgr);
    assert!(engaged(&mgr, A));
    assert!(untouched(&mgr, B));
    assert!(assist_rejected(&logs, B, "gm_ignored"));
}
