//! The System Lords' summit (DA-09, spawns 13850-13869) on the real
//! `ihpet_crater_light.nav` and `.occ`, read from the seed files: where the
//! lords stand, that a listener there is safe, that nobody at the services
//! plaza or the arrival hears them, and that every chatter line names a lord
//! who is there to speak it.

use std::collections::HashSet;

use cimmeria_common::Vector3;

use super::*;
use crate::cell::combat;

/// DA-09's spawn block.
const DA09_SPAWNS: std::ops::RangeInclusive<i32> = 13850..=13869;
/// The circle's centre (`spawnlist_debug_area_lords.sql`).
const CENTRE: Vector3 = Vector3 {
    x: 282.0,
    y: 6.9,
    z: -944.0,
};
/// Where `.gotolocation DebugArea 282 7.2 -953` puts a tester: behind Ba'al,
/// facing Ra (docs/content/debug-area.md#system-lords-summit).
const LANDING: Vector3 = Vector3 {
    x: 282.0,
    y: 7.2,
    z: -953.0,
};
/// The services plaza's centre (`spawnlist_debug_area_plaza.sql`).
const PLAZA: Vector3 = Vector3 {
    x: 252.0,
    y: 7.0,
    z: -923.0,
};
/// How far a lord may sit from the navmesh surface under it, and from the
/// occluder's terrain top (the y was taken from it): the DA-03 limits.
const MAX_NAV_GAP: f32 = 0.6;
const MAX_TERRAIN_GAP: f32 = 0.15;
/// Room to spare between a hostile's aggro radius and the ground a listener
/// stands on.
const MARGIN: f32 = 5.0;

fn lords() -> Vec<SpawnRecord> {
    let rows: Vec<SpawnRecord> = world_records()
        .into_iter()
        .filter(|r| DA09_SPAWNS.contains(&r.spawn_id))
        .collect();
    assert_eq!(rows.len(), 7, "DA-09 seeds six lords and Ra's Jaffa");
    rows
}

/// The summit group's `hear_radius` and the speaker tag of every line, from
/// `ambient_chatter_lords.sql`.
fn chatter() -> (f32, Vec<String>) {
    let files = seed_files("db/resources/Dialogs/Seed", "ambient_chatter");
    let groups = seed_rows(&files, "ambient_chatter_groups");
    let summit = groups
        .iter()
        .find(|g| g["world_id"] == WORLD_ID)
        .expect("a world-1300 chatter group");
    let hear: f32 = summit["hear_radius"].parse().expect("numeric hear_radius");
    let tags = seed_rows(&files, "ambient_chatter_lines")
        .into_iter()
        .filter(|l| l["group_id"] == summit["group_id"])
        .map(|l| unquote(&l["speaker_tag"]))
        .collect();
    (hear, tags)
}

/// Every lord stands on the navmesh and on the occluder's terrain, so the
/// client draws them on the terrace floor, not in it or above it. Revert
/// proof: raise Ra 1 m and this names him.
#[test]
fn every_lord_stands_on_mesh_and_terrain() {
    let Some(mesh) = navmesh() else { return };
    let Some(occ) = occluder() else { return };
    let mut bad = Vec::new();
    for r in &lords() {
        let p = pos(r);
        let tag = r.tag.as_deref().unwrap_or("?");
        if !mesh.is_point_valid(&p) || mesh.start_poly_snap(&p).is_none() {
            bad.push(format!("{tag} {p:?}: off the navmesh"));
            continue;
        }
        match mesh.get_height_near(p.x, p.y, p.z) {
            Some(h) if (h - p.y).abs() <= MAX_NAV_GAP => {}
            other => bad.push(format!("{tag} {p:?}: navmesh surface at {other:?}")),
        }
        if !occ
            .column(p.x, p.z)
            .iter()
            .any(|(_, _, top)| (top - p.y).abs() <= MAX_TERRAIN_GAP)
        {
            bad.push(format!("{tag} {p:?}: no occluder surface under it"));
        }
        if !r.is_stationary {
            bad.push(format!("{tag}: not stationary, so it would wander off"));
        }
    }
    assert!(bad.is_empty(), "misplaced lords: {bad:#?}");
}

/// The lords are harmless and safe to stand among: none is hostile to
/// players, and no hostile world-1300 NPC can aggro on a player anywhere a
/// line can be heard (within `hear_radius` of a lord). Revert proof: move
/// the circle to the gallery terrace (z -600) and the gallery rows name
/// themselves.
#[test]
fn a_listener_at_the_summit_is_out_of_every_hostiles_reach() {
    let records = world_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    let (hear, _) = chatter();
    let lords = lords();
    let mut bad = Vec::new();
    for l in &lords {
        let e = mgr.get_entity(eid(l.spawn_id)).unwrap();
        if combat::is_hostile_to_players(e) || e.faction != 1 {
            bad.push(format!("{:?} is not a friendly (faction 1) NPC", l.tag));
        }
    }
    for h in &records {
        let he = mgr.get_entity(eid(h.spawn_id)).unwrap();
        if !combat::is_hostile_to_players(he) {
            continue;
        }
        let need = combat::aggro_radius(he) + hear + MARGIN;
        for l in &lords {
            let gap = xz(pos(h), pos(l));
            if gap <= need {
                bad.push(format!(
                    "{:?} is {gap:.0} m from {:?}, inside {need:.0} m",
                    h.tag, l.tag
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Every chatter line names a lord seeded here, and a player at the centre
/// of the circle hears every lord. Revert proof: rename a tag in either
/// file, or widen the circle past the hear radius, and this fails.
#[test]
fn every_chatter_speaker_is_a_lord_in_earshot_of_the_centre() {
    let (hear, speaker_tags) = chatter();
    let lords = lords();
    let seated: HashSet<&str> = lords.iter().filter_map(|r| r.tag.as_deref()).collect();
    let missing: Vec<&String> = speaker_tags
        .iter()
        .filter(|t| !seated.contains(t.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "lines spoken by nobody seated: {missing:?}"
    );
    let spoken: HashSet<&str> = speaker_tags.iter().map(String::as_str).collect();
    assert_eq!(
        spoken, seated,
        "every seated lord has a line, and only they do"
    );
    for l in &lords {
        let d = pos(l).distance_to(&CENTRE);
        assert!(
            d + MARGIN <= hear,
            "{:?} is {d:.1} m from the centre: a listener there would miss its lines",
            l.tag
        );
    }
}

/// Nobody at the services plaza or arriving at a world-1300 respawner hears
/// the summit: every lord is more than `hear_radius` from the plaza's centre
/// and its NPC ring (10.5 m round it), and from each respawner. Revert proof:
/// move the circle to the plaza's east edge (265, -923), or raise the
/// group's hear radius to 30, and this names the lord.
#[test]
fn the_plaza_and_the_arrival_do_not_hear_the_summit() {
    let (hear, _) = chatter();
    let mut bad = Vec::new();
    for l in &lords() {
        let p = pos(l);
        let plaza_edge = xz(p, PLAZA) - 10.5;
        if plaza_edge <= hear {
            bad.push(format!(
                "{:?} is {plaza_edge:.1} m from the plaza ring",
                l.tag
            ));
        }
        for (name, at) in world_respawners() {
            let d = xz(p, at);
            if d <= hear {
                bad.push(format!("{:?} is {d:.1} m from respawner {name}", l.tag));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "the summit spills into chat elsewhere: {bad:#?}"
    );
}

/// A tester at the documented landing spot hears every lord, Ra's Jaffa 16.5
/// m away included. The lab check found the Jaffa's "Indeed." going to nobody
/// at a 15 m hear radius. Revert proof: set the group's hear radius back to
/// 15 and this names Ra's Jaffa.
#[test]
fn the_landing_spot_hears_every_lord() {
    let (hear, _) = chatter();
    let deaf: Vec<String> = lords()
        .iter()
        .filter(|l| pos(l).distance_to(&LANDING) > hear)
        .map(|l| format!("{:?} at {:.1} m", l.tag, pos(l).distance_to(&LANDING)))
        .collect();
    assert!(
        deaf.is_empty(),
        "out of earshot of the landing spot: {deaf:?}"
    );
}
