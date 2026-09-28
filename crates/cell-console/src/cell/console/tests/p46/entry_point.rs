//! `.gotolocation <world>` and `.gotospace <spaceId>` with the coordinates
//! left off: the subject lands on the world's **entry point** — new-character
//! start, else stargate arrival, else first authored respawner
//! (`travel/world_entry_point.rs`).
//!
//! Deliberate deviation from legacy `gotoLocation` (`Player.py:344-365`),
//! which always required `x y z`; owner request 2026-09-27.

use super::*;
use crate::cell::spawner::{RespawnerDef, StargateEntry};
use cimmeria_cell_world::cell::ring_transport::RingRegion;

fn gate(world: &str, pos: [f32; 3], arrival: Option<[f32; 3]>) -> StargateEntry {
    StargateEntry {
        world_name: world.to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        yaw: 0.0,
        address_origin: 1,
        arrival: arrival.map(|a| (a, 0.0)),
        event_set_id: None,
    }
}

fn respawner(id: i32, world: &str, pos: [f32; 3]) -> RespawnerDef {
    RespawnerDef {
        respawner_id: id,
        world_name: world.to_string(),
        name: format!("respawner {id}"),
        pos,
    }
}

/// The gate a traveller arrives through wins over the world's respawners,
/// and its authored arrival pin wins over the gate prop's own transform.
/// When two gates share the world, the DHD's pick (lowest `stargate_id`) is
/// the one used.
#[tokio::test]
async fn gotolocation_world_alone_lands_on_the_stargate_arrival() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let castle = mgr.default_space_for_world(CASTLE).unwrap();
    mgr.stargates.insert(
        9,
        gate(CASTLE, [900.0, 0.0, 900.0], Some([901.0, 1.0, 902.0])),
    );
    mgr.stargates.insert(
        4,
        gate(CASTLE, [400.0, 0.0, 400.0], Some([401.0, 1.0, 402.0])),
    );
    mgr.respawners.push(respawner(1, CASTLE, [10.0, 0.0, 10.0]));

    // Lower-case on purpose: the world name is matched the way the
    // coordinate form matches it.
    let t = run("gotolocation", gm, &["castle"], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(gm, CASTLE.to_string(), Some(castle), [401.0, 1.0, 402.0]),
        "the lowest-id gate's arrival pin is the entry point"
    );
    assert!(
        t.mentions("[stargate arrival]"),
        "the feedback must say which rule picked the spot; got {:?}",
        t.feedback
    );
}

/// A gateless world falls back to its lowest-id *authored* respawner; the
/// `(0,0,0)` placeholder rows are never an entry point, even with a lower id.
#[tokio::test]
async fn gotolocation_world_alone_falls_back_to_the_first_authored_respawner() {
    let (mut mgr, gm, _npc) = setup_worlds();
    mgr.respawners.push(respawner(7, CASTLE, [70.0, 1.0, 70.0]));
    mgr.respawners.push(respawner(1, CASTLE, [0.0, 0.0, 0.0]));
    mgr.respawners.push(respawner(3, CASTLE, [30.0, 1.0, 30.0]));

    let t = run("gotolocation", gm, &[CASTLE], None, &mut mgr).await;

    assert_eq!(t.only_gate_travel().3, [30.0, 1.0, 30.0]);
    assert!(t.mentions("[respawner]"), "got {:?}", t.feedback);
}

/// A character-creation starting world lands where new characters begin,
/// ahead of any respawner registered for it.
#[tokio::test]
async fn gotolocation_starting_world_alone_lands_on_the_new_character_start() {
    let (mut mgr, gm, _npc) = setup_worlds();
    mgr.respawners
        .push(respawner(5, INSTANCED, [5.0, 5.0, 5.0]));

    let t = run("gotolocation", gm, &[INSTANCED], None, &mut mgr).await;

    let travel = t.only_gate_travel();
    assert_eq!(travel.1, INSTANCED);
    assert_eq!(travel.3, [-334.231, 73.472, -228.026]);
    assert!(t.mentions("[new-character start]"), "got {:?}", t.feedback);
}

/// Naming your own world alone is still the cheap in-place snap.
#[tokio::test]
async fn gotolocation_own_world_alone_snaps_to_the_entry_point() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let agnos = mgr.get_entity_space_id(gm).unwrap();
    let before = position_of(&mgr, gm);
    mgr.respawners.push(respawner(2, AGNOS, [12.0, 0.0, 34.0]));

    let t = run("gotolocation", gm, &[AGNOS], None, &mut mgr).await;

    assert!(t.gate_travels.is_empty(), "{:?}", t.gate_travels);
    assert_eq!(t.teleports, vec![(gm, agnos, [12.0, 0.0, 34.0], before)]);
}

/// A world with nothing to land on is refused with a line that says to give
/// coordinates — never a silent move to the origin.
#[tokio::test]
async fn gotolocation_world_alone_without_an_entry_point_is_refused() {
    let (mut mgr, gm, _npc) = setup_worlds();
    // An unauthored placeholder row is not an entry point.
    mgr.respawners.push(respawner(1, CASTLE, [0.0, 0.0, 0.0]));

    let t = run("gotolocation", gm, &[CASTLE], None, &mut mgr).await;

    assert_no_move(&t);
    assert!(
        t.mentions("Castle has no known entry point") && t.mentions("give coordinates"),
        "got {:?}",
        t.feedback
    );
}

/// An unknown world keeps legacy's wording on the short form too.
#[tokio::test]
async fn gotolocation_unknown_world_alone_reports_legacy_wording() {
    let (mut mgr, gm, _npc) = setup_worlds();

    let t = run("gotolocation", gm, &[UNKNOWN_WORLD], None, &mut mgr).await;

    assert_no_move(&t);
    assert!(
        t.has_line(&format!("Unable to find world: {UNKNOWN_WORLD}")),
        "got {:?}",
        t.feedback
    );
}

/// One or two coordinates is a typo, not a request for the entry point.
#[tokio::test]
async fn gotolocation_partial_coordinates_are_refused() {
    for args in [&[CASTLE, "1"][..], &[CASTLE, "1", "2"][..]] {
        let (mut mgr, gm, _npc) = setup_worlds();
        mgr.respawners.push(respawner(3, CASTLE, [30.0, 1.0, 30.0]));

        let t = run("gotolocation", gm, args, None, &mut mgr).await;

        assert_no_move(&t);
        assert!(t.mentions("expected <world>"), "{args:?}: {:?}", t.feedback);
    }
}

/// `.gotospace <spaceId>` alone: the world's entry point, inside that exact
/// instance rather than the default one.
#[tokio::test]
async fn gotospace_alone_lands_on_the_entry_point_in_that_instance() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [1.0, 0.0, 1.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [2.0, 0.0, 2.0], "Bob");
    let target_instance = instance_a.max(instance_b);

    let t = run(
        "gotospace",
        gm,
        &[&target_instance.to_string()],
        None,
        &mut mgr,
    )
    .await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            gm,
            INSTANCED.to_string(),
            Some(target_instance),
            [-334.231, 73.472, -228.026]
        ),
    );
    assert!(t.mentions("[new-character start]"), "got {:?}", t.feedback);
}

fn castle_story_pad() -> RingRegion {
    RingRegion {
        region_id: 34,
        world_id: 8,
        world_name: CASTLE.to_string(),
        x: 466.365,
        y: 70.397,
        z: 991.466,
        tag: "Castle_ArmoryRingDropZone".to_string(),
        height: 1.77,
        radius: 3.53,
        event_set_id: 10000,
        display_name_id: 7508,
        destination_ids: Vec::new(),
        point_set_id: 2081,
        required_mission_id: None,
    }
}

/// Castle's story arrival is the ring pad mission 688's ceremony lands on,
/// ahead of Castle's gate. Reverting the pad rule lands on the gate pin.
#[tokio::test]
async fn gotolocation_castle_alone_lands_on_the_story_ring_pad() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let castle = mgr.default_space_for_world(CASTLE).unwrap();
    mgr.ring_regions.insert(34, castle_story_pad());
    mgr.stargates.insert(
        4,
        gate(CASTLE, [400.0, 0.0, 400.0], Some([401.0, 1.0, 402.0])),
    );

    let t = run("gotolocation", gm, &[CASTLE], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            gm,
            CASTLE.to_string(),
            Some(castle),
            [466.365, 70.397, 991.466]
        ),
    );
    assert!(t.mentions("[story ring pad]"), "got {:?}", t.feedback);
}

/// The pad is found by tag *on that world*: a same-tag pad seeded on another
/// world is not Castle's arrival, and the gate rule takes over.
#[tokio::test]
async fn gotolocation_story_pad_on_another_world_is_ignored() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let mut pad = castle_story_pad();
    pad.world_name = AGNOS.to_string();
    mgr.ring_regions.insert(34, pad);
    mgr.stargates.insert(
        4,
        gate(CASTLE, [400.0, 0.0, 400.0], Some([401.0, 1.0, 402.0])),
    );

    let t = run("gotolocation", gm, &[CASTLE], None, &mut mgr).await;

    assert_eq!(t.only_gate_travel().3, [401.0, 1.0, 402.0]);
    assert!(t.mentions("[stargate arrival]"), "got {:?}", t.feedback);
}

/// A gate row still at the origin (Agnos's shipped that way) is not an
/// arrival: the rule falls through to the respawner instead of landing the
/// subject at (0,0,0). Reverting the filter lands on the origin.
#[tokio::test]
async fn gotolocation_skips_a_gate_left_at_the_origin() {
    let (mut mgr, gm, _npc) = setup_worlds();
    mgr.stargates.insert(4, gate(CASTLE, [0.0; 3], None));
    mgr.respawners.push(respawner(3, CASTLE, [30.0, 1.0, 30.0]));

    let t = run("gotolocation", gm, &[CASTLE], None, &mut mgr).await;

    assert_eq!(t.only_gate_travel().3, [30.0, 1.0, 30.0]);
    assert!(t.mentions("[respawner]"), "got {:?}", t.feedback);
}
