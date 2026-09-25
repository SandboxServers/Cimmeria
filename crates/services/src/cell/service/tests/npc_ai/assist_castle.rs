//! NA14 same-room assist over the real `castle_cellblock.nav`: the two
//! MessHall guards (7.2 u apart, same room) and two Hallway guards (18.6 u
//! apart), all NID Guards (template 24, faction 10, no override), at their
//! seeded `spawnlist.sql` positions snapped to the mesh.
//!
//! The player stands where neither guard of a pair would aggro it on its
//! own, so the only way the second guard enters Fighting is an assist.
//! Skips on a checkout without the fixture.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::{LineOfSight, NavMesh};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat::{generate_threat, AggroCause, HOSTILE_FACTION};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{LogCapture, LogCaptureGuard};

const SHOT: u32 = 200;
const NEIGHBOUR: u32 = 201;
const PLAYER: u32 = 101;
const WORLD: &str = "Castle_CellBlock";

/// `MessHall_Guard1` (`spawnlist.sql` spawn 29).
const MESSHALL_GUARD1: [f32; 3] = [-96.25, 34.591, -91.59];
/// `MessHall_Guard2` (spawn 28), 7.2 u from Guard1.
const MESSHALL_GUARD2: [f32; 3] = [-95.89, 34.591, -98.808];
/// `Hallway01_Guard` (spawn 30).
const HALLWAY01_GUARD: [f32; 3] = [-128.853, 39.552, -73.534];
/// `Hallway02_Guard` (spawn 82), 18.6 u from Hallway01.
const HALLWAY02_GUARD: [f32; 3] = [-113.485, 39.552, -63.042];

/// MessHall floor, on the mesh: 25.7 u from Guard1 and 18.7 u from Guard2,
/// so outside both 18 u aggro radii. The shooter.
const MESSHALL_SHOOTER: [f32; 3] = [-90.25, 34.60, -116.59];

struct Fixture {
    mgr: SpaceManager,
    /// The mesh's own answer between the two guards.
    guard_los: LineOfSight,
}

fn fixture(shot: [f32; 3], neighbour: [f32; 3], player: [f32; 3]) -> Option<Fixture> {
    let nav = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav).expect("load castle_cellblock.nav");
    let snap = |p: [f32; 3]| navmesh.get_nearest_point(&Vector3::new(p[0], p[1], p[2]));
    let (s, n) = (snap(shot), snap(neighbour));
    let p = Vector3::new(player[0], player[1], player[2]);
    assert!(navmesh.is_point_valid(&p), "fixture: {p:?} is on the mesh");
    let guard_los = navmesh.line_of_sight(&n, &s);

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
        .spawn_npc(SHOT, WORLD, [s.x, s.y, s.z], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(NEIGHBOUR, WORLD, [n.x, n.y, n.z], [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    for (id, pos) in [(SHOT, s), (NEIGHBOUR, n)] {
        let npc = mgr.get_entity_mut(id).unwrap();
        npc.template_id = Some(24);
        npc.faction = HOSTILE_FACTION;
        npc.spawn_position = Some(pos);
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
    Some(Fixture { mgr, guard_los })
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

fn engaged(mgr: &SpaceManager, id: u32) -> bool {
    let npc = mgr.get_entity(id).unwrap();
    npc.ai_state() == AiState::Fighting && npc.threat_list.contains_key(&PLAYER)
}

fn has_row(logs: &LogCaptureGuard, target: &str, pairs: &[(&str, &str)]) -> bool {
    logs.all()
        .iter()
        .any(|c| c.target == target && pairs.iter().all(|(k, v)| c.has_field(k, v)))
}

/// Shooting MessHall_Guard1 brings MessHall_Guard2 in (`cause=assist`).
/// Without assist Guard2 would stand and watch: the shooter is outside its
/// aggro radius, so its own Idle scan never picks the player.
#[tokio::test]
async fn shooting_messhall_guard1_pulls_in_guard2() {
    let Some(mut f) = fixture(MESSHALL_GUARD1, MESSHALL_GUARD2, MESSHALL_SHOOTER) else {
        return;
    };
    assert_eq!(f.guard_los, LineOfSight::Clear, "the guards share a room");

    // Before the shot, a tick leaves both guards Idle: this is the negative
    // control for the assist below.
    tick(&mut f.mgr).await;
    assert_eq!(
        f.mgr.get_entity(NEIGHBOUR).unwrap().ai_state(),
        AiState::Idle
    );
    assert_eq!(f.mgr.get_entity(SHOT).unwrap().ai_state(), AiState::Idle);

    let logs = LogCapture::install();
    let _ = generate_threat(&mut f.mgr, PLAYER, SHOT, 10.0, AggroCause::Damage);
    assert!(engaged(&f.mgr, SHOT));
    assert!(engaged(&f.mgr, NEIGHBOUR), "Guard2 must assist Guard1");
    assert!(has_row(
        &logs,
        "npc_ai.aggro",
        &[
            ("event", "acquired"),
            ("cause", "assist"),
            ("npc_id", &NEIGHBOUR.to_string()),
        ],
    ));

    tick(&mut f.mgr).await;
    assert!(
        engaged(&f.mgr, NEIGHBOUR),
        "still on the shooter a tick later"
    );
}

/// The Hallway guards are 18.6 u apart: shooting Hallway01 leaves Hallway02
/// Idle, and the reject row says why.
#[tokio::test]
async fn hallway_guards_more_than_10_u_apart_do_not_assist() {
    let Some(mut f) = fixture(HALLWAY01_GUARD, HALLWAY02_GUARD, MESSHALL_SHOOTER) else {
        return;
    };
    let logs = LogCapture::install();
    let _ = generate_threat(&mut f.mgr, PLAYER, SHOT, 10.0, AggroCause::Damage);
    assert!(engaged(&f.mgr, SHOT));
    let hallway02 = f.mgr.get_entity(NEIGHBOUR).unwrap();
    assert_eq!(hallway02.ai_state(), AiState::Idle);
    assert!(hallway02.threat_list.is_empty());
    assert!(has_row(
        &logs,
        "npc_ai.aggro_scan",
        &[
            ("event", "assist_rejected"),
            ("reason", "out_of_radius"),
            ("npc_id", &NEIGHBOUR.to_string()),
        ],
    ));

    tick(&mut f.mgr).await;
    assert!(f.mgr.get_entity(NEIGHBOUR).unwrap().threat_list.is_empty());
}
