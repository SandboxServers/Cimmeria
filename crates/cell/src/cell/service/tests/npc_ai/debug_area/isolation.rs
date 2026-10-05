//! Who engages whom in the DA-03 stations, through the production aggro and
//! assist gates on the real mesh and occluder.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

use super::*;
use crate::cell::combat::{self, generate_threat, AggroCause};

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
    assert_eq!(gallery.len(), 99, "gallery rows");
    if navmesh().is_none() {
        return;
    }
    // A stop at least every 10 u along each row (a new one whenever the next
    // NPC is more than 10 u on), so every NPC is within 10 u of a stop, well
    // inside the 18 u default aggro radius.
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

/// Shooting any gallery NPC with a single-target ability makes it fight
/// back, and pulls none of its neighbours 4 u away: they are NEUTRAL, which
/// the assist gate refuses (`not_hostile`). (An area ability damages, and so
/// wakes, every NPC it hits; that is not assist.) Revert proof: drop the
/// override and the neighbours join.
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
    let mut engaged = engaged_with_player_at(&yard, player).await.unwrap();
    engaged.sort();
    assert_eq!(
        engaged,
        [
            "DebugArea_Yard_Hostile_1",
            "DebugArea_Yard_Hostile_2",
            "DebugArea_Yard_Hostile_3"
        ],
        "all three pen NPCs (12-16 u) engage, and nothing else"
    );
}

/// Shooting a pinned neutral Jaffa (the yard's damageable neutral) wakes
/// that Jaffa and nobody else: the pen's NID Guard (template 24, assist
/// radius 26 u) stands at the back of the pen, 28 u away. The pinned Jaffa is
/// also lifted to the guard's height, so the 4 u vertical band is not what
/// keeps the guard out (review of #1222: with the guard at the front of the
/// pen, 24.1 u away, only 5 cm of band held). Revert proof: swap templates 24
/// and 35 back in the pen and the guard joins.
#[tokio::test]
async fn shooting_a_pinned_jaffa_wakes_only_that_jaffa() {
    let records = da03_records();
    let yard: Vec<SpawnRecord> = tagged(&records, "DebugArea_Yard_")
        .into_iter()
        .cloned()
        .collect();
    let pinned = tagged(&records, "DebugArea_Yard_NeutralPinned_");
    assert_eq!(pinned.len(), 2);
    for victim in pinned {
        let Some(mut mgr) = scene(&yard, standing(404.0, -6.0, -800.0)) else {
            return;
        };
        let guard_y = mgr
            .get_entity(eid(by_tag(&records, "DebugArea_Yard_Hostile_2").spawn_id))
            .unwrap()
            .position
            .y;
        mgr.get_entity_mut(eid(victim.spawn_id)).unwrap().position.y = guard_y;
        let _ = generate_threat(
            &mut mgr,
            PLAYER,
            eid(victim.spawn_id),
            10.0,
            AggroCause::Damage,
        );
        assert!(fighting(&mgr, victim), "{:?} fights back", victim.tag);
        let woken: Vec<&str> = yard
            .iter()
            .filter(|r| r.spawn_id != victim.spawn_id)
            .filter(|r| {
                !mgr.get_entity(eid(r.spawn_id))
                    .unwrap()
                    .threat_list
                    .is_empty()
            })
            .filter_map(|r| r.tag.as_deref())
            .collect();
        assert!(woken.is_empty(), "shooting {:?} woke {woken:?}", victim.tag);
    }
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
