//! Z8, the cover course in the south compound's west wing, on the real mesh,
//! occluder and world-1300 cover (NA22 hold and seek, NA23 peek, the
//! occluder line of sight).

use super::*;
use crate::cell::cover::{horizontal, is_flanked, COVER_ARRIVE_RADIUS, MAX_COVER_DISTANCE};
use cimmeria_entity::stats::HEALTH;

const RIFLEMAN1: &str = "DebugArea_Cover_Rifleman1";
const RIFLEMAN2: &str = "DebugArea_Cover_Rifleman2";
const RIFLEMAN3: &str = "DebugArea_Cover_Rifleman3";
/// The Mid/Better marker rifleman 3 is authored behind, on the hall's south
/// wall, facing north into the hall.
const RIFLEMAN3_SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 130_000_029,
    node_id: 1,
};

fn one<'a>(records: &'a [SpawnRecord], tag: &str) -> &'a SpawnRecord {
    let rows = da04(records, tag);
    assert_eq!(rows.len(), 1, "one {tag} row");
    rows[0]
}

/// Rifleman 3 spawns holding the marker it is authored behind (NA22), so a
/// tester sees an NPC already in cover; riflemen 1 and 2 stand in the open and
/// must seek a slot. Fails if the row or the marker moves, or `use_cover` is
/// dropped from template 1375.
#[tokio::test]
async fn rifleman3_spawns_in_cover_and_the_others_in_the_open() {
    let records = world_records();
    let Some(mgr) = scene(&records) else {
        return;
    };
    assert_eq!(
        held(&mgr, npc_id(one(&records, RIFLEMAN3))),
        Some(RIFLEMAN3_SLOT)
    );
    for tag in [RIFLEMAN1, RIFLEMAN2] {
        assert_eq!(
            held(&mgr, npc_id(one(&records, tag))),
            None,
            "{tag} starts in the open"
        );
    }
}

/// The occluder gates the wing's rooms: rifleman 2 sees a player in its own
/// doorway and engages, but not one 24 u away behind the x 151 wall, inside
/// its 25 u aggro radius.
#[tokio::test]
async fn rifleman2_sees_its_doorway_but_not_through_the_wall() {
    let records = world_records();
    let r2 = one(&records, RIFLEMAN2);
    for (spot, sees) in [([156.0, 6.7, -962.0], true), ([165.0, 6.7, -950.0], false)] {
        let Some(mut mgr) = scene(&records) else {
            return;
        };
        let d = (spot[0] - r2.x).hypot(spot[2] - r2.z);
        assert!(d < 25.0, "fixture: {spot:?} inside the radius ({d})");
        add_player(&mut mgr, PLAYER, spot);
        let (_, targets) = scan(&mut mgr, npc_id(r2)).await;
        assert_eq!(
            targets.contains(&PLAYER),
            sees,
            "rifleman 2 vs a player at {spot:?}"
        );
    }
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
}

/// Put `npc` into a fight with a player at `spot`.
fn engage(mgr: &mut SpaceManager, npc: u32, spot: [f32; 3]) {
    add_player(mgr, PLAYER, spot);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 100_000, 100_000);
    let e = mgr.get_entity_mut(npc).unwrap();
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    e.threat_list.insert(PLAYER, 10.0);
}

/// **Content guard.** A rifleman in the open under fire from the course walks
/// over the navmesh to a free slot that faces the tester and stops there with
/// zero velocity and Cover Stance. Fails if the wing has no usable cover for
/// it (a re-extract that drops world 1300's markers, a moved row).
#[tokio::test]
async fn riflemen_in_the_open_take_cover_facing_the_tester() {
    let records = world_records();
    let nodes = cover_nodes();
    let r3 = npc_id(one(&records, RIFLEMAN3));
    for (tag, spot) in [
        (RIFLEMAN1, [186.0, 6.7, -935.0]),
        (RIFLEMAN2, [156.0, 6.7, -962.0]),
    ] {
        let Some(mut mgr) = scene(&records) else {
            return;
        };
        // Rifleman 3 is dead: its marker is free for anyone to take.
        mgr.cover
            .reservations
            .lock()
            .unwrap()
            .release_for_entity(cimmeria_common::EntityId(r3 as i32));
        {
            let e = mgr.get_entity_mut(r3).unwrap();
            e.stats.get_mut(HEALTH).unwrap().update(0, 0, 450);
            e.set_state_flag(crate::cell::combat::BSF_DEAD);
            crate::cell::service::npc_ai::force_ai_state(e, AiState::Dead);
        }
        let npc = npc_id(one(&records, tag));
        engage(&mut mgr, npc, spot);
        let player = Vector3::new(spot[0], spot[1], spot[2]);
        let mut arrived = None;
        for tick in 0..400 {
            if tick % 10 == 0 {
                ai_tick(&mut mgr).await;
            }
            crate::cell::service::ticks::npc_movement_tick(&mut mgr);
            let Some(slot) = held(&mgr, npc) else {
                continue;
            };
            let node = nodes.iter().find(|n| n.key() == slot).unwrap().clone();
            let e = mgr.get_entity(npc).unwrap();
            if e.nav_path.is_empty()
                && e.velocity == [0.0; 3]
                && horizontal(&e.position, &node.pos) <= COVER_ARRIVE_RADIUS
                && super::super::super::npc_ai_cover_behaviour::cover_defense(&mgr, npc) == 100
            {
                arrived = Some(node);
                break;
            }
        }
        let node = arrived.unwrap_or_else(|| panic!("{tag} never settled into a cover slot"));
        assert!(
            !is_flanked(node.pos, node.orient, player),
            "{tag}'s slot {:?} must face the tester",
            node.key()
        );
        assert_ne!(
            node.key(),
            RIFLEMAN3_SLOT,
            "{tag} took rifleman 3's marker, so rifleman 3 would respawn in the open"
        );
    }
}

/// Rifleman 3's marker is out of the other riflemen's cover search (review
/// finding 5): more than `MAX_COVER_DISTANCE` from where each of them stands,
/// so neither takes it while rifleman 3 is dead and it respawns holding it.
#[test]
fn rifleman3_marker_is_out_of_the_other_riflemen_search() {
    let records = world_records();
    let nodes = cover_nodes();
    let slot = nodes.iter().find(|n| n.key() == RIFLEMAN3_SLOT).unwrap();
    for tag in [RIFLEMAN1, RIFLEMAN2] {
        let r = one(&records, tag);
        let d = (slot.pos.x - r.x).hypot(slot.pos.z - r.z);
        assert!(
            d > MAX_COVER_DISTANCE,
            "{tag} stands {d:.1} u from rifleman 3's marker"
        );
    }
}

/// From its slot rifleman 3 looks past its prop from the peek point (NA23) and
/// engages a tester walking down the hall in front of the cover.
#[tokio::test]
async fn rifleman3_engages_a_tester_in_the_hall_from_cover() {
    let records = world_records();
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    let r3 = npc_id(one(&records, RIFLEMAN3));
    add_player(&mut mgr, PLAYER, [175.0, 6.7, -947.0]);
    let (engaged, targets) = scan(&mut mgr, r3).await;
    assert!(engaged && targets == vec![PLAYER], "{targets:?}");
    assert_eq!(held(&mgr, r3), Some(RIFLEMAN3_SLOT));
}
