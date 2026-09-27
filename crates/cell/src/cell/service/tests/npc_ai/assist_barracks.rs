//! Barracks rally (colo UAT 2026-09-26, mission 687 step 2355): the three
//! `Barracks_Guard1/2/3` NID Guards (template 24) share one room, but the
//! room is 25 u across, and at the NA14 default assist radius of 10 u every
//! shot left at least one guard watching. SigNoz 02:42:42 recorded exactly
//! that: `Barracks_Guard2` proximity-aggroed the player, and Guard1 (13.1 u
//! away) and Guard3 (19.3 u) logged `assist_rejected reason=out_of_radius
//! assist_radius=10`.
//!
//! The fix is the D-NA09 UAT tuning: `entity_templates.assist_radius = 26`
//! on template 24 (`db/resources/Entities/Seed/entity_templates.sql`). These
//! tests run the assist over the world's real collision occluder
//! (`castle_cellblock.occ`, the source production line of sight comes from
//! when a world ships one), so the room's own walls and furniture decide
//! who sees whom. The seed value itself is pinned by the live-DB guard
//! `spawner::tests::live_db_assist::barracks_guards_assist_radius_covers_the_room`.
//!
//! Skips on a checkout without the fixture.

use std::sync::Arc;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::LineOfSight;
use cimmeria_entity::stats::HEALTH;
use cimmeria_occluder::PagedOccluder;

use crate::cell::combat::{generate_threat, AggroCause, HOSTILE_FACTION};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const PLAYER: u32 = 100;
const WORLD: &str = "Castle_CellBlock";

/// The seeded template-24 radius under test (mirrors the seed row).
const SEEDED_NID_GUARD_ASSIST_RADIUS: f32 = 26.0;

/// `spawnlist.sql` rows, all on the lower floor (y 24.67).
const GUARD1: (u32, [f32; 3]) = (201, [-118.41, 24.67, -118.35]); // spawn 25
const GUARD2: (u32, [f32; 3]) = (202, [-131.48, 24.67, -116.97]); // spawn 26
const GUARD3: (u32, [f32; 3]) = (203, [-136.385, 24.671, -135.671]); // spawn 36
const GUARDS: [(u32, [f32; 3]); 3] = [GUARD1, GUARD2, GUARD3];

/// Where the shooter stands: the barracks floor, inside the room.
const SHOOTER: [f32; 3] = [-125.0, 24.67, -128.0];

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

/// The three guards Idle at their spawns with `radius` as their template
/// assist radius (`None` = the 10 u default), and the player in the room.
fn barracks(radius: Option<f32>) -> Option<SpaceManager> {
    scene(&GUARDS, SHOOTER, radius)
}

/// NID Guards (template 24, faction 10) Idle at `guards`, the occluder
/// attached, and the player at `player`.
fn scene(
    guards: &[(u32, [f32; 3])],
    player: [f32; 3],
    radius: Option<f32>,
) -> Option<SpaceManager> {
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
    let mut space_id = None;
    for &(id, pos) in guards {
        space_id = Some(mgr.spawn_npc(id, WORLD, pos, [0.0; 3]).unwrap());
        let npc = mgr.get_entity_mut(id).unwrap();
        npc.template_id = Some(24);
        npc.faction = HOSTILE_FACTION;
        npc.spawn_position = Some(Vector3::new(pos[0], pos[1], pos[2]));
        npc.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
        npc.aggro.assist_radius_override = radius;
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
    }
    mgr.spaces.get_mut(&space_id.unwrap()).unwrap().occluder = Some(occ);
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

fn engaged(mgr: &SpaceManager, id: u32) -> bool {
    let npc = mgr.get_entity(id).unwrap();
    npc.ai_state() == AiState::Fighting && npc.threat_list.contains_key(&PLAYER)
}

/// Precondition: the guards see each other through the real collision
/// geometry, so the room, not the radius, is what the tuning relies on.
#[test]
fn the_barracks_guards_see_each_other() {
    let Some(mgr) = barracks(None) else { return };
    for (a, _) in GUARDS {
        for (b, _) in GUARDS {
            if a != b {
                assert_eq!(
                    mgr.line_of_sight(a, b),
                    LineOfSight::Clear,
                    "guard {a} -> guard {b}"
                );
            }
        }
    }
}

/// With the seeded radius, shooting any one guard rallies the other two
/// (`cause=assist`), whichever guard is shot.
#[tokio::test]
async fn shooting_any_barracks_guard_rallies_the_other_two() {
    for (shot, _) in GUARDS {
        let Some(mut mgr) = barracks(Some(SEEDED_NID_GUARD_ASSIST_RADIUS)) else {
            return;
        };
        let _ = generate_threat(&mut mgr, PLAYER, shot, 10.0, AggroCause::Damage);
        for (id, _) in GUARDS {
            assert!(
                engaged(&mgr, id),
                "shot guard {shot}: guard {id} must be fighting the player"
            );
        }
    }
}

/// The UAT bug, pinned: at the 10 u default, shooting Guard1 leaves Guard3
/// (25 u away) out, and the reject row says `out_of_radius`.
#[tokio::test]
async fn at_the_default_radius_guard3_watches_guard1_die() {
    let Some(mut mgr) = barracks(None) else {
        return;
    };
    let logs = LogCapture::install();
    let _ = generate_threat(&mut mgr, PLAYER, GUARD1.0, 10.0, AggroCause::Damage);
    assert!(engaged(&mgr, GUARD1.0));
    assert!(!engaged(&mgr, GUARD3.0));
    // Guard2, 13.1 u away, is also out at 10 u (the 02:42:42 reject row).
    assert!(logs.all().iter().any(|c| c.target == "npc_ai.aggro_scan"
        && c.has_field("event", "assist_rejected")
        && c.has_field("reason", "out_of_radius")
        && c.has_field("npc_id", &GUARD2.0.to_string())));
}

/// The tuning is per template, so every NID Guard in the Cellblock gets the
/// 26 u radius. The ones it newly reaches on their own floor are the three
/// upper-hallway guards (18.6 u and 20.5 u apart), and the corridor walls
/// still keep them apart: shooting `Hallway02_Guard` rallies neither
/// neighbour. If a rebuilt occluder opens those corners, this says so.
#[tokio::test]
async fn the_tuned_radius_does_not_link_the_upper_hallway_guards() {
    const HALLWAY01: (u32, [f32; 3]) = (211, [-128.853, 39.552, -73.534]); // spawn 30
    const HALLWAY02: (u32, [f32; 3]) = (212, [-113.485, 39.552, -63.042]); // spawn 82
    const HALLWAY03: (u32, [f32; 3]) = (213, [-98.58, 39.55, -77.09]); // spawn 31
    let Some(mut mgr) = scene(
        &[HALLWAY01, HALLWAY02, HALLWAY03],
        [-113.0, 39.55, -58.0],
        Some(SEEDED_NID_GUARD_ASSIST_RADIUS),
    ) else {
        return;
    };
    let logs = LogCapture::install();
    let _ = generate_threat(&mut mgr, PLAYER, HALLWAY02.0, 10.0, AggroCause::Damage);
    assert!(engaged(&mgr, HALLWAY02.0));
    for (id, _) in [HALLWAY01, HALLWAY03] {
        assert!(
            mgr.get_entity(id).unwrap().threat_list.is_empty(),
            "hallway guard {id} must not rally through the corridor walls"
        );
        assert!(logs.all().iter().any(|c| c.target == "npc_ai.aggro_scan"
            && c.has_field("event", "assist_rejected")
            && c.has_field("reason", "no_los")
            && c.has_field("npc_id", &id.to_string())));
    }
}
