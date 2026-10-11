//! The candidate poses a calibration sweep tries, in D-SV6 tie-break order.

use crate::uat::spec::PoseSpec;

/// The sweep's axes. Candidates are enumerated in this field order.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// Tried in this order (D-SV6 tie-break follows it).
    pub radii_m: Vec<f64>,
    /// Positive and divides 360; 30 gives 12 bearings. A step that is not
    /// finite, not positive or above 360 gives no grid bearings, so only the
    /// current pose is tried.
    pub bearing_step_deg: f64,
    /// Yaw after facing, degrees, tried in this order.
    pub yaw_offsets_deg: Vec<f64>,
    /// None = keep the pitch the face lands on (and record it).
    pub pitches_deg: Vec<Option<f64>>,
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            radii_m: vec![3.75, 3.0, 4.5],
            bearing_step_deg: 30.0,
            yaw_offsets_deg: vec![25.0, -25.0, 0.0],
            pitches_deg: vec![None, Some(-35.0), Some(-22.0)],
        }
    }
}

/// One pose to try: where to stand, and how to aim from there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    /// Horizontal stand-off from the target, metres (`PoseSpec::radius_m`).
    pub radius_m: f64,
    /// From the target, server x/z plane, 0 = +x (`PoseSpec::bearing_deg`).
    pub bearing_deg: f64,
    /// Yaw after facing, degrees (`PoseSpec::yaw_offset_deg`).
    pub yaw_offset_deg: f64,
    /// Absolute pitch after facing; None keeps the pitch the face lands on.
    pub pitch_deg: Option<f64>,
}

/// The finest bearing step a caller may ask for: 360 bearings a radius.
pub const MIN_BEARING_STEP_DEG: f64 = 1.0;
/// The most candidates one pose sweeps, the current pose included. The
/// default grid is 325; a caller's grid over this is refused, and
/// [`candidates`] stops here whatever it is given.
pub const MAX_CANDIDATES: usize = 2000;

/// How many grid candidates `grid` gives, without building them (the
/// current pose, when there is one, adds one more).
pub fn count(grid: &Grid) -> usize {
    bearing_count(grid.bearing_step_deg)
        .saturating_mul(grid.radii_m.len())
        .saturating_mul(grid.yaw_offsets_deg.len())
        .saturating_mul(grid.pitches_deg.len())
}

/// The current pose (when it has one) first, then radius-major, bearing
/// ascending from the current pose's bearing rounded to the step (else 0),
/// then yaw, then pitch. Deterministic: the same inputs give the same list.
/// At most [`MAX_CANDIDATES`]; the rest of a larger grid is never built.
pub fn candidates(current: Option<&PoseSpec>, grid: &Grid) -> Vec<Candidate> {
    let mut out = Vec::with_capacity((count(grid) + 1).min(MAX_CANDIDATES));
    if let Some(p) = current {
        out.push(Candidate {
            radius_m: p.radius_m,
            bearing_deg: p.bearing_deg,
            yaw_offset_deg: p.yaw_offset_deg.unwrap_or(0.0),
            pitch_deg: p.pitch_deg,
        });
    }
    let bearings = bearings(current.map(|p| p.bearing_deg), grid.bearing_step_deg);
    for &radius_m in &grid.radii_m {
        for &bearing_deg in &bearings {
            for &yaw_offset_deg in &grid.yaw_offsets_deg {
                for &pitch_deg in &grid.pitches_deg {
                    if out.len() >= MAX_CANDIDATES {
                        return out;
                    }
                    out.push(Candidate {
                        radius_m,
                        bearing_deg,
                        yaw_offset_deg,
                        pitch_deg,
                    });
                }
            }
        }
    }
    out
}

/// One bearing per step of a full turn, starting at `from` rounded to the
/// step (0 when there is none) and wrapping. Integer indices keep it exact.
/// A step that is not finite, not positive or above 360 gives none: the
/// `360 / step` count would be 0 or saturate, and `% n` would panic.
fn bearings(from: Option<f64>, step: f64) -> Vec<f64> {
    let n = bearing_count(step);
    if n == 0 {
        return Vec::new();
    }
    // rem_euclid so a negative bearing wraps (-30 starts at 330) instead of
    // saturating to index 0.
    let first = from.map_or(0, |b| {
        ((b / step).round() as i64).rem_euclid(n as i64) as usize
    });
    (0..n).map(|i| ((first + i) % n) as f64 * step).collect()
}

/// Bearings a step gives a radius; 0 for a step [`bearings`] refuses.
fn bearing_count(step: f64) -> usize {
    if !(step.is_finite() && step > 0.0 && step <= 360.0) {
        return 0;
    }
    ((360.0 / step).round() as usize).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::spec::AimMode;

    /// The contract's frost pose, at the given bearing.
    fn pose_at(bearing_deg: f64) -> PoseSpec {
        PoseSpec {
            id: "frost".into(),
            tag: "ArmYourself_FrostBody".into(),
            radius_m: 4.16,
            bearing_deg,
            dy_m: 0.13,
            pitch_deg: Some(-28.8),
            yaw_offset_deg: Some(25.3),
            zoom: Some(250.0),
            aim: AimMode::Entity,
            aim_dy_m: None,
            calibrated: None,
        }
    }

    #[test]
    fn default_grid_has_324_candidates_in_a_fixed_order() {
        let grid = Grid::default();
        let all = candidates(None, &grid);
        assert_eq!(all.len(), 3 * 12 * 3 * 3);
        assert_eq!(
            all[0],
            Candidate {
                radius_m: 3.75,
                bearing_deg: 0.0,
                yaw_offset_deg: 25.0,
                pitch_deg: None,
            }
        );
        // Nesting is radius > bearing > yaw > pitch, so D-SV6 ties go to the
        // earlier radius first, then bearing, then yaw, then pitch.
        let at = |radius_m, bearing_deg, yaw_offset_deg, pitch_deg| Candidate {
            radius_m,
            bearing_deg,
            yaw_offset_deg,
            pitch_deg,
        };
        assert_eq!(all[1], at(3.75, 0.0, 25.0, Some(-35.0)), "second pitch");
        assert_eq!(all[3], at(3.75, 0.0, -25.0, None), "second yaw");
        assert_eq!(all[9], at(3.75, 30.0, 25.0, None), "next bearing");
        assert_eq!(all[9 * 12], at(3.0, 0.0, 25.0, None), "second radius");
        assert_eq!(all, candidates(None, &grid));
        // With a current pose, it leads and the same grid follows, its
        // bearings rotated to start at the pose's (see the next test).
        let current = pose_at(322.5);
        let with = candidates(Some(&current), &grid);
        assert_eq!(with.len(), all.len() + 1);
        assert_eq!(
            with[0],
            Candidate {
                radius_m: 4.16,
                bearing_deg: 322.5,
                yaw_offset_deg: 25.3,
                pitch_deg: Some(-28.8),
            }
        );
        let key = |c: &Candidate| format!("{c:?}");
        let mut rest: Vec<String> = with[1..].iter().map(key).collect();
        let mut grid_only: Vec<String> = all.iter().map(key).collect();
        rest.sort();
        grid_only.sort();
        assert_eq!(rest, grid_only);
        assert_eq!(with, candidates(Some(&current), &grid));
    }

    #[test]
    fn bearings_start_at_the_current_pose() {
        let grid = Grid::default();
        let with = candidates(Some(&pose_at(322.5)), &grid);
        // One radius, yaw and pitch at a time: the bearings of the first 12
        // grid candidates, each once, from 322.5 rounded up to 330.0.
        let sweep: Vec<f64> = with[1..]
            .iter()
            .filter(|c| c.radius_m == 3.75 && c.yaw_offset_deg == 25.0 && c.pitch_deg.is_none())
            .map(|c| c.bearing_deg)
            .collect();
        assert_eq!(
            sweep,
            [330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0]
        );
    }

    #[test]
    fn a_bad_bearing_step_gives_no_grid_bearings_instead_of_panicking() {
        for step in [0.0, -30.0, 361.0, 720.0, f64::NAN, f64::INFINITY] {
            let grid = Grid {
                bearing_step_deg: step,
                ..Grid::default()
            };
            assert!(candidates(None, &grid).is_empty(), "step {step}");
            // The current pose is still tried on its own.
            assert_eq!(candidates(Some(&pose_at(10.0)), &grid).len(), 1);
        }
        // A whole turn is one bearing; a negative bearing wraps.
        assert_eq!(bearings(Some(90.0), 360.0), [0.0]);
        assert_eq!(bearings(Some(-30.0), 90.0), [0.0, 90.0, 180.0, 270.0]);
        assert_eq!(bearings(Some(-100.0), 90.0), [270.0, 0.0, 90.0, 180.0]);
    }

    #[test]
    fn count_matches_the_list_and_a_huge_grid_stops_at_the_cap() {
        let grid = Grid::default();
        assert_eq!(count(&grid), candidates(None, &grid).len());
        // A step that would give a billion bearings is counted, not built.
        let tiny = Grid {
            bearing_step_deg: 1e-6,
            ..Grid::default()
        };
        assert!(count(&tiny) > MAX_CANDIDATES);
        let all = candidates(Some(&pose_at(10.0)), &tiny);
        assert_eq!(all.len(), MAX_CANDIDATES);
        assert_eq!(all[0].bearing_deg, 10.0, "the current pose still leads");
    }
}
