//! NA12: the walk home over the real `castle_cellblock.nav`.
//!
//! The NPC stands at `Hallway01_Guard`'s spawn and its home is
//! `MessHall_Guard1`'s spawn, two points on the same topside component (the
//! GC1 escort tests use the same pair). Both are projected onto the mesh so
//! the pathfinder's 0.5 u start box finds them (audit S9 is NA11's).
//!
//! The mesh is injected into the space: the production loader keys off a
//! cwd-relative path the test harness does not satisfy. Skips on a checkout
//! without the fixture.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const NPC: u32 = 200;
const PLAYER: u32 = 101;
const WORLD: &str = "Castle_CellBlock";
/// `MessHall_Guard1`'s spawn (`spawnlist.sql`): the NPC's home.
const HOME: [f32; 3] = [-96.25, 34.591, -91.59];
/// `Hallway01_Guard`'s spawn: where the NPC stands when the fight ends,
/// about 37 u from home.
const AWAY: [f32; 3] = [-128.853, 39.552, -73.534];
const SPAWN_FACING: Vector3 = Vector3 {
    x: 0.0,
    y: 1.25,
    z: 0.0,
};

struct Walk {
    mgr: SpaceManager,
    home: Vector3,
    away: Vector3,
}

/// A Fighting NPC at `AWAY`, damaged to 40/100, whose spawn is `HOME`, and a
/// connected player 4 u from it, on its threat list and in combat.
fn walk_fixture(player_health: i32) -> Option<Walk> {
    let nav = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav).expect("load castle_cellblock.nav");
    let snap = |p: [f32; 3]| navmesh.get_nearest_point(&Vector3::new(p[0], p[1], p[2]));
    let (home, away) = (snap(HOME), snap(AWAY));

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
        .spawn_npc(NPC, WORLD, [away.x, away.y, away.z], [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Fighting);
        npc.spawn_position = Some(home);
        npc.spawn_direction = Some(SPAWN_FACING);
        npc.move_speed = 0.6;
        let h = npc.stats.get_mut(HEALTH).unwrap();
        h.update(0, 40, 100);
        h.clear_dirty();
        npc.threat_list.insert(PLAYER, 10.0);
    }
    let p = [away.x + 4.0, away.y, away.z];
    mgr.create_entity(PLAYER, WORLD, p, [0.0; 3]).unwrap();
    if let Some(pl) = mgr.get_entity_mut(PLAYER) {
        pl.is_player = true;
        pl.player_id = Some(PLAYER as i32);
        pl.stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, player_health, 100);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    let _ = combat::enter_player_combat(&mut mgr, PLAYER, NPC);
    Some(Walk { mgr, home, away })
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Assert the NPC is Leashing with a route that ends at home and has not
/// moved: the leash starts a walk, not a teleport.
fn assert_walking_home(w: &Walk) {
    let npc = w.mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Leashing);
    assert_eq!(npc.position, w.away, "entering Leashing must not teleport");
    let end = npc
        .nav_path
        .back()
        .copied()
        .expect("a route home must be installed on entering Leashing");
    assert!(
        horizontal(&end, &w.home) < 1.0,
        "the route must end at spawn, ends at {end:?}"
    );
}

/// D-NA03 walk home: the NPC walks the route home one movement step at a
/// time (never further than `move_speed` per 100 ms tick), then on arrival
/// heals to full, faces its authored heading, has no route and no velocity
/// left, and is Idle under `leash_arrived` (not the snap fallback).
///
/// Reverting to the snap leash fails the first step-size check (a 37 u
/// jump), and reverting the arrival reset fails the health / facing / route
/// assertions.
#[tokio::test]
async fn npc_walks_home_on_the_navmesh_and_resets_on_arrival() {
    let Some(mut w) = walk_fixture(100) else {
        return;
    };
    // Give up at 20 u: the NPC is ~37 u out, beyond the 25 u band.
    w.mgr.get_entity_mut(NPC).unwrap().leash.distance_override = Some(20.0);
    let logs = LogCapture::install();

    ai_tick(&mut w.mgr).await;
    assert_walking_home(&w);
    assert!(
        w.mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty(),
        "the player's combat state is drained when the NPC gives up"
    );

    let mut prev = w.away;
    let mut ticks = 0;
    while w.mgr.get_entity(NPC).unwrap().ai_state() == AiState::Leashing {
        assert!(ticks < 400, "the NPC never arrived (40 s of movement)");
        crate::cell::service::ticks::npc_movement_tick(&mut w.mgr);
        ticks += 1;
        let pos = w.mgr.get_entity(NPC).unwrap().position;
        let step = horizontal(&prev, &pos);
        assert!(
            step <= 0.6 + 0.01,
            "tick {ticks}: the NPC moved {step} u in one 100 ms step, a teleport"
        );
        prev = pos;
        if ticks % 20 == 0 {
            ai_tick(&mut w.mgr).await;
        }
    }

    let npc = w.mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert!(
        horizontal(&npc.position, &w.home) <= 1.5,
        "arrived at spawn: {:?} vs {:?}",
        npc.position,
        w.home
    );
    assert!(npc.nav_path.is_empty(), "no stale route after arrival");
    assert_eq!(npc.velocity, [0.0; 3]);
    assert_eq!(npc.stats.get(HEALTH).unwrap().cur, 100, "healed to full");
    assert_eq!(npc.direction, SPAWN_FACING, "authored facing restored");
    assert!(npc.leash.walk_started_at.is_none());
    assert!(npc.leash.reaggro_suppressed_until.is_some());
    let idle_reasons: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.transition" && c.has_field("to", "idle"))
        .filter_map(|c| c.fields.get("reason").cloned())
        .collect();
    assert_eq!(idle_reasons, ["leash_arrived"], "walked, not snapped");
}

/// S6: the target dies with the NPC 37 u from spawn. The NPC starts walking
/// home rather than parking Idle where it stands.
#[tokio::test]
async fn target_death_walks_the_npc_home() {
    let Some(mut w) = walk_fixture(0) else {
        return;
    };
    ai_tick(&mut w.mgr).await;
    assert_walking_home(&w);
    assert!(w.mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty());
}

/// A walk that runs past the timeout is abandoned for a snap to spawn with
/// the authored facing, under `leash_snap_fallback`.
#[tokio::test]
async fn walk_past_the_timeout_snaps_home() {
    let Some(mut w) = walk_fixture(0) else {
        return;
    };
    ai_tick(&mut w.mgr).await;
    assert_walking_home(&w);
    w.mgr.get_entity_mut(NPC).unwrap().leash.walk_started_at =
        Some(Instant::now() - Duration::from_secs(21));
    let logs = LogCapture::install();

    ai_tick(&mut w.mgr).await;

    let npc = w.mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.position, w.home, "snapped to spawn");
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.direction, SPAWN_FACING);
    assert!(logs
        .all()
        .into_iter()
        .any(|c| c.target == "npc_ai.transition" && c.has_field("reason", "leash_snap_fallback")));
}
