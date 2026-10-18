//! Tests for the `interaction.pose` row: capture, field set, throttle and extras.
//! Each test uses its own actor id (9201 to 9207) because the throttle is process-global.

use super::{
    capture_interaction_pose, emit_interaction_pose_at, InteractionKind, InteractionRoute,
    PoseExtra, POSE_MIN_INTERVAL,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use std::time::{Duration, Instant};
use tracing::Level;

/// Builds an Agnos scene: player `id` at (3.3, 0.128, -2.53) with yaw `yaw_rad`,
/// and a "PoseProbe" NPC at the origin. Returns the manager and the NPC id.
fn scene(id: u32, yaw_rad: f32) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(id, "Agnos", [3.3, 0.128, -2.53], [0.0; 3])
        .unwrap();
    {
        let player = mgr.get_entity_mut(id).unwrap();
        player.is_player = true;
        player.account_id = Some(6);
        player.player_id = Some(12);
        player.session_kind = Some("player");
        player.direction.y = yaw_rad;
    }
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    {
        let probe = mgr.get_entity_mut(npc).unwrap();
        probe.tag = Some("PoseProbe".into());
        probe.template_id = Some(700);
    }
    (mgr, npc)
}

/// Emits one row for `id` against `target` at `now` and returns the result.
fn emit_at(
    mgr: &SpaceManager,
    id: u32,
    target: u32,
    extra: PoseExtra,
    now: Instant,
) -> Option<u64> {
    let pose = capture_interaction_pose(mgr, id, target).unwrap();
    emit_interaction_pose_at(
        &pose,
        InteractionKind::Interact,
        InteractionRoute::Trainer,
        extra,
        now,
    )
}

/// Pins that capture reads the geometry, the target tag and the player identity together.
#[test]
fn capture_reads_both_poses() {
    let id = 9201;
    let (mgr, npc) = scene(id, 0.0);
    let pose = capture_interaction_pose(&mgr, id, npc).unwrap();
    assert!(
        (pose.geometry.radius_m - 4.158).abs() < 0.01,
        "radius_m {}",
        pose.geometry.radius_m
    );
    assert!(
        (pose.geometry.bearing_deg - 322.5).abs() < 0.1,
        "bearing_deg {}",
        pose.geometry.bearing_deg
    );
    assert_eq!(pose.target_tag.as_deref(), Some("PoseProbe"));
    assert_eq!(pose.session_kind, Some("player"));
    assert_eq!(pose.account_id, Some(6));
    assert_eq!(pose.world_id, mgr.get_entity_world_id(id));
}

/// Pins that capture wraps the entity's radian yaw and pitch into degree ranges, not raw negatives.
#[test]
fn capture_wraps_yaw_and_pitch() {
    let id = 9208;
    let (mut mgr, npc) = scene(id, -0.5);
    mgr.get_entity_mut(id).unwrap().direction.x = -0.25;
    mgr.get_entity_mut(npc).unwrap().direction.y = -1.0;
    let pose = capture_interaction_pose(&mgr, id, npc).unwrap();
    assert!(
        (pose.player_yaw_deg - 331.35).abs() < 0.05,
        "{}",
        pose.player_yaw_deg
    );
    assert!(
        (pose.wire_pitch_deg + 14.32).abs() < 0.05,
        "{}",
        pose.wire_pitch_deg
    );
    assert!(
        (pose.target_yaw_deg - 302.70).abs() < 0.05,
        "{}",
        pose.target_yaw_deg
    );
    assert_eq!(pose.target_template_id, Some(700));
}

/// Pins that capture refuses NPC actors, self-targets and missing targets, so no row is built for them.
#[test]
fn capture_skips_npc_actor_self_and_missing() {
    let id = 9202;
    let (mut mgr, npc) = scene(id, 0.0);
    assert!(capture_interaction_pose(&mgr, id, 9999).is_none());
    assert!(capture_interaction_pose(&mgr, id, id).is_none());
    mgr.get_entity_mut(id).unwrap().is_player = false;
    assert!(capture_interaction_pose(&mgr, id, npc).is_none());
}

/// Pins that the row carries the pose and no personal name or target coordinate.
#[test]
fn row_carries_pose_and_no_names() {
    let id = 9203;
    let (mgr, npc) = scene(id, 0.0);
    let guard = LogCapture::install();
    let seq = emit_at(&mgr, id, npc, PoseExtra::default(), Instant::now());
    assert!(seq.is_some());
    let row = guard
        .all()
        .into_iter()
        .find(|c| c.target == "interaction.pose")
        .expect("interaction.pose row");
    assert_eq!(row.level, Level::INFO);
    assert!(row.fields.contains_key("event"));
    assert!(row.fields.contains_key("kind"));
    assert!(row.fields.contains_key("route"));
    assert_eq!(
        row.fields.get("cimmeria.session_kind").map(String::as_str),
        Some("player")
    );
    let radius: f32 = row.fields["radius_m"].parse().unwrap();
    assert!((radius - 4.158).abs() < 0.01, "radius_m {radius}");
    for banned in [
        "account_name",
        "player_name",
        "entity_name",
        "target_entity_name",
        "target_x",
        "dist_3d_m",
    ] {
        assert!(
            !row.fields.contains_key(banned),
            "{banned} must not be emitted"
        );
    }
}

/// Pins the throttle: a row 100 ms after an admitted one is dropped, and one 300 ms after is admitted.
#[test]
fn throttle_drops_rows_inside_250ms() {
    let id = 9204;
    let (mgr, npc) = scene(id, 0.0);
    let guard = LogCapture::install();
    let t0 = Instant::now();
    let first = emit_at(&mgr, id, npc, PoseExtra::default(), t0);
    let dropped = emit_at(
        &mgr,
        id,
        npc,
        PoseExtra::default(),
        t0 + Duration::from_millis(100),
    );
    let third = emit_at(
        &mgr,
        id,
        npc,
        PoseExtra::default(),
        t0 + Duration::from_millis(300),
    );
    assert!(first.is_some());
    assert!(dropped.is_none());
    assert!(third.is_some());
    assert!(third > first, "pose_seq must rise: {first:?} {third:?}");
    assert_eq!(POSE_MIN_INTERVAL, Duration::from_millis(250));
    let rows = guard
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .count();
    assert_eq!(rows, 2, "exactly two rows for actor {id}");
}

/// Pins that extras write their own keys and that the default writes neither.
#[test]
fn extra_fields_are_named() {
    let id = 9205;
    let (mgr, npc) = scene(id, 0.0);
    let guard = LogCapture::install();
    let extra = PoseExtra {
        dialog_id: Some(1),
        item_type_id: Some(42),
    };
    assert!(emit_at(&mgr, id, npc, extra, Instant::now()).is_some());
    let with = guard
        .all()
        .into_iter()
        .rfind(|c| c.target == "interaction.pose")
        .expect("row with extras");
    assert!(with.fields.contains_key("dialog_id"));
    assert!(with.fields.contains_key("item_type_id"));

    let bare_id = 9206;
    let (bare_mgr, bare_npc) = scene(bare_id, 0.0);
    assert!(emit_at(
        &bare_mgr,
        bare_id,
        bare_npc,
        PoseExtra::default(),
        Instant::now()
    )
    .is_some());
    let bare = guard
        .all()
        .into_iter()
        .rfind(|c| c.target == "interaction.pose")
        .expect("row without extras");
    assert!(!bare.fields.contains_key("dialog_id"));
    assert!(!bare.fields.contains_key("dialog_name"));
    assert!(!bare.fields.contains_key("item_type_id"));
    assert!(!bare.fields.contains_key("item_name"));
}

/// Pins the tracing field cap: the row must keep at most 31 fields besides the message.
#[test]
fn row_has_at_most_31_fields() {
    let id = 9207;
    let (mgr, npc) = scene(id, 0.0);
    let guard = LogCapture::install();
    let extra = PoseExtra {
        dialog_id: Some(1),
        item_type_id: Some(42),
    };
    assert!(emit_at(&mgr, id, npc, extra, Instant::now()).is_some());
    let row = guard
        .all()
        .into_iter()
        .find(|c| c.target == "interaction.pose")
        .expect("interaction.pose row");
    let fields = row
        .fields
        .keys()
        .filter(|k| k.as_str() != "message")
        .count();
    assert!(fields <= 31, "row has {fields} fields");
}
