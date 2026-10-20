//! CB-N03t — the other three cover guards of world 12 (`MessHall_Guard2`,
//! `Hallway05_Guard1`, `Hallway05_Guard2`) spawn holding their marker,
//! take Cover Stance against a threat in front, and release when flanked,
//! on the real navmesh and cover seed. `npc_ai_cover_seed.rs` proves the
//! same for `MessHall_Guard1`. Static evidence for CB-N03; the owner's
//! CB-U05 lab row is still the acceptance.

use cimmeria_common::Vector3;

use super::npc_ai_cover_behaviour::cover_defense;
use super::npc_ai_cover_seed::{
    ai_tick, engage, held, mess_hall_guard, node_pos, seeded, Seeded, NPC, PLAYER,
};
use crate::cell::cover::{is_flanked, CoverSlotKey};
use crate::cell::spawner::SpawnRecord;

struct Guard {
    spawn_id: i32,
    tag: &'static str,
    pos: [f32; 3],
    heading: f32,
    slot: CoverSlotKey,
}

/// `spawnlist.sql` rows 28, 32, 33 (template 24) and the cover marker each
/// spawns on.
const GUARDS: &[Guard] = &[
    Guard {
        spawn_id: 28,
        tag: "MessHall_Guard2",
        pos: [-95.89, 34.591, -98.808],
        heading: 2.094_267_4,
        slot: CoverSlotKey {
            chunk_id: 1200053,
            node_id: 0,
        },
    },
    Guard {
        spawn_id: 32,
        tag: "Hallway05_Guard1",
        pos: [-100.16, 24.67, -43.895],
        heading: 1.570_796_4,
        slot: CoverSlotKey {
            chunk_id: 1200056,
            node_id: 1,
        },
    },
    Guard {
        spawn_id: 33,
        tag: "Hallway05_Guard2",
        pos: [-101.5, 24.67, -51.3],
        heading: 1.570_796_4,
        slot: CoverSlotKey {
            chunk_id: 1200044,
            node_id: 3,
        },
    },
];

/// The spawnlist row for one of `GUARDS`, with template 24's combat data.
fn record(g: &Guard) -> SpawnRecord {
    let mut r = mess_hall_guard();
    r.spawn_id = g.spawn_id;
    r.x = g.pos[0];
    r.y = g.pos[1];
    r.z = g.pos[2];
    r.heading = g.heading;
    r.tag = Some(g.tag.to_string());
    r
}

/// A point on the mesh `dist` u from the marker `slot` along (or against)
/// its facing.
fn along(s: &Seeded, slot: CoverSlotKey, dist: f32) -> Vector3 {
    let n = node_pos(&s.nodes, slot);
    (s.navmesh_snap)([
        n.pos.x + dist * n.orient.cos(),
        n.pos.y,
        n.pos.z + dist * n.orient.sin(),
    ])
}

/// The first point in front of `slot` that is not flanked.
fn front(s: &Seeded, slot: CoverSlotKey) -> Vector3 {
    let node = node_pos(&s.nodes, slot).clone();
    [12.0, 10.0, 8.0, 6.0]
        .into_iter()
        .map(|d| along(s, slot, d))
        .find(|p| !is_flanked(node.pos, node.orient, *p))
        .unwrap_or_else(|| panic!("no front point for {slot:?}"))
}

/// The first point behind `slot` that is flanked.
fn behind(s: &Seeded, slot: CoverSlotKey) -> Vector3 {
    let node = node_pos(&s.nodes, slot).clone();
    [-8.0, -6.0, -4.0]
        .into_iter()
        .map(|d| along(s, slot, d))
        .find(|p| is_flanked(node.pos, node.orient, *p))
        .unwrap_or_else(|| panic!("no flank point for {slot:?}"))
}

/// Each of the other three cover guards spawns holding its marker.
#[tokio::test]
async fn other_cover_guards_spawn_holding_their_marker() {
    for g in GUARDS {
        let Some(mut s) = seeded() else {
            return;
        };
        s.mgr.spawn_npc_from_record(NPC, &record(g)).unwrap();
        assert_eq!(held(&s.mgr), Some(g.slot), "{}: spawned holding", g.tag);
    }
}

/// Each takes Cover Stance against a threat in front of its marker.
#[tokio::test]
async fn other_cover_guards_take_cover_stance_against_a_threat_in_front() {
    for g in GUARDS {
        let Some(mut s) = seeded() else {
            return;
        };
        s.mgr.spawn_npc_from_record(NPC, &record(g)).unwrap();
        let player = front(&s, g.slot);
        engage(&mut s.mgr, player);
        ai_tick(&mut s.mgr).await;
        assert_eq!(held(&s.mgr), Some(g.slot), "{}: slot kept", g.tag);
        assert_eq!(cover_defense(&s.mgr, NPC), 100, "{}: Cover Stance", g.tag);
    }
}

/// Each releases its slot and Cover Stance once the player is behind it.
#[tokio::test]
async fn other_cover_guards_release_when_flanked() {
    for g in GUARDS {
        let Some(mut s) = seeded() else {
            return;
        };
        s.mgr.spawn_npc_from_record(NPC, &record(g)).unwrap();
        let player = front(&s, g.slot);
        engage(&mut s.mgr, player);
        ai_tick(&mut s.mgr).await;
        assert_eq!(cover_defense(&s.mgr, NPC), 100, "{}: in cover", g.tag);

        let b = behind(&s, g.slot);
        s.mgr
            .update_entity_position(PLAYER, [b.x, b.y, b.z], [0, 0, 0], [0.0; 3]);
        ai_tick(&mut s.mgr).await;
        assert_ne!(
            held(&s.mgr),
            Some(g.slot),
            "{}: flanked slot released",
            g.tag
        );
        assert_eq!(cover_defense(&s.mgr, NPC), 0, "{}: stance removed", g.tag);
    }
}
