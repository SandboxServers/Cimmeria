//! NA13 proximity aggro over the real `castle_cellblock.nav`: a NID Guard
//! (template 24, faction 10, no override) against one player placed in LoS
//! at 15 u, in LoS at 25 u, behind a wall, and on the storey below.
//!
//! The probe points were picked from the mesh itself (all `is_point_valid`,
//! LoS as stated). The storey case is the one the navmesh ray gets wrong:
//! it reports `Clear` down to the floor 15 u below (audit S15), so only the
//! vertical band keeps the guard from pulling a player on another storey.
//!
//! The mesh is injected into the space, as in [`super::leash_walk`]. Skips
//! on a checkout without the fixture.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::{LineOfSight, NavMesh};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const NPC: u32 = 200;
const PLAYER: u32 = 101;
const WORLD: &str = "Castle_CellBlock";
/// `MessHall_Guard1`'s spawn (`spawnlist.sql` row 29).
const MESSHALL_GUARD: [f32; 3] = [-96.25, 34.591, -91.59];
/// `Hallway01_Guard`'s spawn (row 30), above the Barracks level.
const HALLWAY01_GUARD: [f32; 3] = [-128.853, 39.552, -73.534];

/// Same floor, 15.5 u from the MessHall guard, clear line of sight.
const IN_LOS_15: [f32; 3] = [-111.25, 34.91, -95.59];
/// Same floor, 25.7 u away, clear line of sight.
const IN_LOS_25: [f32; 3] = [-90.25, 34.60, -116.59];
/// Same floor, 10.3 u away, behind a wall (the mesh ray is `Blocked`).
const BEHIND_WALL: [f32; 3] = [-105.25, 34.60, -86.59];
/// The floor below Hallway01: 7.2 u horizontally, 14.8 u down, and the mesh
/// ray reads `Clear`.
const STOREY_BELOW: [f32; 3] = [-134.85, 24.80, -69.53];

struct Fixture {
    mgr: SpaceManager,
    nav_los: LineOfSight,
}

fn fixture(guard: [f32; 3], player: [f32; 3]) -> Option<Fixture> {
    let nav = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav).expect("load castle_cellblock.nav");
    let g = navmesh.get_nearest_point(&Vector3::new(guard[0], guard[1], guard[2]));
    let p = Vector3::new(player[0], player[1], player[2]);
    assert!(
        navmesh.is_point_valid(&p),
        "fixture point {p:?} is on the mesh"
    );
    let nav_los = navmesh.line_of_sight(&g, &p);

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr
        .spawn_npc(NPC, WORLD, [g.x, g.y, g.z], [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.template_id = Some(24);
        npc.faction = HOSTILE_FACTION;
        npc.spawn_position = Some(g);
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
    }
    mgr.create_entity(PLAYER, WORLD, player, [0.0; 3]).unwrap();
    if let Some(pl) = mgr.get_entity_mut(PLAYER) {
        pl.is_player = true;
        pl.player_id = Some(PLAYER as i32);
        pl.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(NPC).contains(&PLAYER),
        "fixture: the player is an AoI witness, so only the NA13 gates decide"
    );
    Some(Fixture { mgr, nav_los })
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

fn rejected_with(logs: &crate::test_support::LogCaptureGuard, reason: &str) -> bool {
    logs.all().iter().any(|c| {
        c.target == "npc_ai.aggro_scan"
            && c.has_field("event", "candidate_rejected")
            && c.has_field("reason", reason)
    })
}

/// A NID Guard aggroes a player 15 u away in line of sight.
#[tokio::test]
async fn nid_guard_aggroes_a_player_at_15_u_in_los() {
    let Some(mut f) = fixture(MESSHALL_GUARD, IN_LOS_15) else {
        return;
    };
    assert_eq!(f.nav_los, LineOfSight::Clear);
    tick(&mut f.mgr).await;
    let npc = f.mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Fighting);
    assert!(npc.threat_list.contains_key(&PLAYER));
}

/// Not at 25 u, although the line of sight is clear.
#[tokio::test]
async fn nid_guard_ignores_a_player_at_25_u() {
    let Some(mut f) = fixture(MESSHALL_GUARD, IN_LOS_25) else {
        return;
    };
    assert_eq!(f.nav_los, LineOfSight::Clear);
    let logs = LogCapture::install();
    tick(&mut f.mgr).await;
    assert_eq!(f.mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
    assert!(rejected_with(&logs, "out_of_radius"));
}

/// Not through a wall, although the player is inside the radius.
#[tokio::test]
async fn nid_guard_ignores_a_player_behind_a_wall() {
    let Some(mut f) = fixture(MESSHALL_GUARD, BEHIND_WALL) else {
        return;
    };
    assert_eq!(f.nav_los, LineOfSight::Blocked);
    let logs = LogCapture::install();
    tick(&mut f.mgr).await;
    assert_eq!(f.mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
    assert!(rejected_with(&logs, "no_los"));
}

/// Not on another storey, although the mesh ray says `Clear` and the player
/// is 7 u away horizontally: the vertical band is the storey guard.
#[tokio::test]
async fn nid_guard_ignores_a_player_on_another_storey() {
    let Some(mut f) = fixture(HALLWAY01_GUARD, STOREY_BELOW) else {
        return;
    };
    assert_eq!(
        f.nav_los,
        LineOfSight::Clear,
        "the navmesh ray cannot see floors (S15); the band must catch this"
    );
    let logs = LogCapture::install();
    tick(&mut f.mgr).await;
    assert_eq!(f.mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
    assert!(rejected_with(&logs, "out_of_vertical_band"));
}

/// D-NA08: an off-mesh endpoint (`Unknown`) fails closed for aggro. The
/// player stands 10 u above the MessHall floor, off the mesh but inside the
/// vertical band because the NPC is lifted too.
#[tokio::test]
async fn unknown_line_of_sight_fails_closed_for_aggro() {
    let Some(mut f) = fixture(MESSHALL_GUARD, IN_LOS_15) else {
        return;
    };
    // Lift both ends 10 u off the floor: same band, no mesh under either.
    for id in [NPC, PLAYER] {
        let e = f.mgr.get_entity_mut(id).unwrap();
        e.position.y += 10.0;
    }
    assert_eq!(f.mgr.line_of_sight(NPC, PLAYER), LineOfSight::Unknown);
    let logs = LogCapture::install();
    tick(&mut f.mgr).await;
    assert_eq!(f.mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
    assert!(rejected_with(&logs, "no_los"));
}
