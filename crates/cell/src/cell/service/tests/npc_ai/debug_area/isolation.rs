//! Who engages whom in the DA-03 stations, through the production aggro and
//! assist gates on the real mesh and occluder.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

use super::*;
use crate::cell::combat::{
    self, generate_threat, AggroCause, DEFAULT_AGGRO_RADIUS, DEFAULT_ASSIST_RADIUS,
};

/// Run every DA-03 NPC's Idle aggro scan once with the player at `player`,
/// and return the tags of the NPCs that engaged.
async fn engaged_with_player_at(records: &[SpawnRecord], player: Vector3) -> Option<Vec<String>> {
    let mut mgr = scene(records, player)?;
    let (tx, _rx) = mpsc::channel(1024);
    let mut out = Vec::new();
    for r in records {
        let id = eid(r.spawn_id);
        if crate::cell::service::npc_ai::npc_idle_aggro_scan_for_test(id, &tx, &mut mgr).await {
            out.push(r.tag.clone().unwrap_or_default());
        }
    }
    Some(out)
}

/// The player standing at `(x, z)` on the navmesh surface near `y`.
fn standing(x: f32, y: f32, z: f32) -> Vector3 {
    let mesh = navmesh().expect("checked by the caller");
    let h = mesh.get_height_near(x, y, z).unwrap_or(y);
    Vector3::new(x, h, z)
}

fn fighting(mgr: &SpaceManager, r: &SpawnRecord) -> bool {
    let e = mgr.get_entity(eid(r.spawn_id)).unwrap();
    e.ai_state() == AiState::Fighting && e.threat_list.contains_key(&PLAYER)
}

/// D-DA9: every gallery NPC is passive, so standing right next to it (2 u)
/// pulls nothing, and none of them looks for NPC targets either. Revert
/// proof: drop `aggression_override` from the gallery rows and every one of
/// them engages.
#[tokio::test]
async fn walking_the_gallery_pulls_nothing() {
    let records = da03_records();
    let gallery: Vec<SpawnRecord> = tagged(&records, "DebugArea_Gallery_")
        .into_iter()
        .cloned()
        .collect();
    assert!(gallery.len() >= 99, "gallery rows: {}", gallery.len());
    if navmesh().is_none() {
        return;
    }
    // One stop per 30 u of terrace covers every NPC within 18 u.
    let mut xs: Vec<(f32, f32)> = gallery.iter().map(|r| (r.x, r.z)).collect();
    xs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut stops = Vec::new();
    for (x, z) in xs {
        if stops
            .last()
            .is_none_or(|&(sx, sz): &(f32, f32)| x - sx > 10.0 || z != sz)
        {
            stops.push((x, z));
        }
    }
    for (x, z) in stops {
        let player = standing(x + 2.0, 23.06, z);
        let Some(engaged) = engaged_with_player_at(&gallery, player).await else {
            return;
        };
        assert!(
            engaged.is_empty(),
            "player at {player:?} pulled gallery NPCs {engaged:?}"
        );
    }
}

/// Shooting any gallery NPC makes it fight back, and pulls none of its
/// neighbours 4 u away: they are NEUTRAL, which the assist gate refuses
/// (`not_hostile`). Revert proof: drop the override and the neighbours join.
#[tokio::test]
async fn shooting_a_gallery_npc_rallies_no_neighbour() {
    let records = da03_records();
    let gallery: Vec<SpawnRecord> = tagged(&records, "DebugArea_Gallery_")
        .into_iter()
        .cloned()
        .collect();
    let Some(mut mgr) = scene(&gallery, standing(110.0, 23.06, -600.0)) else {
        return;
    };
    for victim in &gallery {
        let _ = generate_threat(
            &mut mgr,
            PLAYER,
            eid(victim.spawn_id),
            10.0,
            AggroCause::Damage,
        );
        assert!(fighting(&mgr, victim), "{:?} fights back", victim.tag);
    }
    // Every gallery NPC was shot exactly once, so each holds exactly the
    // threat of its own shot: an assist seed on top would show.
    for r in &gallery {
        let threat = mgr.get_entity(eid(r.spawn_id)).unwrap().threat_list[&PLAYER];
        assert_eq!(threat, 10.0, "{:?} was also recruited", r.tag);
    }
}

/// The faction yard: standing among the friendly row or the neutral row
/// engages nobody (the pen is 24 u and more away); walking up to the pen
/// (12 u) engages the pen and nothing else. Revert proof: put the pen at
/// x 420 and the neutral-row stop pulls it.
#[tokio::test]
async fn the_yard_pen_engages_only_a_player_who_walks_up() {
    let records = da03_records();
    let yard: Vec<SpawnRecord> = tagged(&records, "DebugArea_Yard_")
        .into_iter()
        .cloned()
        .collect();
    assert_eq!(yard.len(), 11);
    if navmesh().is_none() {
        return;
    }
    for (x, z) in [(396.0, -800.0), (412.0, -800.0), (404.0, -790.0)] {
        let player = standing(x, -6.0, z);
        let engaged = engaged_with_player_at(&yard, player).await.unwrap();
        assert!(
            engaged.is_empty(),
            "player at {player:?} pulled {engaged:?}"
        );
    }
    let player = standing(424.0, -3.0, -796.0);
    let engaged = engaged_with_player_at(&yard, player).await.unwrap();
    assert!(!engaged.is_empty(), "the pen engages at 12-16 u");
    assert!(
        engaged
            .iter()
            .all(|t| t.starts_with("DebugArea_Yard_Hostile_")),
        "only the pen engages: {engaged:?}"
    );
}

/// The friendly and neutral rows can never fight: their factions have no
/// enemy in the reaction table, and the pinned Jaffa is NEUTRAL. Only the
/// pinned row and the pen are damageable (faction 10).
#[test]
fn the_yard_rows_never_fight_and_say_why() {
    let records = da03_records();
    let Some(mgr) = scene(&records, standing(404.0, -6.0, -800.0)) else {
        return;
    };
    for r in tagged(&records, "DebugArea_Yard_") {
        let e = mgr.get_entity(eid(r.spawn_id)).unwrap();
        let tag = r.tag.as_deref().unwrap();
        let hostile = combat::is_hostile_to_players(e);
        let seeks = combat::seeks_npc_targets(e);
        let damageable = e.faction == combat::HOSTILE_FACTION;
        if tag.starts_with("DebugArea_Yard_Hostile_") {
            assert!(hostile && damageable, "{tag}");
        } else {
            assert!(!hostile && !seeks, "{tag} must never start a fight");
            assert_eq!(
                damageable,
                tag.starts_with("DebugArea_Yard_NeutralPinned_"),
                "{tag}: only the pinned Jaffa can be shot"
            );
        }
    }
}

/// The assist trio: shooting A pulls B (6 u) and not C (14 u, outside the
/// 10 u assist radius); shooting C pulls nobody. A player 12 u from the trio
/// pulls none of it by proximity (6 u aggro radius), so the test is the
/// shot, not the approach.
#[tokio::test]
async fn the_assist_trio_rallies_exactly_one_neighbour() {
    let records = da03_records();
    let trio: Vec<SpawnRecord> = tagged(&records, "DebugArea_Slope_Assist_")
        .into_iter()
        .cloned()
        .collect();
    let [a, b, c] = [
        by_tag(&records, "DebugArea_Slope_Assist_A").clone(),
        by_tag(&records, "DebugArea_Slope_Assist_B").clone(),
        by_tag(&records, "DebugArea_Slope_Assist_C").clone(),
    ];
    let shooter = standing(a.x - 12.0, a.y, a.z);
    let Some(engaged) = engaged_with_player_at(&trio, shooter).await else {
        return;
    };
    assert!(engaged.is_empty(), "approach pulled {engaged:?}");

    let mut mgr = scene(&trio, shooter).unwrap();
    let _ = generate_threat(&mut mgr, PLAYER, eid(a.spawn_id), 10.0, AggroCause::Damage);
    assert!(fighting(&mgr, &a) && fighting(&mgr, &b), "A shot: B joins");
    assert!(!fighting(&mgr, &c), "A shot: C, 14 u away, stays");

    let mut mgr = scene(&trio, shooter).unwrap();
    let _ = generate_threat(&mut mgr, PLAYER, eid(c.spawn_id), 10.0, AggroCause::Damage);
    assert!(fighting(&mgr, &c));
    assert!(
        !fighting(&mgr, &a) && !fighting(&mgr, &b),
        "C shot: nobody joins"
    );
}

/// The reach of a hostile station NPC: its spawn, or every point of its
/// patrol route, or its wander disc.
fn reach_distance(r: &SpawnRecord, p: Vector3) -> f32 {
    if r.patrol_path.len() >= 2 {
        let mut best = f32::MAX;
        for i in 0..r.patrol_path.len() {
            let (a, b) = (
                r.patrol_path[i],
                r.patrol_path[(i + 1) % r.patrol_path.len()],
            );
            best = best.min(segment_xz(a, b, p));
        }
        return best;
    }
    (xz(pos(r), p) - r.wander_radius).max(0.0)
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

/// The station a DA-03 tag belongs to: the yard pen, each Z5 station, or the
/// gallery.
fn station(tag: &str) -> &str {
    [
        "DebugArea_Yard_",
        "DebugArea_Slope_Patrol",
        "DebugArea_Slope_Wander",
        "DebugArea_Slope_Leash",
        "DebugArea_Slope_Assist_",
        "DebugArea_Gallery_",
    ]
    .into_iter()
    .find(|p| tag.starts_with(p))
    .unwrap_or(tag)
}

/// No station is in reach of another (task 4 of DA-03): for every pair of
/// NPCs in different stations where one is hostile to players, a player
/// inside the hostile one's aggro radius is outside the other's, and the
/// hostile one is outside the other's assist radius (both ways: neither can
/// be recruited when the other is shot). Patrol routes and wander discs
/// count at their nearest point. Revert proof: move the wanderer to
/// (60, -700), 20 u from the leash station, and this names the pair.
#[test]
fn no_station_reaches_another() {
    let records = da03_records();
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
            let (ht, ot) = (h.tag.as_deref().unwrap(), o.tag.as_deref().unwrap());
            if station(ht) == station(ot) {
                continue;
            }
            let oe = mgr.get_entity(eid(o.spawn_id)).unwrap();
            let gap = reach_distance(h, pos(o)) - o.wander_radius;
            let o_aggro = if combat::is_hostile_to_players(oe) {
                combat::aggro_radius(oe)
            } else {
                0.0
            };
            let need = (combat::aggro_radius(he) + o_aggro)
                .max(combat::assist_radius(he))
                .max(combat::assist_radius(oe));
            if gap <= need {
                bad.push(format!("{ht} reaches {ot}: {gap:.1} u <= {need:.1} u"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    // The defaults this rule was written against.
    assert_eq!(DEFAULT_AGGRO_RADIUS, 18.0);
    assert_eq!(DEFAULT_ASSIST_RADIUS, 10.0);
}

/// The other Debug Area zones (docs/analysis/debug-area/README.md, DA-02 and
/// DA-04) are out of reach of every hostile DA-03 NPC: 2x its aggro radius
/// plus 25 u for the zone itself.
#[test]
fn no_da03_hostile_reaches_another_packets_zone() {
    const ZONES: [(&str, f32, f32); 6] = [
        ("Z1 arrival", 251.0, -962.0),
        ("Z2 services plaza", 252.0, -923.0),
        ("Z3 dummies range", 252.0, -872.0),
        ("Z6 arena pit", 250.0, -725.0),
        ("Z8 cover course", 166.0, -930.0),
        ("Z9 death and respawn", 438.0, -916.0),
    ];
    let records = da03_records();
    let Some(mgr) = scene(&records, Vector3::new(0.0, 0.0, -2000.0)) else {
        return;
    };
    for r in &records {
        let e = mgr.get_entity(eid(r.spawn_id)).unwrap();
        if !combat::is_hostile_to_players(e) {
            continue;
        }
        for (zone, x, z) in ZONES {
            let gap = reach_distance(r, Vector3::new(x, 0.0, z));
            let need = 2.0 * combat::aggro_radius(e) + 25.0;
            assert!(gap > need, "{:?} is {gap:.1} u from {zone}", r.tag);
        }
    }
}
