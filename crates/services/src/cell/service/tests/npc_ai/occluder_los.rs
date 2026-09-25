//! NA27 on the real `castle_cellblock.occ`: where a world ships a
//! collision-geometry occluder, NPC line of sight comes from it, eye to eye
//! at 1.5 m, instead of the navmesh ray (D-NA13).
//!
//! Every position is from a seed row or a UAT-1 `npc_ai.los` row (the colo
//! session of 2026-09-25, build `059d6038`):
//!
//! - the Find Ambernol drone (spawn 10) and the player south of the
//!   med-station desk (NA16);
//! - `Hallway02_Guard` (spawn 82) and where it shot the player dead round
//!   two hallway corners (NA23);
//! - `Hallway01_Guard` (spawn 30) and the three places Lomiada stood when
//!   it rejected her `no_los` at 5.9, 8.1 and 13.6 u;
//! - the Armory, Barracks and MessHall guard spawns for walls and storeys.
//!
//! Only the occluder is injected: none of these tests depends on the
//! navmesh, which NA28 is rebuilding. The navmesh-only behaviour of the
//! same positions stays pinned by `stationary_los` and `npc_ai_cover_peek`.

use std::sync::Arc;

use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::LineOfSight;
use cimmeria_entity::stats::HEALTH;
use cimmeria_occluder::PagedOccluder;
use tokio::sync::mpsc;

use super::seed_default_ability;
use crate::cell::combat::{HOSTILE_FACTION, NPC_DEFAULT_ABILITY};
use crate::cell::space_manager::{AttackLosPolicy, SpaceManager};
use crate::test_support::LogCapture;

const NPC: u32 = 200;
const PLAYER: u32 = 100;
const WORLD: &str = "Castle_CellBlock";

/// Spawn 10, the drone.
const DRONE: [f32; 3] = [-220.257, 66.744, -121.375];
/// South of the med-station desk, 15.2 m from the drone, same floor.
const SOUTH_OF_THE_DESK: [f32; 3] = [-234.0, 65.6, -127.7];
/// Spawn 82 and its cover marker 1200034/0.
const HALLWAY02_GUARD: [f32; 3] = [-113.485, 39.552, -63.042];
/// Where `Hallway02_Guard` shot the player through the walls (UAT-1).
const HALLWAY02_VICTIM: [f32; 3] = [-130.31534, 39.5495, -79.506516];
/// Spawn 30, behind its counter.
const HALLWAY01_GUARD: [f32; 3] = [-128.853, 39.552, -73.534];
/// Lomiada at 13.6 u (12:16:20-12:16:32), 8.1 u (12:16:44), 5.9 u (12:16:38).
const LOMIADA_13_6: [f32; 3] = [-133.3383, 39.5515, -86.377625];
const LOMIADA_8_1: [f32; 3] = [-130.86244, 39.5515, -81.4195];
const LOMIADA_5_9: [f32; 3] = [-130.30719, 39.5515, -79.26729];
/// Spawnlist guard rows: the Armory and Barracks on the lower floor, the
/// MessHall one storey up.
const ARMORY_GUARD: [f32; 3] = [-49.69, 24.67, -127.11];
const BARRACKS_GUARD: [f32; 3] = [-131.48, 24.67, -116.97];
const MESSHALL_GUARD: [f32; 3] = [-95.89, 34.591, -98.808];

fn occluder() -> Option<Arc<PagedOccluder>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.occ");
    if !p.exists() {
        eprintln!("SKIPPED: {} is absent", p.display());
        return None;
    }
    Some(Arc::new(
        PagedOccluder::load(&p).unwrap_or_else(|e| panic!("load {}: {e}", p.display())),
    ))
}

/// A Castle_CellBlock space with NPC 200 at `npc` and player 100 at
/// `player`, both on the NPC's AoI, the occluder attached (or not).
fn scene(npc: [f32; 3], player: [f32; 3], with_occluder: bool) -> Option<SpaceManager> {
    let occ = occluder()?;
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.spawn_npc(NPC, WORLD, npc, [0.0; 3]).unwrap();
    if with_occluder {
        mgr.spaces.get_mut(&space_id).unwrap().occluder = Some(occ);
    }
    if let Some(e) = mgr.get_entity_mut(NPC) {
        e.template_id = Some(24);
        e.faction = HOSTILE_FACTION;
        e.class_id = 0x04;
        e.spawn_position = Some(cimmeria_common::Vector3::new(npc[0], npc[1], npc[2]));
        e.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
        crate::cell::service::npc_ai::force_ai_state(e, AiState::Idle);
    }
    mgr.create_entity(PLAYER, WORLD, player, [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
        p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    }
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    Some(mgr)
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

fn los(npc: [f32; 3], player: [f32; 3]) -> Option<LineOfSight> {
    let mgr = scene(npc, player, true)?;
    Some(mgr.line_of_sight(NPC, PLAYER))
}

/// Audit S11 without the D-NA11 workaround: the stationary drone sees over
/// the 1 m desk, and fires on the occluder's own verdict
/// (`los_policy=occluder`, not `stationary_relaxed`). Take the occluder
/// away and the same tick reads the navmesh again.
#[tokio::test]
async fn the_drone_sees_over_the_med_station_desk_and_fires_on_the_occluder() {
    let Some(mut mgr) = scene(DRONE, SOUTH_OF_THE_DESK, true) else {
        return;
    };
    assert_eq!(mgr.line_of_sight(NPC, PLAYER), LineOfSight::Clear);
    seed_default_ability(&mut mgr, 0, 30);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.threat_list.insert(PLAYER, 1.0);
        npc.abilities.add_ability(NPC_DEFAULT_ABILITY);
        npc.is_stationary = true;
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Fighting);
    }
    assert!(mgr.attack_line_of_sight(NPC, PLAYER, true));
    assert_eq!(
        mgr.attack_los_policy(NPC, PLAYER, true, LineOfSight::Clear),
        AttackLosPolicy::Occluder(true)
    );
    let capture = LogCapture::install();
    tick(&mut mgr).await;
    assert!(
        mgr.get_entity(NPC)
            .unwrap()
            .abilities
            .is_on_cooldown(NPC_DEFAULT_ABILITY),
        "the drone fires across the desk"
    );
    let row = capture
        .find_message(tracing::Level::DEBUG, "NPC AI tick")
        .expect("the drone's tick row");
    let f = |k: &str| row.fields.get(k).cloned().unwrap_or_default();
    assert!(f("los").contains("clear"), "los: {:?}", f("los"));
    assert!(
        f("los_policy").contains("occluder"),
        "los_policy: {:?}",
        f("los_policy")
    );
}

/// The gain for a mobile NPC: it no longer needs to walk round the desk.
/// On the navmesh (no occluder) the same shot is refused.
#[test]
fn a_mobile_npc_shoots_over_the_desk_only_with_the_occluder() {
    let Some(with) = scene(DRONE, SOUTH_OF_THE_DESK, true) else {
        return;
    };
    assert!(with.attack_line_of_sight(NPC, PLAYER, false));
    assert_eq!(
        with.attack_los_policy(NPC, PLAYER, false, LineOfSight::Blocked),
        AttackLosPolicy::Occluder(false),
        "an occluder Blocked is not relaxed for anyone"
    );
    assert_eq!(
        with.attack_los_policy(NPC, PLAYER, true, LineOfSight::Blocked),
        AttackLosPolicy::Occluder(false),
        "not even for a stationary NPC: D-NA11 is retired here"
    );
}

/// UAT-1: `Hallway02_Guard` killed the player through two walls at 23.5 u.
/// From its cover marker the occluder is blocked, for aggro and for the shot.
#[test]
fn hallway02_guard_does_not_see_through_the_hallway_walls() {
    let Some(mgr) = scene(HALLWAY02_GUARD, HALLWAY02_VICTIM, true) else {
        return;
    };
    assert_eq!(mgr.line_of_sight(NPC, PLAYER), LineOfSight::Blocked);
    assert!(!mgr.attack_line_of_sight(NPC, PLAYER, false));
    assert!(!mgr.attack_line_of_sight(NPC, PLAYER, true));
}

/// UAT-1 defect 1: `Hallway01_Guard` rejected Lomiada `no_los` from behind
/// its own counter at 5.9, 8.1 and 13.6 u. At eye height it sees over the
/// counter at all three, including the 13.6 u spot the peek point (NA23)
/// still read as blocked.
#[tokio::test]
async fn hallway01_guard_sees_lomiada_over_its_counter_and_aggroes() {
    for spot in [LOMIADA_5_9, LOMIADA_8_1, LOMIADA_13_6] {
        let Some(mut mgr) = scene(HALLWAY01_GUARD, spot, true) else {
            return;
        };
        assert_eq!(
            mgr.line_of_sight(NPC, PLAYER),
            LineOfSight::Clear,
            "{spot:?}"
        );
        tick(&mut mgr).await;
        assert_eq!(
            mgr.get_entity(NPC).unwrap().ai_state(),
            AiState::Fighting,
            "the guard aggroes Lomiada at {spot:?}"
        );
    }
}

/// Walls between rooms on one floor, and the floor between storeys, block.
#[test]
fn guard_room_walls_and_storeys_block() {
    for (a, b, what) in [
        (
            ARMORY_GUARD,
            BARRACKS_GUARD,
            "Armory to Barracks, same floor",
        ),
        (
            MESSHALL_GUARD,
            BARRACKS_GUARD,
            "MessHall to the Barracks a storey down",
        ),
        (MESSHALL_GUARD, HALLWAY01_GUARD, "MessHall to Hallway01"),
    ] {
        match los(a, b) {
            None => return,
            Some(l) => assert_eq!(l, LineOfSight::Blocked, "{what}"),
        }
    }
}

/// Off the occluder's trimmed area (the map-sized terrain sheet outside
/// the Cellblock interior) the answer is `Unknown`: aggro fails closed
/// (D-NA08), and the attack check falls back to the navmesh rules.
#[tokio::test]
async fn off_the_explorable_area_aggro_fails_closed_and_attack_falls_back() {
    const FAR_TERRAIN: [f32; 3] = [300.0, 0.2, 300.0];
    let Some(mut mgr) = scene(FAR_TERRAIN, [305.0, 0.2, 300.0], true) else {
        return;
    };
    assert_eq!(mgr.line_of_sight(NPC, PLAYER), LineOfSight::Unknown);
    let policy = mgr.attack_los_policy(NPC, PLAYER, false, LineOfSight::Unknown);
    assert_eq!(
        policy,
        AttackLosPolicy::Strict(true),
        "navmesh rules, not Occluder"
    );
    tick(&mut mgr).await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Idle,
        "an Unknown line of sight never pulls aggro"
    );
}

/// The `npc_ai.los` row says which source answered and the eye height it
/// used, so a SigNoz view can tell occluder verdicts from navmesh ones.
#[test]
fn the_los_row_names_the_occluder_and_its_eye_height() {
    let Some(mgr) = scene(HALLWAY02_GUARD, HALLWAY02_VICTIM, true) else {
        return;
    };
    let capture = LogCapture::install();
    let _ = mgr.line_of_sight(NPC, PLAYER);
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "npc_ai.los")
        .expect("a blocked verdict is reported");
    assert!(row.has_field("source", "occluder"), "{row:?}");
    assert!(row.has_field("eye_height_used", "1.5"), "{row:?}");
    assert!(row.fields.contains_key("occluder_hash"), "{row:?}");
}

/// Paging: the pages round the player are unpacked by the 1 Hz refresh,
/// and dropped when nobody is left in the world.
#[test]
fn residency_follows_the_player_and_empties_when_they_leave() {
    let Some(mut mgr) = scene(DRONE, SOUTH_OF_THE_DESK, true) else {
        return;
    };
    let key = WORLD.to_lowercase();
    let occ = mgr
        .spaces
        .values()
        .find_map(|s| s.occluder.clone())
        .unwrap();
    // The fixture attached it by hand; register it as the loader would.
    mgr.occluders.insert(key, Some(occ.clone()));
    let capture = LogCapture::install();
    mgr.refresh_occluder_residency();
    let resident = occ.stats().resident_pages;
    assert!(resident > 0, "the player's pages are unpacked");
    assert!(resident < occ.stats().pages, "and not the whole world");
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "npc_ai.occluder" && c.has_field("event", "residency")));
    for s in mgr.spaces.values_mut() {
        s.players.remove(&PLAYER);
    }
    mgr.refresh_occluder_residency();
    assert_eq!(occ.stats().resident_pages, 0);
    assert_eq!(occ.stats().resident_bytes, 0);
}
