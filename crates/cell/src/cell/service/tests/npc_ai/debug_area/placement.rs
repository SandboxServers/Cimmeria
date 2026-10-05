//! Where the DA-03 rows stand, checked against `ihpet_crater_light.nav` and
//! `.occ`.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::LineOfSight;

use super::*;

/// How far a spawn may sit from the navmesh surface under it. Spawns were
/// authored at the occluder's terrain top where the two agree within 0.6 m,
/// against a disagreement of up to 3.3 m elsewhere on open ground.
const MAX_NAV_GAP: f32 = 0.6;

/// The named exceptions to [`MAX_NAV_GAP`]: the wanderer's spot has no point
/// within 0.6 m in reach of the plan's position, and it walks its disc anyway.
const NAV_GAP_EXCEPTIONS: [(&str, f32); 1] = [("DebugArea_Slope_Wander", 1.0)];

/// How far a spawn may sit from the top of a solid span of the occluder
/// column under it: the y was taken from that top.
const MAX_TERRAIN_GAP: f32 = 0.15;

/// Every DA-03 spawn is on the mesh (`is_point_valid` and the start snap
/// `find_path` uses), within [`MAX_NAV_GAP`] of the walkable surface, and on
/// the occluder's terrain, so the client draws it on the ground the player
/// sees and the server paths it from where it stands. Revert proof: lower a
/// pen NPC 2 m under the terrain and this names it.
#[test]
fn every_da03_spawn_stands_on_mesh_and_terrain() {
    let Some(mesh) = navmesh() else { return };
    let Some(occ) = occluder() else { return };
    let records = da03_records();
    let mut bad = Vec::new();
    for r in &records {
        let p = pos(r);
        let tag = r.tag.as_deref().unwrap_or("?");
        if !mesh.is_point_valid(&p) || mesh.start_poly_snap(&p).is_none() {
            bad.push(format!("{tag} {p:?}: off the navmesh"));
            continue;
        }
        let max_gap = NAV_GAP_EXCEPTIONS
            .iter()
            .find(|(t, _)| *t == tag)
            .map_or(MAX_NAV_GAP, |(_, g)| *g);
        match mesh.get_height_near(p.x, p.y, p.z) {
            Some(h) if (h - p.y).abs() <= max_gap => {}
            other => bad.push(format!("{tag} {p:?}: navmesh surface at {other:?}")),
        }
        let on_terrain = occ
            .column(p.x, p.z)
            .iter()
            .any(|(_, _, top)| (top - p.y).abs() <= MAX_TERRAIN_GAP);
        if !on_terrain {
            bad.push(format!(
                "{tag} {p:?}: no occluder surface within {MAX_TERRAIN_GAP} m: {:?}",
                occ.column(p.x, p.z)
            ));
        }
    }
    assert!(bad.is_empty(), "misplaced DA-03 spawns: {bad:#?}");
}

/// The patroller's two-point route routes both ways and arrives: a partial
/// route would slide the NPC across whatever lies between.
#[test]
fn the_slope_patrol_route_is_walkable() {
    let Some(mesh) = navmesh() else { return };
    let records = da03_records();
    let patrol = by_tag(&records, "DebugArea_Slope_Patrol");
    assert_eq!(patrol.patrol_path.len(), 2, "the A <-> B point set");
    assert!(
        xz(pos(patrol), patrol.patrol_path[0]) < 0.5,
        "the patroller spawns on waypoint A"
    );
    let (a, b) = (patrol.patrol_path[0], patrol.patrol_path[1]);
    assert!(xz(a, b) > 100.0, "the route spans the slope");
    for (from, to) in [(a, b), (b, a)] {
        let outcome = mesh.find_path(&from, &to);
        let status = outcome.status.label();
        let end = outcome
            .into_waypoints()
            .and_then(|w| w.last().map(|e| e.distance_to(&to)));
        assert!(
            status == "ok" && end.is_some_and(|d| d < 2.0),
            "{from:?} -> {to:?}: {status}, ends {end:?} short"
        );
    }
}

/// The wanderer's disc is walkable: eight points on its rim, each reached
/// from the spawn. A wander target off the mesh is dropped, so a disc that
/// is mostly off the mesh would look like an NPC that never wanders.
#[test]
fn the_wanderer_disc_is_walkable() {
    let Some(mesh) = navmesh() else { return };
    let records = da03_records();
    let w = by_tag(&records, "DebugArea_Slope_Wander");
    assert!(w.wander_radius > 0.0 && w.patrol_path.is_empty());
    let c = pos(w);
    let mut bad = Vec::new();
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4;
        let (x, z) = (
            c.x + w.wander_radius * a.cos(),
            c.z + w.wander_radius * a.sin(),
        );
        let Some(y) = mesh.get_height_near(x, c.y, z) else {
            bad.push(format!(
                "rim point {i} ({x}, {z}) has no surface near y {}",
                c.y
            ));
            continue;
        };
        let to = Vector3::new(x, y, z);
        let outcome = mesh.find_path(&c, &to);
        let status = outcome.status.label();
        let end = outcome
            .into_waypoints()
            .and_then(|w| w.last().map(|e| e.distance_to(&to)));
        if status != "ok" || !end.is_some_and(|d| d < 2.0) {
            bad.push(format!(
                "rim point {i} {to:?}: {status}, ends {end:?} short"
            ));
        }
    }
    assert!(bad.is_empty(), "wander disc: {bad:#?}");
}

/// The leash station is kited east, downhill: the route 25 u east of the
/// spawn exists, so the NPC can be drawn past its 15 u leash plus the 5 u
/// band (`leash/policy.rs`) by walking, not only by teleporting.
#[test]
fn the_leash_kite_route_reaches_past_the_leash() {
    let Some(mesh) = navmesh() else { return };
    let records = da03_records();
    let l = by_tag(&records, "DebugArea_Slope_Leash");
    let leash = l.leash_distance.expect("the leash template sets a radius");
    let from = pos(l);
    let (x, z) = (from.x + leash + 10.0, from.z);
    let y = mesh.get_height_near(x, from.y, z).expect("surface east");
    let to = Vector3::new(x, y, z);
    let outcome = mesh.find_path(&from, &to);
    let status = outcome.status.label();
    let end = outcome
        .into_waypoints()
        .and_then(|w| w.last().map(|e| e.distance_to(&to)));
    assert!(
        status == "ok" && end.is_some_and(|d| d < 2.0),
        "{from:?} -> {to:?}: {status}, ends {end:?} short"
    );
}

/// Assist needs line of sight (occluder, eye to eye). A and B see each other;
/// so do A and C, which keeps the C test about the radius, not a wall.
#[test]
fn the_assist_trio_sees_itself() {
    let records = da03_records();
    let trio = tagged(&records, "DebugArea_Slope_Assist_");
    assert_eq!(trio.len(), 3);
    let Some(mgr) = scene(&records, pos(trio[0])) else {
        return;
    };
    for a in &trio {
        for b in &trio {
            if a.spawn_id != b.spawn_id {
                assert_eq!(
                    mgr.line_of_sight(eid(a.spawn_id), eid(b.spawn_id)),
                    LineOfSight::Clear,
                    "{:?} -> {:?}",
                    a.tag,
                    b.tag
                );
            }
        }
    }
}
