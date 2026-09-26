//! NA42 (handoff §17/§18, §26 test 16): an escort survives a fight.
//!
//! `generate_threat` preempts a mob in Follow into Fighting and keeps its
//! `follow_target_id`. When the fight ends the leash resets a follower
//! where it stands. Before NA42 that reset always landed in Idle, which the
//! dispatcher never promotes back to Follow, so one stray hit ended an
//! escort for good (chain 1302 existed only so a player could click
//! Zuritska to re-arm hers). Now the reset goes back to Follow while the
//! leader is still in the space, and clears the target and idles when the
//! leader has gone.
//!
//! These run on the real `castle_cellblock.nav`, on the topside route the
//! GC1 escort walks, so "a routed follow leg" means a navmesh route rather
//! than the meshless straight-line fallback.

use std::path::Path;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat::{generate_threat, AggroCause};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const NPC: u32 = 200;
const PLAYER: u32 = 101;

/// Two u off the Ring 3 landing pad: where chain 1173 puts Col Marsh.
const RING3_PAD: [f32; 3] = [-91.689, 45.188, -161.533];
/// `MessHall_Guard1`'s spawn, on the same mesh component as the pad.
const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// A non-instanced `Castle_CellBlock` with the real mesh, a player on the
/// pad and a mob escort beside it, in Follow on the player. The mob's spawn
/// is far away (its Preparation-room spawn) so a snap home would show.
/// `castle_cellblock.nav` is tracked in git, so a missing file fails.
fn escort_on_the_pad() -> SpaceManager {
    let nav = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces/castle_cellblock.nav");
    let mesh = NavMesh::load(&nav).unwrap_or_else(|e| panic!("load {}: {e}", nav.display()));
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.space_id_for_world("Castle_CellBlock").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(mesh);

    let player_pos = [RING3_PAD[0] + 1.5, RING3_PAD[1], RING3_PAD[2]];
    mgr.create_entity(PLAYER, "Castle_CellBlock", player_pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.connect_entity(PLAYER);

    mgr.spawn_npc(NPC, "Castle_CellBlock", RING3_PAD, [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.spawn_position = Some(Vector3::new(-191.0, 54.72, -138.588));
        npc.follow_target_id = Some(PLAYER);
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Follow);
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// Hit the escort, then end the fight the way content or a lost target
/// does: an empty threat list. Two ticks: Fighting -> Leashing, then the
/// leash resets the follower where it stands.
async fn fight_and_leash(mgr: &mut SpaceManager) -> Vector3 {
    let _ = generate_threat(mgr, PLAYER, NPC, 25.0, AggroCause::Damage);
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Fighting, "precondition: preempted");
    assert_eq!(npc.follow_target_id, Some(PLAYER), "preemption keeps it");
    let fought_at = npc.position;

    mgr.get_entity_mut(NPC).unwrap().threat_list.clear();
    ai_tick(mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Leashing);
    ai_tick(mgr).await;
    fought_at
}

/// §26 test 16. Follow -> damage -> Fighting -> threat cleared -> leash
/// arrive -> Follow, target kept, no snap to spawn, a `follow_resumed`
/// transition row, and on the next tick a routed follow leg toward the
/// leader. Revert proof: with `leash::arrive` setting Idle unconditionally
/// the state assertion fails (it ends Idle) and no leg is ever planned.
#[tokio::test]
async fn an_escort_hit_mid_follow_resumes_follow_after_the_fight() {
    let mut mgr = escort_on_the_pad();
    let logs = LogCapture::install();
    let fought_at = fight_and_leash(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(
        npc.ai_state(),
        AiState::Follow,
        "the leash must hand a follower whose leader is here back to Follow"
    );
    assert_eq!(npc.follow_target_id, Some(PLAYER));
    assert_eq!(npc.position, fought_at, "a follower is reset in place");
    assert!(npc.threat_list.is_empty());
    let resumed: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| {
            c.target == "npc_ai.transition"
                && c.has_field("from", "leashing")
                && c.has_field("to", "follow")
                && c.has_field("reason", "follow_resumed")
        })
        .collect();
    assert_eq!(resumed.len(), 1, "{:#?}", logs.all());

    // The leader walks off down the escort route: the next tick routes the
    // escort after it on the mesh.
    mgr.get_entity_mut(PLAYER).unwrap().position = v(MESSHALL);
    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Follow);
    assert!(
        npc.nav_path.len() > 1,
        "the Follow handler must route a real navmesh leg toward the leader; \
         got {:?}",
        npc.nav_path
    );
    let end = *npc.nav_path.back().unwrap();
    let to_leader = ((end.x - MESSHALL[0]).powi(2) + (end.z - MESSHALL[2]).powi(2)).sqrt();
    assert!(
        to_leader < 5.0,
        "the leg must end near the leader (one follow_min_distance short), \
         got {end:?}, {to_leader} u off"
    );
}

/// Negative case: the leader despawned during the fight. The leash clears
/// the dangling target and goes Idle (the Follow handler's own lost-target
/// rule), and says so with a WARN.
#[tokio::test]
async fn an_escort_whose_leader_left_during_the_fight_clears_the_target_and_idles() {
    let mut mgr = escort_on_the_pad();
    let _ = generate_threat(&mut mgr, PLAYER, NPC, 25.0, AggroCause::Damage);
    mgr.get_entity_mut(NPC).unwrap().threat_list.clear();
    mgr.destroy_entity(PLAYER);
    assert!(mgr.get_entity(PLAYER).is_none());

    let logs = LogCapture::install();
    ai_tick(&mut mgr).await; // Fighting -> Leashing
    ai_tick(&mut mgr).await; // reset in place
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.follow_target_id, None, "a dangling target is cleared");
    let lost: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.leash" && c.has_field("event", "follow_target_lost"))
        .collect();
    assert_eq!(lost.len(), 1, "{:#?}", logs.all());
    assert_eq!(lost[0].level, tracing::Level::WARN);
}
