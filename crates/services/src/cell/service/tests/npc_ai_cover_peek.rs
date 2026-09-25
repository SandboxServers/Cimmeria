//! NA23 on the real Castle_CellBlock data: guards in cover look from their
//! slot's peek point (D-NA12), and the mess-hall flank churn is damped.
//!
//! Every position here is from the UAT-1 colo session (2026-09-25, build
//! `059d6038`), read off the `npc_ai.los` and `cover.flank_check` rows:
//!
//! - `Hallway01_Guard` (spawn 30) spawns holding marker 1200037/3, a Mid
//!   counter facing -Z down the hallway. The player stood at
//!   (-130.86, 39.55, -81.42), 8.1 u in front of it on the same floor, and
//!   was rejected `no_los`: the ray from the guard hit its own counter
//!   0.34-0.42 u out. The slot's peek point is 1.08 u over the counter.
//! - `Hallway02_Guard` (spawn 82) holds 1200034/0 and shot the player dead
//!   at (-130.32, 39.55, -79.51), 23.5 u away round two hallway corners,
//!   under NA22's `in_cover_slot` exemption.
//! - `MessHall_Guard2` (spawn 28) holds 1200053/0 (facing +X). At 12:14:06
//!   the player was at (-93.75, 34.59, -106.16), in front; two seconds and
//!   2.5 u of strafe later at (-96.24, 34.57, -105.64), 101 degrees off the
//!   facing, and NA22's 5 degree band released the slot as flanked.
//!
//! Skips on a checkout without the navmesh fixture.

use std::time::{Duration, Instant};

use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

use super::npc_ai_cover_seed::{
    ai_tick, engage, held, mess_hall_guard, node_pos, seeded, Seeded, NPC, PLAYER, WORLD,
};
use crate::cell::cover::{is_flanked, CoverSlotKey, COVER_BLIND_GRACE};
use crate::cell::space_manager::{SightOrigin, SpaceManager};
use crate::cell::spawner::SpawnRecord;

const HALLWAY01_SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 1200037,
    node_id: 3,
};
const HALLWAY02_SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 1200034,
    node_id: 0,
};
const MESS_HALL2_SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 1200053,
    node_id: 0,
};
/// Where the player stood when `Hallway01_Guard` rejected it at 8.1 u.
const HALLWAY01_PLAYER: Vector3 = Vector3 {
    x: -130.86244,
    y: 39.5515,
    z: -81.4195,
};
/// Where `Hallway02_Guard` killed the player through the walls.
const HALLWAY02_VICTIM: Vector3 = Vector3 {
    x: -130.31534,
    y: 39.5495,
    z: -79.506516,
};

/// A spawnlist guard row: template 24's combat data on `mess_hall_guard`'s
/// record, at another spawn.
fn guard(spawn_id: i32, tag: &str, at: [f32; 3], heading: f32) -> SpawnRecord {
    SpawnRecord {
        spawn_id,
        tag: Some(tag.to_string()),
        x: at[0],
        y: at[1],
        z: at[2],
        heading,
        ..mess_hall_guard()
    }
}

fn hallway01_guard() -> SpawnRecord {
    guard(
        30,
        "Hallway01_Guard",
        [-128.853, 39.552, -73.534],
        3.141_497,
    )
}

fn hallway02_guard() -> SpawnRecord {
    guard(82, "Hallway02_Guard", [-113.485, 39.552, -63.042], 0.0)
}

fn mess_hall_guard2() -> SpawnRecord {
    guard(28, "MessHall_Guard2", [-95.89, 34.591, -98.808], 2.094_267_4)
}

/// A live player at `pos`, in the NPC's AoI, without touching the NPC.
fn add_player(mgr: &mut SpaceManager, pos: Vector3) {
    mgr.create_entity(PLAYER, WORLD, [pos.x, pos.y, pos.z], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
}

fn move_player(mgr: &mut SpaceManager, pos: Vector3) {
    mgr.update_entity_position(PLAYER, [pos.x, pos.y, pos.z], [0, 0, 0], [0.0; 3]);
}

fn cooling(mgr: &SpaceManager, now: Instant) -> Option<CoverSlotKey> {
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .cooling_slot(EntityId(NPC as i32), now)
}

fn spawn(s: &mut Seeded, record: &SpawnRecord, slot: CoverSlotKey) {
    s.mgr.spawn_npc_from_record(NPC, record).unwrap();
    assert_eq!(held(&s.mgr), Some(slot), "spawned holding its marker");
}

/// UAT-1 defect 1: an Idle guard spawned in cover aggroes a player in front
/// of its cover on the same floor. Before NA23 the ray from the guard hit
/// its own counter and the scan rejected the player `no_los`: revert
/// `aggro_gates::same_room` to `line_of_sight` and the guard stays Idle.
#[tokio::test]
async fn hallway01_guard_in_cover_aggroes_the_uat_player() {
    let Some(mut s) = seeded() else {
        return;
    };
    spawn(&mut s, &hallway01_guard(), HALLWAY01_SLOT);
    let slot = node_pos(&s.nodes, HALLWAY01_SLOT).clone();
    let npc_pos = s.mgr.get_entity(NPC).unwrap().position;
    assert!(
        !is_flanked(slot.pos, slot.orient, HALLWAY01_PLAYER),
        "fixture: the player is in front of the counter"
    );
    let d = ((HALLWAY01_PLAYER.x - npc_pos.x).powi(2) + (HALLWAY01_PLAYER.z - npc_pos.z).powi(2))
        .sqrt();
    assert!((7.5..8.5).contains(&d), "fixture: 8.1 u away ({d})");
    add_player(&mut s.mgr, HALLWAY01_PLAYER);
    assert_eq!(
        s.mgr.line_of_sight(NPC, PLAYER),
        cimmeria_entity::navigation::LineOfSight::Blocked,
        "fixture: the guard's own ray hits its counter (the UAT-1 bug shape)"
    );
    assert!(matches!(
        s.mgr.npc_sight_origin(NPC),
        SightOrigin::CoverPeek(_)
    ));

    ai_tick(&mut s.mgr).await;
    let npc = s.mgr.get_entity(NPC).unwrap();
    assert_eq!(
        npc.ai_state(),
        AiState::Fighting,
        "proximity aggro from cover"
    );
    assert!(npc.threat_list.contains_key(&PLAYER));
}

/// UAT-1 defect 2: a guard in cover does not shoot through walls. From
/// `Hallway02_Guard`'s slot there is no navmesh line to where it killed the
/// player, so it holds fire (`cover_no_shot`) and keeps its slot. Before
/// NA23 `in_cover_slot` let it fire whatever the ray said: revert the
/// `CoverPeek` arm of `attack_los_policy` and the shot is allowed.
#[tokio::test]
async fn hallway02_guard_in_cover_holds_fire_through_the_wall() {
    use crate::test_support::LogCapture;
    let logs = LogCapture::install();
    let Some(mut s) = seeded() else {
        return;
    };
    spawn(&mut s, &hallway02_guard(), HALLWAY02_SLOT);
    engage(&mut s.mgr, HALLWAY02_VICTIM);
    ai_tick(&mut s.mgr).await;

    assert_eq!(held(&s.mgr), Some(HALLWAY02_SLOT), "in range, not flanked");
    assert!(s.mgr.npc_sight_origin(NPC).from_cover());
    assert!(
        !s.mgr.attack_line_of_sight(NPC, PLAYER, false),
        "no shot from the slot through the hallway walls"
    );
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai" && c.has_field("npc_id", &NPC.to_string()))
        .collect();
    assert!(
        rows.iter()
            .any(|c| c.has_field("decision_outcome", "cover_no_shot")),
        "held fire: {rows:#?}"
    );
    assert!(
        !rows
            .iter()
            .any(|c| c.has_field("decision_outcome", "attack_in_place")),
        "never fired: {rows:#?}"
    );
}

/// After [`COVER_BLIND_GRACE`] with no shot the guard gives the slot up
/// (`cover_released_no_shot`) and the slot cools, so it fights on out of
/// cover instead of standing blind forever.
#[tokio::test]
async fn a_blind_guard_gives_its_slot_up_after_the_grace() {
    use crate::cell::service::npc_ai::fight_cover::{blind_in_slot, BlindInSlot};
    let Some(mut s) = seeded() else {
        return;
    };
    spawn(&mut s, &hallway02_guard(), HALLWAY02_SLOT);
    engage(&mut s.mgr, HALLWAY02_VICTIM);
    let t0 = Instant::now();
    assert_eq!(
        blind_in_slot(&mut s.mgr, NPC, PLAYER, t0),
        BlindInSlot::Hold
    );
    assert_eq!(
        blind_in_slot(&mut s.mgr, NPC, PLAYER, t0 + COVER_BLIND_GRACE / 2),
        BlindInSlot::Hold
    );
    assert_eq!(
        blind_in_slot(
            &mut s.mgr,
            NPC,
            PLAYER,
            t0 + COVER_BLIND_GRACE + Duration::from_millis(1)
        ),
        BlindInSlot::Released
    );
    assert_eq!(held(&s.mgr), None);
    assert_eq!(cooling(&s.mgr, t0), Some(HALLWAY02_SLOT));
}

/// UAT-1 tuning: the mess-hall strafe that released `MessHall_Guard2`'s slot
/// at 12:14:08 (101 degrees off the facing) no longer does. With NA22's
/// 5 degree band the second tick releases the slot.
#[tokio::test]
async fn the_uat_mess_hall_strafe_does_not_flank_the_guard() {
    let Some(mut s) = seeded() else {
        return;
    };
    spawn(&mut s, &mess_hall_guard2(), MESS_HALL2_SLOT);
    engage(&mut s.mgr, Vector3::new(-93.75, 34.59, -106.161));
    ai_tick(&mut s.mgr).await;
    assert_eq!(held(&s.mgr), Some(MESS_HALL2_SLOT), "12:14:06: in front");
    move_player(&mut s.mgr, Vector3::new(-96.238, 34.568, -105.637));
    ai_tick(&mut s.mgr).await;
    assert_eq!(
        held(&s.mgr),
        Some(MESS_HALL2_SLOT),
        "12:14:08: the strafe stays inside the hold band"
    );
}

/// A guard flanked for real gives the slot up, fires from where it stands
/// when it has a shot (no chase to 1 u), and does not re-pick the slot it
/// was flanked out of while the cooldown runs, even when the target walks
/// back in front of it. UAT-1: NPC 100160 chased to 1 u and re-picked the
/// same slot 6 s later. With `MessHall_Guard1`'s marker taken, the re-pick
/// below takes 1200053/0 again without the cooldown.
#[tokio::test]
async fn a_flanked_guard_fires_in_place_and_does_not_re_pick_its_slot() {
    use crate::test_support::LogCapture;
    let logs = LogCapture::install();
    let Some(mut s) = seeded() else {
        return;
    };
    spawn(&mut s, &mess_hall_guard2(), MESS_HALL2_SLOT);
    let front = Vector3::new(-93.75, 34.59, -106.161);
    engage(&mut s.mgr, front);
    ai_tick(&mut s.mgr).await;
    assert_eq!(held(&s.mgr), Some(MESS_HALL2_SLOT));

    // Behind the table, 5 u from the guard, in plain navmesh sight of it.
    let behind = Vector3::new(-100.96, 34.6, -99.09);
    let slot = node_pos(&s.nodes, MESS_HALL2_SLOT).clone();
    assert!(
        is_flanked(slot.pos, slot.orient, behind),
        "fixture: flanked"
    );
    let before = logs.all().len();
    move_player(&mut s.mgr, behind);
    ai_tick(&mut s.mgr).await;
    assert_ne!(held(&s.mgr), Some(MESS_HALL2_SLOT), "released as flanked");
    assert_eq!(cooling(&s.mgr, Instant::now()), Some(MESS_HALL2_SLOT));
    let npc = s.mgr.get_entity(NPC).unwrap();
    assert!(npc.nav_path.is_empty(), "no chase: it has a shot");
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .skip(before)
        .filter(|c| c.target == "npc_ai" && c.has_field("npc_id", &NPC.to_string()))
        .collect();
    assert!(
        rows.iter()
            .any(|c| c.has_field("decision_outcome", "cover_released_flanked")),
        "{rows:#?}"
    );
    // The attack branch: it fires, or holds on a cooldown (`no_ability`)
    // when an earlier tick already fired. Either way it stands and shoots.
    assert!(
        rows.iter()
            .any(|c| c.has_field("decision_outcome", "attack_in_place")
                || c.has_field("decision_outcome", "no_ability")),
        "fired from where it stood: {rows:#?}"
    );
    assert!(
        !rows
            .iter()
            .any(|c| c.has_field("decision_outcome", "chase")),
        "no chase: {rows:#?}"
    );

    // Back in front, where the slot sees the player over its table (60
    // degrees off its facing, 10 u): the guard may take other cover, but
    // not that slot.
    let seen = Vector3::new(-90.03, 34.6, -90.33);
    assert!(
        s.mgr.slot_has_shot(NPC, &slot, seen),
        "fixture: the slot has a shot"
    );
    // MessHall_Guard1's marker would out-score it; that guard holds it in
    // play, so hold it here too.
    s.mgr
        .cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(
            EntityId(999),
            CoverSlotKey {
                chunk_id: 1200046,
                node_id: 0,
            },
        )
        .unwrap();
    move_player(&mut s.mgr, seen);
    for _ in 0..3 {
        ai_tick(&mut s.mgr).await;
        assert_ne!(held(&s.mgr), Some(MESS_HALL2_SLOT), "slot cooling");
    }
}

/// Cover is a firing position (D-NA05): an NPC in the open near
/// `Hallway02_Guard`'s marker does not take it against a target it could not
/// see from there (the UAT-1 victim's spot, round two hallway corners). Pass
/// `|_| true` as the fight tick's shot check and it reserves 1200034/0 and
/// walks to a slot it would stand in blind.
#[tokio::test]
async fn an_npc_does_not_pick_a_slot_it_has_no_shot_from() {
    let Some(mut s) = seeded() else {
        return;
    };
    let start = (s.navmesh_snap)([-110.0, 39.55, -63.0]);
    s.mgr
        .spawn_npc(NPC, WORLD, [start.x, start.y, start.z], [0.0; 3])
        .unwrap();
    {
        let npc = s.mgr.get_entity_mut(NPC).unwrap();
        npc.use_cover = true;
        npc.faction = 10;
        npc.move_speed = 0.6;
        npc.spawn_position = Some(start);
    }
    let slot = node_pos(&s.nodes, HALLWAY02_SLOT).clone();
    assert!(!is_flanked(slot.pos, slot.orient, HALLWAY02_VICTIM));
    assert!(
        !s.mgr.slot_has_shot(NPC, &slot, HALLWAY02_VICTIM),
        "fixture: no line from the slot"
    );
    engage(&mut s.mgr, HALLWAY02_VICTIM);
    ai_tick(&mut s.mgr).await;
    assert_eq!(held(&s.mgr), None, "no blind slot taken");
}
