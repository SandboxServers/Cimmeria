//! Distance guards: no DA-03 station is in reach of another station, of
//! another packet's world-1300 rows or of a respawner, and no NPC that fights
//! NPCs can reach a DA-03 NPC it would target. Every world-1300 row of every
//! seed file takes part, so a later packet that seeds an NPC too close fails
//! here.

use cimmeria_common::Vector3;

use super::*;
use crate::cell::combat::{self, DEFAULT_AGGRO_RADIUS, DEFAULT_ASSIST_RADIUS};

/// Horizontal distance from `p` to the ground `r` covers: its spawn, every
/// point of its patrol route, or its wander disc.
fn reach_distance(r: &SpawnRecord, p: Vector3) -> f32 {
    if r.patrol_path.len() >= 2 {
        let n = r.patrol_path.len();
        return (0..n)
            .map(|i| segment_xz(r.patrol_path[i], r.patrol_path[(i + 1) % n], p))
            .fold(f32::MAX, f32::min);
    }
    (xz(pos(r), p) - r.wander_radius).max(0.0)
}

/// Points along the ground `r` covers, 2 u apart on a patrol route.
fn reach_points(r: &SpawnRecord) -> Vec<Vector3> {
    if r.patrol_path.len() < 2 {
        return vec![pos(r)];
    }
    let n = r.patrol_path.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (r.patrol_path[i], r.patrol_path[(i + 1) % n]);
        let steps = (xz(a, b) / 2.0).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            out.push(Vector3::new(
                a.x + t * (b.x - a.x),
                a.y + t * (b.y - a.y),
                a.z + t * (b.z - a.z),
            ));
        }
    }
    out
}

/// The closest the ground `a` covers comes to the ground `b` covers.
fn reach_gap(a: &SpawnRecord, b: &SpawnRecord) -> f32 {
    reach_points(b)
        .into_iter()
        .map(|p| reach_distance(a, p))
        .fold(f32::MAX, f32::min)
        - if b.patrol_path.len() < 2 {
            b.wander_radius
        } else {
            0.0
        }
}

fn segment_xz(a: Vector3, b: Vector3, p: Vector3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let len2 = dx * dx + dz * dz;
    let t = if len2 > 0.0 {
        (((p.x - a.x) * dx + (p.z - a.z) * dz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a.x + t * dx - p.x).powi(2) + (a.z + t * dz - p.z).powi(2)).sqrt()
}

/// The station a row belongs to. DA-03 stations are named by tag prefix; the
/// pinned Jaffa and the pen are separate stations (review of #1222: one
/// "yard" bucket hid the pen's 26 u assist radius reaching the pinned row).
/// The pen and the assist trio are one station each because their members
/// are meant to rally each other. Another packet's row is its own station.
fn station(r: &SpawnRecord) -> String {
    let tag = r.tag.as_deref().unwrap_or("");
    if !is_da03(r) {
        return format!("other:{}", r.spawn_id);
    }
    [
        "DebugArea_Yard_Friendly_",
        "DebugArea_Yard_NeutralPinned_",
        "DebugArea_Yard_Neutral_",
        "DebugArea_Yard_Hostile_",
        "DebugArea_Slope_Patrol",
        "DebugArea_Slope_Wander",
        "DebugArea_Slope_Leash",
        "DebugArea_Slope_Assist_",
        "DebugArea_Gallery_",
    ]
    .into_iter()
    .find(|p| tag.starts_with(p))
    .unwrap_or_else(|| panic!("DA-03 spawn {} has an unknown tag {tag:?}", r.spawn_id))
    .to_string()
}

/// No station is in reach of another (task 4 of DA-03): for every pair of
/// world-1300 NPCs in different stations, at least one of them DA-03's, where
/// one is hostile to players, a player inside the hostile one's aggro radius
/// is outside the other's, and neither is inside the other's assist radius
/// (neither can be recruited when the other is shot). Patrol routes and
/// wander discs count at their nearest point. Revert proof: move the
/// wanderer to (60, -700), 20 u from the leash station, or swap the pen's
/// NID Guard back to the front (24.1 u from the pinned Jaffa, inside its
/// 26 u assist radius), and this names the pair.
#[test]
fn no_station_reaches_another() {
    let records = world_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    let mut bad = Vec::new();
    for h in &records {
        let he = mgr.get_entity(eid(h.spawn_id)).unwrap();
        if !combat::is_hostile_to_players(he) {
            continue;
        }
        for o in &records {
            if !(is_da03(h) || is_da03(o)) || station(h) == station(o) {
                continue;
            }
            let oe = mgr.get_entity(eid(o.spawn_id)).unwrap();
            let gap = reach_gap(h, o);
            let o_aggro = if combat::is_hostile_to_players(oe) {
                combat::aggro_radius(oe)
            } else {
                0.0
            };
            let need = (combat::aggro_radius(he) + o_aggro)
                .max(combat::assist_radius(he))
                .max(combat::assist_radius(oe));
            if gap <= need {
                bad.push(format!(
                    "{:?} reaches {:?}: {gap:.1} u <= {need:.1} u",
                    h.tag, o.tag
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    // The defaults this rule was written against.
    assert_eq!(DEFAULT_AGGRO_RADIUS, 18.0);
    assert_eq!(DEFAULT_ASSIST_RADIUS, 10.0);
}

/// A player arriving at a world-1300 respawner (DA-01's 130/131, read from
/// `respawners.sql`) is outside every DA-03 hostile's aggro radius with 25 u
/// to spare. Before DA-01 seeds them there is nothing to check.
#[test]
fn no_da03_hostile_reaches_a_respawner() {
    let records = da03_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    for (name, p) in world_respawners() {
        for r in &records {
            let e = mgr.get_entity(eid(r.spawn_id)).unwrap();
            if !combat::is_hostile_to_players(e) {
                continue;
            }
            let gap = reach_distance(r, p);
            let need = combat::aggro_radius(e) + 25.0;
            assert!(
                gap > need,
                "{:?} is {gap:.1} u from respawner {name}",
                r.tag
            );
        }
    }
}

/// NPC-vs-NPC (#1009): no world-1300 NPC that looks for NPC targets can find
/// a DA-03 NPC it would attack, or the other way round. The override that
/// makes the gallery and the pinned Jaffa passive narrows only what *they*
/// seek; a faction-3, -2 or -11 NPC still takes them as targets
/// (`npc_aggression_toward` reads the viewer's override). The margin is the
/// scan's own consider radius, twice the viewer's aggro radius. Another
/// packet's squads (DA-04's arena: factions 3, 27, 29) are read from the seed
/// at test time. Revert proof: seed a faction-3 NPC 30 u from the gallery and
/// this names it.
#[test]
fn no_npc_fighter_reaches_a_da03_target() {
    let records = world_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    let mut bad = Vec::new();
    for v in &records {
        let ve = mgr.get_entity(eid(v.spawn_id)).unwrap();
        if !combat::seeks_npc_targets(ve) {
            continue;
        }
        for t in &records {
            if !(is_da03(v) || is_da03(t)) {
                continue;
            }
            let te = mgr.get_entity(eid(t.spawn_id)).unwrap();
            if !combat::npc_may_target_npc(ve, te) {
                continue;
            }
            let gap = reach_gap(v, t);
            let need = 2.0 * combat::aggro_radius(ve);
            if gap <= need {
                bad.push(format!(
                    "{:?} (faction {}) can seek {:?} (faction {}): {gap:.1} u <= {need:.1} u",
                    v.tag, ve.faction, t.tag, te.faction
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The NPC-vs-NPC guard is not vacuous: the passive gallery and pinned rows
/// are still NPC targets for a faction-3 viewer, which is what it protects.
#[test]
fn passive_da03_rows_are_still_npc_targets() {
    let records = da03_records();
    let gallery = by_tag(&records, "DebugArea_Gallery_24").clone();
    let mut viewer = gallery.clone();
    viewer.spawn_id = 13599;
    viewer.tag = Some("probe".into());
    viewer.faction = Some(3);
    viewer.aggression_override = None;
    viewer.x += 40.0;
    let Some(mgr) = scene(&[gallery.clone(), viewer.clone()], pos(&gallery)) else {
        return;
    };
    let (ve, te) = (
        mgr.get_entity(eid(viewer.spawn_id)).unwrap(),
        mgr.get_entity(eid(gallery.spawn_id)).unwrap(),
    );
    assert!(combat::seeks_npc_targets(ve));
    assert!(
        combat::npc_may_target_npc(ve, te),
        "a NEUTRAL pin is still a target"
    );
}

/// The reach guards above see the other packets' rows, not only DA-03's:
/// DA-04's arena, cover-course and death-yard spawns (its block 13600-13799,
/// `spawnlist_debug_area_combat.sql`) load through the same seed scan, faction
/// 3, 27 and 29 squads included. If the scan stopped picking up another
/// packet's file, the guards would pass on DA-03 alone; this fails instead.
#[test]
fn the_reach_guards_see_da04_rows() {
    let records = world_records();
    let da04: Vec<&SpawnRecord> = records
        .iter()
        .filter(|r| (13600..=13799).contains(&r.spawn_id))
        .collect();
    // Every INSERT line of DA-04's file is one world-1300 row.
    let raw = seed("../../db/resources/Worlds/Seed/spawnlist_debug_area_combat.sql")
        .lines()
        .filter(|l| l.starts_with("INSERT INTO spawnlist"))
        .count();
    assert!(raw > 0, "DA-04's spawn file has rows");
    assert_eq!(da04.len(), raw, "every DA-04 row reaches the guards");
    for f in [3, 10, 27, 29] {
        assert!(
            da04.iter().any(|r| r.faction == Some(f)),
            "a DA-04 faction-{f} row"
        );
    }
}
