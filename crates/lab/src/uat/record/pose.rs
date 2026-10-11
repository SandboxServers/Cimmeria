//! LR-03: the pose derivation from the camera readout and the entity positions.

use crate::supervisor::world::geometry::{server_to_client, yaw_error, Vec3};
use crate::uat::spec::{AimMode, PoseSpec};

use super::log::{CameraSample, EntitySample};

/// Why a recorded click cannot become a pose.
#[derive(Debug, Clone, PartialEq)]
pub enum PoseError {
    /// The player stands closer than 1 m or farther than 8 m from the target.
    RadiusOutOfRange(f64),
    /// The player stands more than 3 m above or below the target's origin
    /// (outside SV-03's `dy_m` range; clamping would replay it elsewhere).
    DyOutOfRange(f64),
    /// The target has no `spawnlist` tag, or one the spec validator rejects
    /// (not `[A-Za-z0-9_.]+`).
    NoTag,
    /// The camera readout holds a non-finite value. A camera that was not
    /// read at all is not an error: its fields are `None`.
    NoCamera,
    /// A position is missing or not finite.
    NoPosition,
}

/// The pose `@stand` + `@camera { pose }` would reproduce (D-SV3, D-SV4).
/// Values are rounded to the precision `PoseSpec::to_inline_toml` writes and
/// the camera fields clamped to the ranges it clamps to, so the returned
/// struct equals the line that is written (`a_derived_pose_survives_the_inline_toml_round_trip`).
pub fn derive_pose(
    id: &str,
    target: &EntitySample,
    player: &EntitySample,
    camera: Option<&CameraSample>,
    date: &str,
) -> Result<PoseSpec, PoseError> {
    if !(target.pos.iter().chain(&player.pos)).all(|v| v.is_finite()) {
        return Err(PoseError::NoPosition);
    }
    let tag = target
        .tag
        .clone()
        .filter(|t| {
            !t.is_empty()
                && t.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        })
        .ok_or(PoseError::NoTag)?;
    let dx = player.pos[0] - target.pos[0];
    let dz = player.pos[2] - target.pos[2];
    let radius = dx.hypot(dz);
    if !(1.0..=8.0).contains(&round_to(radius, 2)) {
        return Err(PoseError::RadiusOutOfRange(radius));
    }
    let dy = round_to(player.pos[1] - target.pos[1], 2);
    if !(-3.0..=3.0).contains(&dy) {
        return Err(PoseError::DyOutOfRange(player.pos[1] - target.pos[1]));
    }
    let bearing = match round_to(dz.atan2(dx).to_degrees().rem_euclid(360.0), 1) {
        b if b >= 360.0 => 0.0,
        b => b,
    };
    let (pitch_deg, yaw_offset_deg, zoom) = match camera {
        Some(cam) => {
            let finite = cam.pos.iter().all(|v| v.is_finite())
                && [cam.yaw_deg, cam.pitch_deg, cam.zoom, cam.yaw_gain]
                    .iter()
                    .all(|v| v.is_finite());
            if !finite {
                return Err(PoseError::NoCamera);
            }
            let cam_at = server_to_client(server_vec(cam.pos));
            let target_at = server_to_client(server_vec(target.pos));
            let error = yaw_error(cam_at, cam.yaw_deg.to_radians(), target_at).to_degrees();
            let offset = if cam.yaw_gain < 0.0 { error } else { -error };
            (
                // The same clamps `to_inline_toml` applies (the readout's
                // pitch limit -78.75 rounds to -78.8, which would not load).
                Some(round_to(cam.pitch_deg, 1).clamp(-78.7, 78.7)),
                Some(round_to(offset, 1).clamp(-180.0, 180.0)),
                Some(round_to(cam.zoom, 1).clamp(100.0, 775.0)),
            )
        }
        None => (None, None, None),
    };
    Ok(PoseSpec {
        id: id.to_string(),
        tag,
        radius_m: round_to(radius, 2),
        bearing_deg: bearing,
        dy_m: dy,
        pitch_deg,
        yaw_offset_deg,
        zoom,
        aim: AimMode::Entity,
        aim_dy_m: None,
        calibrated: Some(format!("recorded {date}")),
    })
}

/// A server `[x, y, z]` position as the geometry helpers take it.
fn server_vec(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}

/// `v` rounded to `places` decimals, the value a `{:.N}` pose field holds.
/// Mirrors the private `round_to` in `uat/spec.rs`; the round-trip test
/// catches drift. `+ 0.0` turns `-0.0` into `0.0` so no `-0.00` is written.
fn round_to(v: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (v * k).round() / k + 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::record::log::read_log;

    const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

    /// The FS-P3 target (the Frost entity on tick 32), the player and the
    /// camera readout on tick 31, the tick before the interact.
    fn fs_p3() -> (EntitySample, EntitySample, CameraSample) {
        let log = read_log(FIXTURE).unwrap();
        let tick = |seq| log.ticks.iter().find(|t| t.seq == seq).unwrap();
        let target = tick(32).entities[0].clone();
        let player = tick(31).player.clone().unwrap();
        let camera = tick(31).camera.clone().unwrap();
        (target, player, camera)
    }

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn fs_p3_positions_give_the_calibrated_frost_pose() {
        let (target, player, camera) = fs_p3();
        let pose = derive_pose("frost", &target, &player, Some(&camera), "2026-10-12").unwrap();
        assert_eq!(pose.id, "frost");
        assert_eq!(pose.tag, "ArmYourself_FrostBody");
        assert_eq!(pose.radius_m, 4.16);
        assert_eq!(pose.bearing_deg, 322.5);
        assert_eq!(pose.dy_m, 0.13);
        assert_eq!(pose.pitch_deg, Some(-28.8));
        assert_eq!(pose.yaw_offset_deg, Some(25.3));
        assert_eq!(pose.zoom, Some(250.0));
        assert_eq!(pose.aim, AimMode::Entity);
        assert_eq!(pose.aim_dy_m, None);
        assert_eq!(pose.calibrated.as_deref(), Some("recorded 2026-10-12"));
    }

    #[test]
    fn inverting_the_yaw_gain_flips_the_offset() {
        let (target, player, mut camera) = fs_p3();
        camera.yaw_gain = -20.0;
        let pose = derive_pose("frost", &target, &player, Some(&camera), "2026-10-12").unwrap();
        assert_eq!(pose.yaw_offset_deg, Some(-25.3));
    }

    #[test]
    fn without_a_camera_the_camera_fields_are_none() {
        let (target, player, _) = fs_p3();
        let pose = derive_pose("frost", &target, &player, None, "2026-10-12").unwrap();
        assert_eq!(pose.pitch_deg, None);
        assert_eq!(pose.yaw_offset_deg, None);
        assert_eq!(pose.zoom, None);
        assert_eq!(pose.radius_m, 4.16);
    }

    #[test]
    fn a_round_trip_through_stand_point_lands_on_the_player() {
        let (target, player, camera) = fs_p3();
        let pose = derive_pose("frost", &target, &player, Some(&camera), "2026-10-12").unwrap();
        let stand = pose.stand_point(target.pos);
        assert!(dist(stand, player.pos) < 0.01, "stand {stand:?}");
    }

    #[test]
    fn a_far_click_is_radius_out_of_range() {
        let (target, mut player, camera) = fs_p3();
        player.pos[0] = target.pos[0] + 9.3;
        player.pos[2] = target.pos[2];
        let err = derive_pose("frost", &target, &player, Some(&camera), "2026-10-12").unwrap_err();
        assert!(
            matches!(err, PoseError::RadiusOutOfRange(r) if (r - 9.3).abs() < 1e-9),
            "{err:?}"
        );
    }

    /// `pose` with the player moved to `radius` metres along +x of the target.
    fn at_radius(radius: f64) -> Result<PoseSpec, PoseError> {
        let (target, mut player, camera) = fs_p3();
        player.pos[0] = target.pos[0] + radius;
        player.pos[2] = target.pos[2];
        derive_pose("frost", &target, &player, Some(&camera), "2026-10-12")
    }

    #[test]
    fn the_radius_range_is_one_to_eight_metres_inclusive() {
        assert!(matches!(
            at_radius(0.5),
            Err(PoseError::RadiusOutOfRange(_))
        ));
        assert_eq!(at_radius(1.0).unwrap().radius_m, 1.0);
        assert_eq!(at_radius(8.0).unwrap().radius_m, 8.0);
    }

    #[test]
    fn a_bearing_that_rounds_to_360_is_written_as_zero() {
        let (target, mut player, camera) = fs_p3();
        player.pos[0] = target.pos[0] + 4.0;
        player.pos[2] = target.pos[2] - 1e-4;
        let pose = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap();
        assert_eq!(pose.bearing_deg, 0.0);
    }

    #[test]
    fn a_missing_or_unloadable_tag_is_no_tag() {
        let (mut target, player, camera) = fs_p3();
        for tag in [None, Some(""), Some("Frost Body"), Some("a\"b")] {
            target.tag = tag.map(str::to_string);
            let err = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap_err();
            assert_eq!(err, PoseError::NoTag, "{tag:?}");
        }
    }

    #[test]
    fn non_finite_positions_and_cameras_are_errors() {
        let (target, mut player, mut camera) = fs_p3();
        camera.zoom = f64::NAN;
        let err = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap_err();
        assert_eq!(err, PoseError::NoCamera);
        player.pos[1] = f64::INFINITY;
        let err = derive_pose("frost", &target, &player, None, "d").unwrap_err();
        assert_eq!(err, PoseError::NoPosition);
    }

    #[test]
    fn a_player_more_than_three_metres_up_is_dy_out_of_range() {
        let (target, mut player, camera) = fs_p3();
        player.pos[1] = target.pos[1] + 3.5;
        let err = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap_err();
        assert!(
            matches!(err, PoseError::DyOutOfRange(d) if (d - 3.5).abs() < 1e-9),
            "{err:?}"
        );
    }

    /// Parse one `to_inline_toml` line back into a `PoseSpec`.
    fn reparse(line: &str) -> PoseSpec {
        let table: toml::Table = toml::from_str(&format!("p = {line}")).unwrap();
        table["p"].clone().try_into().unwrap()
    }

    #[test]
    fn a_derived_pose_survives_the_inline_toml_round_trip() {
        let (target, mut player, mut camera) = fs_p3();
        let pose = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap();
        assert_eq!(reparse(&pose.to_inline_toml()), pose);
        // Edges the emitter clamps or rounds: pitch at the readout limit, a
        // zoom past the range, a dy that rounds to -0.00.
        camera.pitch_deg = -78.75;
        camera.zoom = 900.0;
        player.pos[1] = target.pos[1] - 0.004;
        let pose = derive_pose("frost", &target, &player, Some(&camera), "d").unwrap();
        assert_eq!(pose.pitch_deg, Some(-78.7));
        assert_eq!(pose.zoom, Some(775.0));
        assert!(pose.dy_m.is_sign_positive(), "dy {}", pose.dy_m);
        assert_eq!(reparse(&pose.to_inline_toml()), pose);
    }
}
