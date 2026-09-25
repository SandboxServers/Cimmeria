//! NA27's Castle_CellBlock line-of-sight pins, re-checked at the seeded
//! body-set eye heights (NA31). `occluder_los` pins them at 1.5 m on both
//! ends; here each being looks from its own eyes: the guards are
//! `BS_HumanMale` (1.81 m), the Find Ambernol drone is
//! `BS_MOB_DroneFlyer` (3.31 m, its mesh floats), and the player is a
//! human male or a Jaffa male (2.12 m). A regression here means the new
//! eye heights broke a verdict UAT-1 depended on.

use std::sync::Arc;

use cimmeria_entity::navigation::LineOfSight;
use cimmeria_occluder::PagedOccluder;

use crate::cell::space_manager::SpaceManager;

const NPC: u32 = 200;
const PLAYER: u32 = 100;
const WORLD: &str = "Castle_CellBlock";

const HUMAN: &str = "BS_HumanMale.BS_HumanMale";
const JAFFA: &str = "BS_JaffaMale.BS_JaffaMale";
const DRONE_FLYER: &str = "MOB_CA_DroneTank.BS_MOB_DroneFlyer";

const DRONE: [f32; 3] = [-220.257, 66.744, -121.375];
const SOUTH_OF_THE_DESK: [f32; 3] = [-234.0, 65.6, -127.7];
const HALLWAY02_GUARD: [f32; 3] = [-113.485, 39.552, -63.042];
const HALLWAY02_VICTIM: [f32; 3] = [-130.31534, 39.5495, -79.506516];
const HALLWAY01_GUARD: [f32; 3] = [-128.853, 39.552, -73.534];
const LOMIADA_13_6: [f32; 3] = [-133.3383, 39.5515, -86.377625];
const LOMIADA_8_1: [f32; 3] = [-130.86244, 39.5515, -81.4195];
const LOMIADA_5_9: [f32; 3] = [-130.30719, 39.5515, -79.26729];
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
    Some(Arc::new(PagedOccluder::load(&p).expect("load")))
}

/// NPC 200 (`npc_body`) at `npc`, player 100 (`player_body`) at `player`,
/// with the seeded eye heights of the three body sets.
fn los(npc: [f32; 3], npc_body: &str, player: [f32; 3], player_body: &str) -> Option<LineOfSight> {
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
    for (bs, h) in [(HUMAN, 1.81), (JAFFA, 2.12), (DRONE_FLYER, 3.31)] {
        mgr.body_set_eye_heights.insert(bs.to_string(), h);
    }
    let sid = mgr.spawn_npc(NPC, WORLD, npc, [0.0; 3]).unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder = Some(occ);
    mgr.get_entity_mut(NPC).unwrap().body_set = Some(npc_body.to_string());
    mgr.create_entity(PLAYER, WORLD, player, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.body_set = Some(player_body.to_string());
    Some(mgr.line_of_sight(NPC, PLAYER))
}

#[test]
fn the_drone_still_sees_over_the_desk_from_its_floating_eye() {
    for body in [HUMAN, JAFFA] {
        let Some(l) = los(DRONE, DRONE_FLYER, SOUTH_OF_THE_DESK, body) else {
            return;
        };
        assert_eq!(l, LineOfSight::Clear, "player {body}");
    }
}

#[test]
fn hallway02_guard_still_does_not_see_through_the_walls() {
    for body in [HUMAN, JAFFA] {
        let Some(l) = los(HALLWAY02_GUARD, HUMAN, HALLWAY02_VICTIM, body) else {
            return;
        };
        assert_eq!(l, LineOfSight::Blocked, "player {body}");
    }
}

#[test]
fn hallway01_guard_still_sees_lomiada_over_its_counter() {
    for spot in [LOMIADA_5_9, LOMIADA_8_1, LOMIADA_13_6] {
        for body in [HUMAN, JAFFA] {
            let Some(l) = los(HALLWAY01_GUARD, HUMAN, spot, body) else {
                return;
            };
            assert_eq!(l, LineOfSight::Clear, "{spot:?} player {body}");
        }
    }
}

#[test]
fn guard_room_walls_and_storeys_still_block() {
    for (a, b, what) in [
        (
            ARMORY_GUARD,
            BARRACKS_GUARD,
            "Armory to Barracks, same floor",
        ),
        (
            MESSHALL_GUARD,
            BARRACKS_GUARD,
            "MessHall to the Barracks below",
        ),
        (MESSHALL_GUARD, HALLWAY01_GUARD, "MessHall to Hallway01"),
    ] {
        for body in [HUMAN, JAFFA] {
            let Some(l) = los(a, HUMAN, b, body) else {
                return;
            };
            assert_eq!(l, LineOfSight::Blocked, "{what}, player {body}");
        }
    }
}
