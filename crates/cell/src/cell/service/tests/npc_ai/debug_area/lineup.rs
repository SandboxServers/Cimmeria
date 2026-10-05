//! The NPC lineup (DA-10, spawns 13870-14099) on the real
//! `ihpet_crater_light.nav` and `.occ`, read from the seed files: every
//! clone stands on the ground the client draws, a player can walk up to
//! each one from the Compound ring, they stand apart, and none of them can
//! fight or be fought (`docs/content/debug-area.md#npc-lineup`).

use std::f32::consts::PI;

use cimmeria_common::Vector3;

use super::*;
use crate::cell::combat;

/// DA-10's spawn block.
const DA10_SPAWNS: std::ops::RangeInclusive<i32> = 13870..=14099;
/// The Compound ring pad (`debug_area_rings.sql`), the station a tester
/// rings to.
const RING_PAD: Vector3 = Vector3 {
    x: 224.0,
    y: 7.44,
    z: -938.0,
};
/// Where `.gotolocation DebugArea 300 6.6 -897` puts a tester: the east
/// wing's doorway (`spawnlist_debug_area_lineup.sql`).
const LANDING: Vector3 = Vector3 {
    x: 300.0,
    y: 6.6,
    z: -897.0,
};
/// How far a clone may sit from the navmesh surface under it, and from the
/// occluder's terrain top (the y was taken from it): the DA-03 limits.
const MAX_NAV_GAP: f32 = 0.6;
const MAX_TERRAIN_GAP: f32 = 0.15;
/// Where a tester stands to look at a clone: this far in front of it.
const VIEW_DISTANCE: f32 = 1.5;
/// Closest two clones may stand: a humanoid's shoulders, and a creature's
/// or machine's body (`MOB_*` body sets).
const MIN_GAP: f32 = 2.0;
const MIN_GAP_MOB: f32 = 3.5;
/// Room to spare between a hostile's aggro radius and a clone a tester
/// stands beside.
const MARGIN: f32 = 5.0;

fn lineup() -> Vec<SpawnRecord> {
    let rows: Vec<SpawnRecord> = world_records()
        .into_iter()
        .filter(|r| DA10_SPAWNS.contains(&r.spawn_id))
        .collect();
    assert!(
        rows.len() >= 161,
        "DA-10 seeds a clone per look: {}",
        rows.len()
    );
    rows
}

/// Every clone stands on the navmesh and on the occluder's terrain, so the
/// client draws it on the floor of the east wing, not in it or above it,
/// and is stationary. Revert proof: raise a clone 1 m, or move one onto a
/// wall, and this names it.
#[test]
fn every_lineup_npc_stands_on_mesh_and_terrain() {
    let Some(mesh) = navmesh() else { return };
    let Some(occ) = occluder() else { return };
    let mut bad = Vec::new();
    for r in &lineup() {
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
    assert!(bad.is_empty(), "misplaced lineup NPCs: {bad:#?}");
}

/// A tester who rings to the Compound station can walk up to every clone:
/// the spot 1.5 m in front of it (along its heading) routes from the ring
/// pad and from the documented landing spot, and the route arrives. Revert
/// proof: turn a row to face the wall behind it, or move a clone into one
/// of the closed blocks, and this names it.
#[test]
fn every_lineup_npc_can_be_walked_up_to() {
    let Some(mesh) = navmesh() else { return };
    let mut bad = Vec::new();
    for from in [RING_PAD, LANDING] {
        assert!(
            mesh.start_poly_snap(&from).is_some(),
            "{from:?} is on the navmesh"
        );
        for r in &lineup() {
            let front = Vector3::new(
                r.x + VIEW_DISTANCE * r.heading.sin(),
                r.y,
                r.z + VIEW_DISTANCE * r.heading.cos(),
            );
            let outcome = mesh.find_path(&from, &front);
            let status = outcome.status.label();
            let end = outcome
                .into_waypoints()
                .and_then(|w| w.last().map(|e| e.distance_to(&front)));
            if status != "ok" || !end.is_some_and(|d| d < 2.0) {
                bad.push(format!(
                    "{:?} from {from:?}: {status}, ends {end:?} short",
                    r.tag
                ));
            }
        }
    }
    assert!(bad.is_empty(), "clones a tester cannot reach: {bad:#?}");
}

/// The rows face their walkways (heading 0 or pi) and no two clones overlap.
/// Revert proof: move a clone onto its neighbour's spot and this names the
/// pair.
#[test]
fn lineup_npcs_stand_apart_facing_their_walkway() {
    let rows = lineup();
    let mut bad = Vec::new();
    for (i, a) in rows.iter().enumerate() {
        if a.heading.abs() > 0.001 && (a.heading - PI).abs() > 0.001 {
            bad.push(format!("{:?} faces {}, not along z", a.tag, a.heading));
        }
        for b in &rows[i + 1..] {
            let need = if a.body_set.starts_with("MOB_") || b.body_set.starts_with("MOB_") {
                MIN_GAP_MOB
            } else {
                MIN_GAP
            };
            let gap = xz(pos(a), pos(b));
            if gap < need {
                bad.push(format!("{:?} and {:?} are {gap:.2} m apart", a.tag, b.tag));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The lineup is harmless and safe to walk: every clone is a friendly
/// (faction 1) NPC that players cannot damage, that looks for no NPC
/// targets, and that no world-1300 NPC would take as a target; and no
/// hostile can aggro on a tester beside any clone. Revert proof: give a
/// clone faction 10 or 3, or move the Z9 death yard's lethal squad into the
/// east wing, and this names it.
#[test]
fn the_lineup_is_friendly_and_out_of_every_hostiles_reach() {
    let records = world_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    let player = mgr.get_entity(PLAYER).unwrap();
    let lineup = lineup();
    let mut bad = Vec::new();
    for l in &lineup {
        let e = mgr.get_entity(eid(l.spawn_id)).unwrap();
        if e.faction != 1
            || combat::is_hostile_to_players(e)
            || combat::player_may_attack_pve(player, e)
            || combat::seeks_npc_targets(e)
        {
            bad.push(format!("{:?} can fight or be fought", l.tag));
        }
        for v in &records {
            let ve = mgr.get_entity(eid(v.spawn_id)).unwrap();
            if combat::npc_may_target_npc(ve, e) {
                bad.push(format!("{:?} would take {:?} as a target", v.tag, l.tag));
            }
        }
    }
    for h in &records {
        let he = mgr.get_entity(eid(h.spawn_id)).unwrap();
        if !combat::is_hostile_to_players(he) {
            continue;
        }
        let need = combat::aggro_radius(he) + MARGIN;
        for l in &lineup {
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
