//! Fire-time line of sight for player abilities (NA31, D-NA14).
//!
//! The refusal is `onErrorCode(0, ability_id, 39)` (`CONDITION_FEEDBACK_LOS`,
//! the one LoS entry in `ErrorStrings.pak` with authored text). Two
//! fixtures:
//!
//! - the real `castle_cellblock.occ` at UAT-1 positions: the spot where
//!   `Hallway02_Guard` shot the player round two hallway corners (a real
//!   wall), and Lomiada's 13.6 u spot in front of `Hallway01_Guard`'s counter
//!   (a clear line);
//! - a synthetic corner (a 4 m wall on x 19.85-20.15 ending at z 20, built
//!   with the shipped builder at the shipped 0.5 m cell) for the tolerance
//!   rays, where the numbers can be chosen with margins. At x 21, one metre
//!   behind the wall, an eye ray from (5, 10) is clear for a target at
//!   z > 20.81 and blocked below it.

use std::sync::Arc;

use cimmeria_entity::abilities::{TARGET_GROUND, TARGET_SELF, TARGET_TARGET};
use cimmeria_occluder::PagedOccluder;

use super::*;
use crate::cell::abilities::{fire_line_of_sight, FireLos};
use crate::cell::space_manager::occluder_fixtures::{corner, synthetic};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const ABILITY: i32 = 7;
/// Player run speed on world 12, u/s (see `entity_templates.move_speed`).
const RUN: f32 = 8.125;

/// `Hallway02_Guard` (spawn 82) and where it shot the player through the
/// walls in UAT-1, 23.5 u apart.
const HALLWAY02_GUARD: [f32; 3] = [-113.485, 39.552, -63.042];
const HALLWAY02_VICTIM: [f32; 3] = [-130.31534, 39.5495, -79.506516];
/// `Hallway01_Guard` (spawn 30) and Lomiada 13.6 u in front of its counter.
const HALLWAY01_GUARD: [f32; 3] = [-128.853, 39.552, -73.534];
const LOMIADA_13_6: [f32; 3] = [-133.3383, 39.5515, -86.377625];

fn cellblock_occluder() -> Option<Arc<PagedOccluder>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.occ");
    if !p.exists() {
        eprintln!("SKIPPED: {} is absent", p.display());
        return None;
    }
    Some(Arc::new(
        PagedOccluder::load(&p).expect("load castle_cellblock.occ"),
    ))
}

/// Player 1 at `player`, hostile NPC 2 at `npc`, both in one
/// Castle_CellBlock space with `occ` attached (or none). The player knows
/// ability 7: single target, 40 u, no ammo.
fn scene(occ: Option<Arc<PagedOccluder>>, player: [f32; 3], npc: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let sid = mgr
        .create_entity(PLAYER, "Castle_CellBlock", player, [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&sid).unwrap().occluder = occ;
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.is_player = true;
        p.player_id = Some(101);
        p.abilities.add_ability(ABILITY);
    }
    mgr.create_entity(NPC, "Castle_CellBlock", npc, [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(NPC) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    let mut def = make_ability(ABILITY, 0, 40);
    def.target_type_id = TARGET_TARGET;
    mgr.ability_defs.insert(ABILITY, def);
    mgr
}

fn verdict(mgr: &SpaceManager) -> FireLos {
    fire_line_of_sight(mgr, PLAYER, NPC, mgr.ability_defs.get(&ABILITY))
}

fn error_codes(msgs: &[CellToBaseMsg]) -> Vec<Vec<u8>> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index,
                args,
            } if *method_index == method_idx::ON_ERROR_CODE => Some(args.clone()),
            _ => None,
        })
        .collect()
}

async fn fire(mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(64);
    let committed = handle_use_ability(PLAYER, ABILITY, NPC as i32, &tx, mgr).await;
    (committed, drain(&mut rx))
}

/// UAT-1's through-the-walls shot, reversed: the player cannot shoot
/// `Hallway02_Guard` from where it killed them. Exactly one packet, the
/// seven `onErrorCode` bytes with code 39, and no cooldown started.
#[tokio::test]
async fn a_shot_through_the_hallway_walls_is_refused_with_error_39() {
    let Some(occ) = cellblock_occluder() else {
        return;
    };
    let mut mgr = scene(Some(occ), HALLWAY02_VICTIM, HALLWAY02_GUARD);
    assert!(
        matches!(verdict(&mgr), FireLos::Refused(_)),
        "{:?}",
        verdict(&mgr)
    );
    let (committed, msgs) = fire(&mut mgr).await;
    assert!(!committed);
    assert_eq!(msgs.len(), 1, "only the error code goes out: {msgs:?}");
    assert_eq!(
        error_codes(&msgs),
        vec![vec![0x00, 0x07, 0x00, 0x00, 0x00, 0x27, 0x00]],
        "onErrorCode(SystemID 0, InstanceID 7, ErrorCodeID 39)"
    );
    assert!(
        !mgr.get_entity(PLAYER)
            .unwrap()
            .abilities
            .is_on_cooldown(ABILITY),
        "a refused shot must not start the cooldown"
    );
}

/// The refusal row carries the evidence (target `abilities`, exported at
/// DEBUG): source, both eye heights, the ray and where it stopped.
#[tokio::test]
async fn the_refusal_is_logged_with_its_ray() {
    let Some(occ) = cellblock_occluder() else {
        return;
    };
    let cap = crate::test_support::LogCapture::install();
    // The other refusal tests hit the same callsite on other threads with
    // no subscriber; make sure its cached interest includes this one.
    tracing::callsite::rebuild_interest_cache();
    let mut mgr = scene(Some(occ), HALLWAY02_VICTIM, HALLWAY02_GUARD);
    let _ = fire(&mut mgr).await;
    let rows = cap.all();
    let row = rows
        .iter()
        .find(|r| r.target == "abilities" && r.has_field("event", "los_refused"))
        .unwrap_or_else(|| panic!("no los_refused row: {rows:?}"));
    for field in [
        "shooter_eye",
        "target_eye",
        "ray_from",
        "ray_to",
        "hit_xyz",
        "world",
    ] {
        assert!(row.fields.contains_key(field), "{field} missing: {row:?}");
    }
    assert!(row.has_field("source", "occluder"), "{row:?}");
    assert!(row.has_field("world", "Castle_CellBlock"), "{row:?}");
}

/// Lomiada's 13.6 u spot sees `Hallway01_Guard` over its counter: the shot
/// commits and no error goes out.
#[tokio::test]
async fn a_clear_shot_over_the_counter_fires() {
    let Some(occ) = cellblock_occluder() else {
        return;
    };
    let mut mgr = scene(Some(occ), LOMIADA_13_6, HALLWAY01_GUARD);
    assert_eq!(
        verdict(&mgr),
        FireLos::Clear(super::super::fire_los::ClearRay::Eye)
    );
    let (committed, msgs) = fire(&mut mgr).await;
    assert!(committed);
    assert!(error_codes(&msgs).is_empty(), "{msgs:?}");
}

/// Without an occluder the navmesh would be the only source, and it reads
/// furniture as walls, so nothing is refused.
#[tokio::test]
async fn a_world_without_an_occluder_never_refuses() {
    let mut mgr = scene(None, HALLWAY02_VICTIM, HALLWAY02_GUARD);
    assert_eq!(verdict(&mgr), FireLos::NotChecked("no_occluder"));
    let (committed, msgs) = fire(&mut mgr).await;
    assert!(committed);
    assert!(error_codes(&msgs).is_empty(), "{msgs:?}");
}

/// An eye off the occluder's grid is `Unknown`, and allowed.
#[tokio::test]
async fn an_endpoint_off_the_occluder_grid_is_allowed() {
    let mut mgr = scene(Some(corner()), [-20.0, 0.0, 10.0], [5.0, 0.0, 10.0]);
    assert_eq!(verdict(&mgr), FireLos::Unknown);
    let (committed, msgs) = fire(&mut mgr).await;
    assert!(committed);
    assert!(error_codes(&msgs).is_empty(), "{msgs:?}");
}

/// Self and ground abilities, and NPC shooters, are not checked.
#[test]
fn only_a_player_aiming_at_another_entity_is_checked() {
    let mut mgr = scene(Some(corner()), [5.0, 0.0, 10.0], [21.0, 0.0, 18.0]);
    assert!(matches!(verdict(&mgr), FireLos::Refused(_)));
    for (tt, why) in [
        (TARGET_SELF, "self_ability"),
        (TARGET_GROUND, "ground_ability"),
    ] {
        mgr.ability_defs.get_mut(&ABILITY).unwrap().target_type_id = tt;
        assert_eq!(verdict(&mgr), FireLos::NotChecked(why));
    }
    mgr.ability_defs.get_mut(&ABILITY).unwrap().target_type_id = TARGET_TARGET;
    mgr.get_entity_mut(PLAYER).unwrap().is_player = false;
    assert_eq!(verdict(&mgr), FireLos::NotChecked("npc_shooter"));
}

/// A target two metres into the wall's shadow stays refused whatever it
/// and the shooter are doing: the tolerance rays do not open real walls.
#[test]
fn the_tolerance_rays_do_not_see_round_a_real_corner() {
    let mut mgr = scene(Some(corner()), [5.0, 0.0, 10.0], [21.0, 0.0, 18.0]);
    mgr.get_entity_mut(NPC).unwrap().velocity = [0.0, 0.0, -RUN];
    mgr.get_entity_mut(PLAYER).unwrap().velocity = [0.0, 0.0, RUN];
    let FireLos::Refused(r) = verdict(&mgr) else {
        panic!("{:?}", verdict(&mgr));
    };
    assert_eq!(
        r.rays, 5,
        "eye, target lagged, shooter lead, two body edges"
    );
}

/// The target is 0.6 m into the wall's shadow, running into cover. The
/// client still draws it where the server had it a tick ago, in the open:
/// the lagged ray clears the shot. Stationary, the same spot is refused.
#[tokio::test]
async fn a_target_running_into_cover_is_hit_where_the_client_still_sees_it() {
    let mut mgr = scene(Some(corner()), [5.0, 0.0, 10.0], [21.0, 0.0, 20.2]);
    assert!(matches!(verdict(&mgr), FireLos::Refused(_)));
    mgr.get_entity_mut(NPC).unwrap().velocity = [0.0, 0.0, -RUN];
    assert_eq!(
        verdict(&mgr),
        FireLos::Clear(super::super::fire_los::ClearRay::TargetLagged)
    );
    let (committed, msgs) = fire(&mut mgr).await;
    assert!(committed);
    assert!(error_codes(&msgs).is_empty(), "{msgs:?}");
}

/// The shooter is peeking out from behind the corner: its client has it a
/// tick further out than the server does.
#[test]
fn a_shooter_stepping_out_of_cover_is_ahead_on_its_own_client() {
    let mut mgr = scene(Some(corner()), [21.0, 0.0, 20.2], [5.0, 0.0, 10.0]);
    assert!(matches!(verdict(&mgr), FireLos::Refused(_)));
    mgr.get_entity_mut(PLAYER).unwrap().velocity = [0.0, 0.0, RUN];
    assert_eq!(
        verdict(&mgr),
        FireLos::Clear(super::super::fire_los::ClearRay::ShooterLead)
    );
}

/// The eye ray grazes 0.2 m inside the corner; the target's shoulder
/// (0.35 m to the side) is in the open.
#[test]
fn a_target_half_behind_the_corner_is_hit_on_its_body_edge() {
    let mgr = scene(Some(corner()), [5.0, 0.0, 10.0], [21.0, 0.0, 20.6]);
    assert_eq!(
        verdict(&mgr),
        FireLos::Clear(super::super::fire_los::ClearRay::BodyEdge)
    );
}

// ── Per-body-set eye heights (NA31) ────────────────────────────────────

/// Under spawn 10's spot (the Find Ambernol drone hovers 1.1 m up), on the
/// floor, and the player south of the med-station desk: 15.2 m apart on
/// one floor (NA16). The desk top is about 0.9 m above the floor.
const DESK_FAR_SIDE: [f32; 3] = [-220.257, 65.6, -121.375];
const SOUTH_OF_THE_DESK: [f32; 3] = [-234.0, 65.6, -127.7];

/// The seeded values for the body sets these tests use
/// (`db/resources/Visuals/Seed/body_sets.sql`).
fn seeded_eye_heights(mgr: &mut SpaceManager) {
    for (bs, h) in [
        ("BS_HumanMale.BS_HumanMale", 1.81),
        ("BS_JaffaMale.BS_JaffaMale", 2.12),
        ("MOB_AMBRat.BS_MOB_Rat", 0.15),
    ] {
        mgr.body_set_eye_heights.insert(bs.to_string(), h);
    }
}

fn set_body(mgr: &mut SpaceManager, id: u32, body_set: &str) {
    mgr.get_entity_mut(id).unwrap().body_set = Some(body_set.to_string());
}

#[test]
fn an_entity_takes_its_body_sets_eye_height_or_the_default() {
    use crate::cell::space_manager::{eye_height_for, DEFAULT_EYE_HEIGHT};
    let mut t = std::collections::HashMap::new();
    t.insert("BS_Asgard.BS_Asgard".to_string(), 1.25);
    t.insert("Broken.Zero".to_string(), 0.0);
    t.insert("Broken.NaN".to_string(), f32::NAN);
    assert_eq!(eye_height_for(Some("BS_Asgard.BS_Asgard"), &t), 1.25);
    assert_eq!(eye_height_for(None, &t), DEFAULT_EYE_HEIGHT);
    assert_eq!(
        eye_height_for(Some("GLB_Components.WorldObject_Small"), &t),
        DEFAULT_EYE_HEIGHT
    );
    assert_eq!(eye_height_for(Some("Broken.Zero"), &t), DEFAULT_EYE_HEIGHT);
    assert_eq!(eye_height_for(Some("Broken.NaN"), &t), DEFAULT_EYE_HEIGHT);
}

/// A 1 m wall one metre in front of the target. A human (eye 1.81 m) sees
/// a Jaffa's eyes (2.12 m) over it, but not a rat's (0.15 m): the body set
/// alone changes the verdict.
#[test]
fn a_low_wall_hides_a_rat_but_not_a_jaffa() {
    let wall = synthetic(&[([19.0, 0.0, 0.0], [19.3, 1.0, 40.0])]);
    let mut mgr = scene(Some(wall), [5.0, 0.0, 20.0], [20.3, 0.0, 20.0]);
    seeded_eye_heights(&mut mgr);
    set_body(&mut mgr, PLAYER, "BS_HumanMale.BS_HumanMale");
    set_body(&mut mgr, NPC, "BS_JaffaMale.BS_JaffaMale");
    assert_eq!(
        verdict(&mgr),
        FireLos::Clear(super::super::fire_los::ClearRay::Eye)
    );
    set_body(&mut mgr, NPC, "MOB_AMBRat.BS_MOB_Rat");
    let FireLos::Refused(r) = verdict(&mgr) else {
        panic!("{:?}", verdict(&mgr));
    };
    assert_eq!((r.shooter_eye, r.target_eye), (1.81, 0.15));
}

/// The same on the real Castle_CellBlock collision geometry: across the
/// med-station desk a player sees a human standing on the far side, and
/// does not see a rat there.
#[test]
fn across_the_med_station_desk_a_rat_is_hidden_and_a_human_is_not() {
    let Some(occ) = cellblock_occluder() else {
        return;
    };
    let mut mgr = scene(Some(occ), SOUTH_OF_THE_DESK, DESK_FAR_SIDE);
    seeded_eye_heights(&mut mgr);
    set_body(&mut mgr, PLAYER, "BS_HumanMale.BS_HumanMale");
    set_body(&mut mgr, NPC, "BS_HumanMale.BS_HumanMale");
    assert!(
        matches!(verdict(&mgr), FireLos::Clear(_)),
        "{:?}",
        verdict(&mgr)
    );
    set_body(&mut mgr, NPC, "MOB_AMBRat.BS_MOB_Rat");
    assert!(
        matches!(verdict(&mgr), FireLos::Refused(_)),
        "{:?}",
        verdict(&mgr)
    );
}

/// The NPC side uses the same lookup: `SpaceManager::line_of_sight` (aggro,
/// assist, attack) casts between the two body sets' eyes.
#[test]
fn npc_line_of_sight_casts_between_body_set_eyes() {
    use cimmeria_entity::navigation::LineOfSight;
    let wall = synthetic(&[([19.0, 0.0, 0.0], [19.3, 1.0, 40.0])]);
    let mut mgr = scene(Some(wall), [5.0, 0.0, 20.0], [20.3, 0.0, 20.0]);
    seeded_eye_heights(&mut mgr);
    set_body(&mut mgr, PLAYER, "BS_HumanMale.BS_HumanMale");
    set_body(&mut mgr, NPC, "BS_JaffaMale.BS_JaffaMale");
    assert_eq!(mgr.line_of_sight(NPC, PLAYER), LineOfSight::Clear);
    set_body(&mut mgr, NPC, "MOB_AMBRat.BS_MOB_Rat");
    assert_eq!(mgr.line_of_sight(NPC, PLAYER), LineOfSight::Blocked);
}
