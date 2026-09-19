//! A stationary NPC turns to face what it is fighting, including while it
//! is holding fire.
//!
//! The 2026-09-18 Castle playtest (finding H4b) found that `direction` was
//! written only by the movement tick, so a path-less attacker's yaw froze.
//! The first fix covered the stop-and-attack branch. A stationary NPC that
//! is out of range, or reads "no LoS" across a navmesh gap, returns from
//! `npc_ai_fight` earlier than that, in the `stationary_holds` branch, and
//! never gets a nav path at all. Harset seeds thirteen such sentries.

use super::*;
use cimmeria_common::Vector3;
use std::f32::consts::FRAC_PI_2;
use tokio::sync::mpsc;

/// NPC 200 at the origin, pinned, fighting player 101 at `target_pos`, with
/// a stale facing of due east left over from its authored heading.
fn pinned_sentry_fighting(target_pos: [f32; 3]) -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.create_entity(101, "Castle", target_pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(101, 10.0);
        npc.is_stationary = true;
        npc.direction = Vector3::new(0.0, FRAC_PI_2, 0.0);
    }
    mgr
}

async fn tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// Target 40 units due WEST: past `NPC_ATTACK_RANGE` (30) and inside
/// `LEASH_DISTANCE` (50), so the tick lands in `stationary_holds`. The
/// sentry must still turn to yaw = atan2(-40, 0) = -PI/2.
///
/// The bearing is deliberately in the negative half of the compass: that is
/// the half `pack_angle` used to collapse, and a `[0, PI]` fixture would
/// also pass against a yaw that was merely zeroed.
#[tokio::test]
async fn stationary_npc_holding_fire_turns_to_face_its_target() {
    let mut mgr = pinned_sentry_fighting([-40.0, 0.0, 0.0]);
    tick(&mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "control: the sentry must still be pinned (no nav path), otherwise this \
         test is exercising the chase branch, not stationary_holds"
    );
    let yaw = npc.direction.y;
    assert!(
        (yaw + FRAC_PI_2).abs() < 1e-4,
        "a stationary NPC holding fire must face its target (-PI/2), not keep \
         the stale +PI/2 heading; got {yaw}"
    );
}

/// A target directly overhead has no bearing in XZ. The sentry keeps the
/// yaw it had instead of snapping to 0 (due north), which is what
/// `atan2(0, 0)` would produce.
#[tokio::test]
async fn target_directly_overhead_does_not_snap_the_npc_to_north() {
    let mut mgr = pinned_sentry_fighting([0.0, 5.0, 0.0]);
    tick(&mut mgr).await;

    let yaw = mgr.get_entity(200).unwrap().direction.y;
    assert!(
        (yaw - FRAC_PI_2).abs() < 1e-4,
        "with no XZ bearing to the target the NPC must keep its yaw (+PI/2); got {yaw}"
    );
}
