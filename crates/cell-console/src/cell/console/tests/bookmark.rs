//! `.bug` playtest bookmark: the snapshot must describe the scene the way a
//! tester's screenshot would — who is near, which way they are *transmitted*
//! as facing, and whether that is toward the tester.
//!
//! Filter prefix: `playtest_bug_`.

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::{decode_feedback, setup};
use crate::cell::console::bookmark::{
    angle_diff_deg, capture, unpack_yaw_byte, CAPTURE_MAX_ENTITIES, CAPTURE_RADIUS,
};
use crate::cell::console::exec;
use crate::test_support::LogCapture;

#[test]
fn playtest_bug_angle_diff_wraps_to_the_short_way_round() {
    assert!((angle_diff_deg(0.1, -0.1) - 11.459).abs() < 0.01);
    // 179 deg vs -179 deg is 2 deg apart, not 358.
    let a = 179.0_f32.to_radians();
    assert!((angle_diff_deg(a, -a) - 2.0).abs() < 0.01);
    assert!((angle_diff_deg(std::f32::consts::PI, 0.0) - 180.0).abs() < 0.01);
    assert!((unpack_yaw_byte(64) - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
}

/// The "walks at me backwards" signature: an NPC whose transmitted yaw points
/// directly away from the tester must read ~180 degrees, and the selected
/// target must lead the list regardless of distance.
#[test]
fn playtest_bug_reports_wire_facing_relative_to_the_tester() {
    let (mut mgr, gm, npc) = setup(); // gm (10,0,10), npc (12,0,12)
                                      // Bearing npc -> gm is atan2(-2,-2) = -135 deg. Face the NPC at +45 deg
                                      // (away from the tester); a positive yaw packs correctly today, so this
                                      // stays valid whether or not the negative-yaw saturation bug is fixed.
    mgr.get_entity_mut(npc).unwrap().direction =
        Vector3::new(0.0, std::f32::consts::FRAC_PI_4, 0.0);
    // A closer, unselected NPC must still sort AFTER the selected target.
    let near = mgr.allocate_npc_id();
    mgr.spawn_npc(near, "Agnos", [10.5, 0.0, 10.0], [0.0; 3])
        .unwrap();

    let b = capture(gm, Some(npc), "guard moonwalking", &mgr).expect("caller exists");

    assert_eq!(b.note, "guard moonwalking");
    assert!(
        !b.crouched && b.cover_sets.is_empty(),
        "fixture player is neither"
    );
    assert_eq!(b.world_name, "Agnos");
    assert!(!b.navmesh_loaded, "fixture space has no navmesh");
    assert_eq!(b.selected_target_id, npc);
    assert_eq!(b.entities.len(), 2);
    let first = &b.entities[0];
    assert_eq!(first.entity_id, npc, "selected target leads the list");
    assert!(first.is_selected_target);
    assert_eq!(first.yaw_byte, 32, "pi/4 packs to 32/256 of a turn");
    assert!(
        (first.wire_facing_vs_caller_deg - 180.0).abs() < 2.0,
        "facing directly away from the tester, got {}",
        first.wire_facing_vs_caller_deg
    );
    assert!((first.dist - 8.0_f32.sqrt()).abs() < 1e-3);
    assert_eq!(b.entities[1].entity_id, near);
    assert!(!b.caller.is_selected_target);
}

/// Nearest-first, capped, radius-limited — but the selected target is captured
/// even when it stands outside the radius (the tester is pointing at it).
#[test]
fn playtest_bug_caps_by_distance_and_always_keeps_the_target() {
    let (mut mgr, gm, npc) = setup();
    for i in 0..40 {
        let id = mgr.allocate_npc_id();
        mgr.spawn_npc(id, "Agnos", [10.0 + 1.0 + i as f32, 0.0, 10.0], [0.0; 3])
            .unwrap();
    }
    let far = mgr.allocate_npc_id();
    mgr.spawn_npc(
        far,
        "Agnos",
        [10.0 + CAPTURE_RADIUS + 25.0, 0.0, 10.0],
        [0.0; 3],
    )
    .unwrap();

    let b = capture(gm, Some(far), "", &mgr).unwrap();
    assert_eq!(b.entities.len(), CAPTURE_MAX_ENTITIES);
    assert_eq!(
        b.entities_in_radius, 42,
        "40 + setup's npc + the far target"
    );
    assert_eq!(b.entities[0].entity_id, far, "out-of-radius target is kept");
    assert!(b.entities[1..].windows(2).all(|w| w[0].dist <= w[1].dist));

    let untargeted = capture(gm, None, "", &mgr).unwrap();
    assert!(
        untargeted.entities.iter().all(|e| e.entity_id != far),
        "an unselected entity outside the radius is not captured"
    );
    let _ = npc;
}

/// NA24 (UAT-1 B): every colo entity row read `witness_count=0` and
/// `caller_witnesses_it=false`, even for a guard fighting the tester, because
/// the snapshot read the NPC's own `witnesses` set -- which only players have.
/// After an AoI pass the tester sees the NPC, so both fields must say so.
/// Revert proof: read `e.witnesses` again and both assertions fail.
#[test]
fn playtest_bug_reports_the_players_witnessing_an_npc() {
    let (mut mgr, gm, npc) = setup();
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_entity(gm)
            .unwrap()
            .witnesses
            .contains(&mgr.get_entity(npc).unwrap().entity_id),
        "fixture: the AoI pass puts the NPC in the tester's view"
    );

    let b = capture(gm, Some(npc), "", &mgr).unwrap();
    let row = b.entities.iter().find(|e| e.entity_id == npc).unwrap();
    assert!(row.caller_witnesses_it, "the tester sees this NPC");
    assert_eq!(row.witness_count, 1, "one player (the tester) sees it");
}

/// NA44 (handoff §2): the entity row says what the NPC fights with and what
/// the tester is aiming at. An NPC's weapon lives in its template
/// `components`, not in `weapon_visual`, so the row must read it from there.
/// Revert proof: drop the three fields from `snapshot_entity` and this fails
/// to compile; point `weapon_visual` back at `e.weapon_visual` alone and the
/// NPC's weapon reads empty.
#[test]
fn playtest_bug_reports_abilities_weapon_and_current_target() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.abilities.add_ability(559);
        e.components = vec![
            "AR_H_Ablative.AR_HM_AT3_AT300".to_string(),
            "WP-Human.WP_SMG_1A".to_string(),
        ];
    }

    let b = capture(gm, Some(npc), "", &mgr).unwrap();
    let row = b.entities.iter().find(|e| e.entity_id == npc).unwrap();
    assert_eq!(
        row.ability_ids,
        vec![559, 592],
        "sorted; 592 is spawn_npc's Pistol Shot fallback"
    );
    assert_eq!(row.weapon_visual, "WP-Human.WP_SMG_1A");
    assert_eq!(row.current_target_id, 0, "an NPC selects nothing");
    assert_eq!(b.caller.current_target_id, npc as i32);
}

/// End to end through the dispatcher: header row + one row per entity share a
/// `bookmark_id`, and the tester gets an acknowledgement.
#[tokio::test]
async fn playtest_bug_emits_header_and_entity_rows_and_acks() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    let logs = LogCapture::install();

    exec(
        "bug",
        gm,
        &["he", "is", "floating"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let header = logs
        .find_message(Level::INFO, "tester flagged this moment")
        .expect("header row");
    assert_eq!(header.target, "playtest.bookmark");
    assert!(header.has_field("note", "he is floating"));
    let entity = logs
        .find_message(Level::INFO, "entity near the tester")
        .expect("entity row");
    assert_eq!(entity.target, "playtest.bookmark.entity");
    assert_eq!(
        entity.fields.get("bookmark_id"),
        header.fields.get("bookmark_id"),
        "rows join on bookmark_id"
    );
    assert!(entity.has_field("is_selected_target", "true"));
    // NA44: the identity fields reach the exported row, not just the snapshot.
    assert!(
        entity.has_field("ability_ids", "[592]"),
        "{:?}",
        entity.fields
    );
    assert!(entity.fields.contains_key("weapon_visual"));
    assert!(entity.has_field("current_target_id", "0"));

    let mut acked = false;
    while let Ok(msg) = rx.try_recv() {
        if decode_feedback(&msg).is_some_and(|t| t.contains("Bookmark") && t.contains("recorded")) {
            acked = true;
        }
    }
    assert!(acked, "tester must get visible confirmation");
}
