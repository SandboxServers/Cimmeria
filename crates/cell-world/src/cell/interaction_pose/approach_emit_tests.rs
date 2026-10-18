//! Tests for the `interaction.approach` row: joined to the pose row by
//! `pose_seq`, and absent without a trail or when the pose is throttled.
//! Each test uses its own actor id (9211 to 9213) because the throttle is process-global.

use super::{
    capture_interaction_pose, emit_interaction_pose_at, InteractionKind, InteractionRoute,
    PoseExtra,
};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use std::time::{Duration, Instant};
use tracing::Level;

/// Builds an Agnos scene: player `id` at (3.3, 0.128, -2.53) and a "PoseProbe" NPC at the origin.
/// Returns the manager and the NPC id.
fn scene(id: u32) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(id, "Agnos", [3.3, 0.128, -2.53], [0.0; 3])
        .unwrap();
    {
        let player = mgr.get_entity_mut(id).unwrap();
        player.is_player = true;
        player.account_id = Some(6);
        player.player_id = Some(12);
        player.session_kind = Some("player");
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

/// Seeds three accepted positions 1 m apart, one second apart, ending now.
fn seed_trail(mgr: &mut SpaceManager, id: u32) {
    for i in 0..3u64 {
        let at = Instant::now()
            .checked_sub(Duration::from_secs(3 - i))
            .unwrap();
        mgr.sample_accepted_position_at(id, [3.3 + i as f32, 0.128, -2.53], at);
    }
}

/// Captures the pose of `id` against `target` and emits it as of `now`.
fn emit_now(mgr: &SpaceManager, id: u32, target: u32, now: Instant) -> Option<u64> {
    let pose = capture_interaction_pose(mgr, id, target).unwrap();
    emit_interaction_pose_at(
        &pose,
        InteractionKind::Interact,
        InteractionRoute::Trainer,
        PoseExtra::default(),
        now,
    )
}

/// Pins that an admitted pose with a trail writes one approach row joined to it by `pose_seq`.
#[test]
fn pose_row_with_trail_writes_a_joined_approach_row() {
    let id = 9211;
    let (mut mgr, npc) = scene(id);
    seed_trail(&mut mgr, id);
    let guard = LogCapture::install();
    let seq = emit_now(&mgr, id, npc, Instant::now()).expect("pose admitted");
    let id_s = id.to_string();
    let rows = |target: &str| {
        guard
            .all()
            .into_iter()
            .filter(|c| {
                c.target == target
                    && c.fields.get("entity_id").map(|v| v.as_str()) == Some(id_s.as_str())
            })
            .collect::<Vec<_>>()
    };
    let pose_rows = rows("interaction.pose");
    let approach = rows("interaction.approach");
    assert_eq!(pose_rows.len(), 1, "one pose row");
    assert_eq!(approach.len(), 1, "one approach row");
    let row = &approach[0];
    assert_eq!(row.level, Level::INFO);
    assert_eq!(row.fields["pose_seq"], seq.to_string());
    assert_eq!(row.fields["pose_seq"], pose_rows[0].fields["pose_seq"]);
    assert_eq!(row.fields["n_points"], "3");
    assert_eq!(row.fields["points"].matches(';').count(), 2);
    let field = |k: &str| row.fields.get(k).map(String::as_str);
    assert_eq!(field("event"), Some("interaction_approach"));
    assert_eq!(field("cimmeria.session_kind"), Some("player"));
    assert_eq!(field("world"), Some("Agnos"));
    assert_eq!(field("target_tag"), Some("PoseProbe"));
    assert_eq!(field("target_template_id"), Some("700"));
    assert_eq!(field("target_entity_id"), Some(npc.to_string().as_str()));
    assert_eq!(field("player_id"), Some("12"));
    let path: f32 = row.fields["path_len_m"].parse().unwrap();
    assert!((path - 2.0).abs() < 0.01, "path_len_m {path}");
    let span: f32 = row.fields["span_s"].parse().unwrap();
    assert!((span - 2.0).abs() < 0.1, "span_s {span}");
    for banned in [
        "account_name",
        "player_name",
        "entity_name",
        "target_entity_name",
        "account_id",
    ] {
        assert!(
            !row.fields.contains_key(banned),
            "{banned} must not be emitted"
        );
    }
}

/// Pins that a pose with no samples in the approach ring writes no approach row.
#[test]
fn no_trail_writes_no_approach_row() {
    let id = 9212;
    let (mgr, npc) = scene(id);
    let guard = LogCapture::install();
    assert!(emit_now(&mgr, id, npc, Instant::now()).is_some());
    let id_s = id.to_string();
    let rows: Vec<_> = guard
        .all()
        .into_iter()
        .filter(|c| c.fields.get("entity_id").map(|v| v.as_str()) == Some(id_s.as_str()))
        .collect();
    let pose = rows
        .iter()
        .filter(|c| c.target == "interaction.pose")
        .count();
    let approach = rows
        .iter()
        .filter(|c| c.target == "interaction.approach")
        .count();
    assert_eq!(pose, 1, "one pose row");
    assert_eq!(approach, 0, "no approach row without a trail");
}

/// Pins that a throttled pose writes neither row, so a trail never leaves an orphan approach row.
#[test]
fn throttled_pose_writes_no_approach_row() {
    let id = 9213;
    let (mut mgr, npc) = scene(id);
    seed_trail(&mut mgr, id);
    let guard = LogCapture::install();
    let t0 = Instant::now();
    assert!(emit_now(&mgr, id, npc, t0).is_some());
    assert!(emit_now(&mgr, id, npc, t0 + Duration::from_millis(100)).is_none());
    let id_s = id.to_string();
    let rows: Vec<_> = guard
        .all()
        .into_iter()
        .filter(|c| c.fields.get("entity_id").map(|v| v.as_str()) == Some(id_s.as_str()))
        .collect();
    let pose = rows
        .iter()
        .filter(|c| c.target == "interaction.pose")
        .count();
    let approach = rows
        .iter()
        .filter(|c| c.target == "interaction.approach")
        .count();
    assert_eq!(pose, 1, "one pose row");
    assert_eq!(approach, 1, "one approach row for the admitted pose only");
}
