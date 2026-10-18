use super::geometry::*;

/// Fails if the bearing `atan2` arguments are swapped or the radius is wrong for the lab stand-off pose.
#[test]
fn frost_stand_off_matches_the_lab_pose() {
    let g = pose_geometry([-325.0, 73.6, -212.8], 0.0, [-328.30, 73.472, -210.27]);
    assert!((g.radius_m - 4.158).abs() < 0.01, "radius {}", g.radius_m);
    assert!(
        (g.bearing_deg - 322.5).abs() < 0.1,
        "bearing {}",
        g.bearing_deg
    );
    assert!((g.dy_m - 0.128).abs() < 0.001, "dy {}", g.dy_m);
    assert!(g.dist_3d_m >= g.radius_m);
}

/// Fails if the facing error ignores the player's yaw, since facing the target must give zero error.
#[test]
fn facing_the_target_gives_zero_error() {
    let first = pose_geometry([3.0, 0.0, 4.0], 0.0, [0.0, 0.0, 0.0]);
    let g = pose_geometry(
        [3.0, 0.0, 4.0],
        first.yaw_to_target_deg.to_radians(),
        [0.0, 0.0, 0.0],
    );
    assert!(
        g.facing_error_deg.abs() < 0.01,
        "error {}",
        g.facing_error_deg
    );
}

/// Fails if the facing error does not wrap across north, where -20 and +20 meet.
#[test]
fn facing_error_wraps_across_north() {
    let left = pose_geometry([0.0, 0.0, 0.0], 20.0_f32.to_radians(), [0.0, 0.0, 5.0]);
    assert!(
        (left.facing_error_deg + 20.0).abs() < 0.01,
        "{}",
        left.facing_error_deg
    );
    let right = pose_geometry([0.0, 0.0, 0.0], 340.0_f32.to_radians(), [0.0, 0.0, 5.0]);
    assert!(
        (right.facing_error_deg - 20.0).abs() < 0.01,
        "{}",
        right.facing_error_deg
    );
}

/// Fails if a cardinal direction maps to the wrong bearing quadrant.
#[test]
fn cardinal_bearings() {
    let cases = [
        ([3.0, 0.0, 0.0], 0.0),
        ([0.0, 0.0, 3.0], 90.0),
        ([-3.0, 0.0, 0.0], 180.0),
        ([0.0, 0.0, -3.0], 270.0),
    ];
    for (player, want) in cases {
        let g = pose_geometry(player, 0.0, [0.0, 0.0, 0.0]);
        assert!(
            (g.bearing_deg - want).abs() < 0.01,
            "{player:?}: {}",
            g.bearing_deg
        );
    }
}

/// Fails if the degree wrappers lose their range contract at the edges.
#[test]
fn wrap_bounds() {
    assert!((wrap_deg_360(-37.5) - 322.5).abs() < 1e-4);
    assert_eq!(wrap_deg_360(360.0), 0.0);
    assert!((wrap_deg_180(190.0) + 170.0).abs() < 1e-4);
    assert!((wrap_deg_180(-190.0) - 170.0).abs() < 1e-4);
    assert_eq!(wrap_deg_180(180.0), 180.0);
}

/// Fails if a label drifts from the contract string that downstream filters match on.
#[test]
fn labels_are_stable() {
    assert_eq!(InteractionKind::Interact.as_str(), "interact");
    assert_eq!(InteractionKind::UseItem.as_str(), "use_item");
    assert_eq!(InteractionRoute::Trainer.as_str(), "trainer");
    assert_eq!(InteractionRoute::ChainTag.as_str(), "chain_tag");
    assert_eq!(InteractionRoute::ChainTemplate.as_str(), "chain_template");
    assert_eq!(InteractionRoute::Fallback.as_str(), "fallback");
    assert_eq!(InteractionRoute::ItemUsed.as_str(), "item_used");
}
