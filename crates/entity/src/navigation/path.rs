//! Pathfinding: `NavMesh::find_path` and its typed [`PathOutcome`].
//!
//! Before NA02 `find_path` returned `Option<Vec<Vector3>>`, which could not
//! say *which* Detour stage declined (audit gap T6) and never reported a
//! partial corridor at all (audit S8). Detour's `findPath` returns
//! `DT_SUCCESS | DT_PARTIAL_RESULT` when the goal polygon is on another mesh
//! island; the old FFI checked only the failure bit, so an NPC chasing a
//! player on another island walked to the island edge and repathed forever,
//! logged as a plain `chase`.
//!
//! The outcome is purely descriptive. [`PathOutcome::into_waypoints`] yields
//! exactly what the old function returned for every status — including a
//! partial path and the two-point straighten-failure fallback — so a caller
//! that switches to it changes no behaviour.

use cimmeria_common::Vector3;

use super::{NavMesh, DEST_EXTENTS, MAX_POLY_PATH, MAX_STRAIGHT_PATH, START_EXTENTS};
use crate::detour_ffi::{self, dt_status_failed, dt_status_partial};

/// Which stage of a path query decided the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    /// A full corridor from the start polygon to the end polygon.
    Ok,
    /// Detour returned its best guess: the corridor stops at the polygon
    /// nearest the goal on the start's mesh island. Still returned as a
    /// usable path, exactly as before.
    Partial,
    /// No polygon within the tight `±0.5` start box (audit S9): the mover
    /// is hovering, sunk, or off the mesh.
    NoStartPoly,
    /// No polygon within the `±3.0` destination box.
    NoEndPoly,
    /// Both ends snapped, but A* produced no corridor.
    NoCorridor,
    /// A corridor exists but string-pulling failed; the path is the two
    /// snapped endpoints joined directly (the pre-existing fallback).
    StraightenFailed,
}

impl PathStatus {
    /// Stable snake_case label — a log field and a metric label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::NoStartPoly => "no_start_poly",
            Self::NoEndPoly => "no_end_poly",
            Self::NoCorridor => "no_corridor",
            Self::StraightenFailed => "straighten_failed",
        }
    }

    /// Whether the query produced waypoints a caller walks today.
    pub fn yields_path(self) -> bool {
        matches!(self, Self::Ok | Self::Partial | Self::StraightenFailed)
    }

    /// Classify a completed corridor. Pure so the decision table is
    /// unit-testable without a mesh.
    ///
    /// Partial is reported when Detour sets `DT_PARTIAL_RESULT` on either
    /// query, **or** when the corridor's last polygon is not the end
    /// polygon — the latter is how `findPath` expresses an unreachable goal
    /// even on Detour builds that do not set the flag on every path.
    pub fn classify_corridor(
        corridor_status: u32,
        corridor_len: i32,
        last_poly: u32,
        end_ref: u32,
        straight_status: Option<u32>,
        straight_len: i32,
    ) -> Self {
        if dt_status_failed(corridor_status) || corridor_len <= 0 {
            return Self::NoCorridor;
        }
        let Some(straight_status) = straight_status else {
            return Self::NoCorridor;
        };
        if dt_status_failed(straight_status) || straight_len <= 0 {
            return Self::StraightenFailed;
        }
        if dt_status_partial(corridor_status)
            || dt_status_partial(straight_status)
            || last_poly != end_ref
        {
            Self::Partial
        } else {
            Self::Ok
        }
    }
}

/// The full result of [`NavMesh::find_path`].
#[derive(Debug, Clone, PartialEq)]
pub struct PathOutcome {
    pub status: PathStatus,
    /// Waypoints, first = snapped start. Empty unless
    /// [`PathStatus::yields_path`].
    pub waypoints: Vec<Vector3>,
    /// Where the start snapped onto its polygon, when it did.
    pub start_snap: Option<Vector3>,
    /// Where the destination snapped onto its polygon, when it did.
    pub end_snap: Option<Vector3>,
}

impl PathOutcome {
    fn failed(status: PathStatus, start_snap: Option<Vector3>, end_snap: Option<Vector3>) -> Self {
        Self {
            status,
            waypoints: Vec::new(),
            start_snap,
            end_snap,
        }
    }

    /// The pre-NA02 return value: `Some(waypoints)` for every status that
    /// produced a path (including [`PathStatus::Partial`] and
    /// [`PathStatus::StraightenFailed`]), `None` otherwise.
    pub fn into_waypoints(self) -> Option<Vec<Vector3>> {
        self.status.yields_path().then_some(self.waypoints)
    }

    /// Vertical distance the start moved to reach its polygon (`snap.y -
    /// requested.y`). A large magnitude means the mover was floating or sunk.
    pub fn start_snap_dy(&self, requested: &Vector3) -> Option<f32> {
        self.start_snap.map(|s| s.y - requested.y)
    }

    /// 3D distance the destination moved to reach its polygon.
    pub fn end_snap_dist(&self, requested: &Vector3) -> Option<f32> {
        self.end_snap.map(|s| s.distance_to(requested))
    }
}

impl NavMesh {
    /// Find a path from `start` to `end` across the navigation mesh.
    ///
    /// Uses Detour's A* followed by straight-path simplification and reports
    /// which stage decided the result — see [`PathStatus`]. Callers that only
    /// want the old `Option<Vec<Vector3>>` use
    /// [`PathOutcome::into_waypoints`].
    ///
    /// Nothing here logs: the two unthrottled `no start/end poly` warnings
    /// this replaced carried no entity id and fired on every AI tick of a
    /// stuck NPC. The NPC AI logs the outcome once, with the NPC's identity,
    /// under `npc_ai.path`.
    pub fn find_path(&self, start: &Vector3, end: &Vector3) -> PathOutcome {
        let start_pos = [start.x, start.y, start.z];
        let end_pos = [end.x, end.y, end.z];

        // Tight extents — an entity should be standing on its polygon.
        let Some((start_ref, start_pt)) = self.nearest_poly_raw(&start_pos, &START_EXTENTS) else {
            return PathOutcome::failed(PathStatus::NoStartPoly, None, None);
        };
        let start_snap = Some(to_vec(&start_pt));

        // Loose extents — the destination may be approximate.
        let Some((end_ref, end_pt)) = self.nearest_poly_raw(&end_pos, &DEST_EXTENTS) else {
            return PathOutcome::failed(PathStatus::NoEndPoly, start_snap, None);
        };
        let end_snap = Some(to_vec(&end_pt));

        let mut poly_path = vec![0u32; MAX_POLY_PATH as usize];
        let mut path_count: i32 = 0;
        let corridor_status = unsafe {
            detour_ffi::detour_find_path(
                self.query,
                start_ref,
                end_ref,
                start_pt.as_ptr(),
                end_pt.as_ptr(),
                poly_path.as_mut_ptr(),
                &mut path_count,
                MAX_POLY_PATH,
            )
        };
        if dt_status_failed(corridor_status) || path_count <= 0 {
            return PathOutcome::failed(PathStatus::NoCorridor, start_snap, end_snap);
        }
        let last_poly = poly_path[(path_count - 1) as usize];

        let mut straight_path = vec![0.0f32; (MAX_STRAIGHT_PATH * 3) as usize];
        let mut straight_count: i32 = 0;
        let straight_status = unsafe {
            detour_ffi::detour_find_straight_path(
                self.query,
                start_pt.as_ptr(),
                end_pt.as_ptr(),
                poly_path.as_ptr(),
                path_count,
                straight_path.as_mut_ptr(),
                &mut straight_count,
                MAX_STRAIGHT_PATH,
            )
        };

        let status = PathStatus::classify_corridor(
            corridor_status,
            path_count,
            last_poly,
            end_ref,
            Some(straight_status),
            straight_count,
        );
        let waypoints = if status == PathStatus::StraightenFailed {
            // Detour found a corridor but could not straighten it. The
            // pre-existing fallback: the two snapped endpoints, direct.
            vec![to_vec(&start_pt), to_vec(&end_pt)]
        } else {
            (0..straight_count as usize)
                .map(|i| {
                    Vector3::new(
                        straight_path[i * 3],
                        straight_path[i * 3 + 1],
                        straight_path[i * 3 + 2],
                    )
                })
                .collect()
        };
        PathOutcome {
            status,
            waypoints,
            start_snap,
            end_snap,
        }
    }

    /// The point `pos` snaps to under `find_path`'s tight start box, or
    /// `None` when `find_path` from here would report
    /// [`PathStatus::NoStartPoly`]. [`NavMesh::is_point_valid`] is far
    /// looser (3.0 horizontal, 4.0 up), so a spawn can pass validation and
    /// still be unable to start any path (audit S9).
    pub fn start_poly_snap(&self, pos: &Vector3) -> Option<Vector3> {
        self.nearest_poly_raw(&[pos.x, pos.y, pos.z], &START_EXTENTS)
            .map(|(_, p)| to_vec(&p))
    }

    fn nearest_poly_raw(&self, center: &[f32; 3], extents: &[f32; 3]) -> Option<(u32, [f32; 3])> {
        let mut poly_ref: u32 = 0;
        let mut pt = [0.0f32; 3];
        let status = unsafe {
            detour_ffi::detour_find_nearest_poly(
                self.query,
                center.as_ptr(),
                extents.as_ptr(),
                &mut poly_ref,
                pt.as_mut_ptr(),
            )
        };
        (!dt_status_failed(status) && poly_ref != 0).then_some((poly_ref, pt))
    }
}

fn to_vec(p: &[f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detour_ffi::{DT_FAILURE, DT_PARTIAL_RESULT};

    const DT_SUCCESS: u32 = 1 << 30;

    #[test]
    fn classification_table() {
        let c = PathStatus::classify_corridor;
        assert_eq!(c(DT_SUCCESS, 3, 7, 7, Some(DT_SUCCESS), 4), PathStatus::Ok);
        assert_eq!(
            c(DT_SUCCESS | DT_PARTIAL_RESULT, 3, 7, 7, Some(DT_SUCCESS), 4),
            PathStatus::Partial,
            "the corridor's partial flag alone is a partial path"
        );
        assert_eq!(
            c(DT_SUCCESS, 3, 6, 7, Some(DT_SUCCESS), 4),
            PathStatus::Partial,
            "a corridor ending short of the end polygon is partial even without the flag"
        );
        assert_eq!(
            c(DT_SUCCESS, 3, 7, 7, Some(DT_SUCCESS | DT_PARTIAL_RESULT), 4),
            PathStatus::Partial
        );
        assert_eq!(c(DT_FAILURE, 0, 0, 7, None, 0), PathStatus::NoCorridor);
        assert_eq!(c(DT_SUCCESS, 0, 0, 7, None, 0), PathStatus::NoCorridor);
        assert_eq!(
            c(DT_SUCCESS, 3, 7, 7, Some(DT_FAILURE), 0),
            PathStatus::StraightenFailed
        );
        assert_eq!(
            c(DT_SUCCESS, 3, 7, 7, Some(DT_SUCCESS), 0),
            PathStatus::StraightenFailed
        );
    }

    /// `into_waypoints` must reproduce the pre-NA02 `Option` for every
    /// status — this is the "callers keep today's behaviour" contract.
    #[test]
    fn into_waypoints_preserves_the_old_option_contract() {
        let wp = vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)];
        for status in [
            PathStatus::Ok,
            PathStatus::Partial,
            PathStatus::StraightenFailed,
        ] {
            let o = PathOutcome {
                status,
                waypoints: wp.clone(),
                start_snap: None,
                end_snap: None,
            };
            assert_eq!(o.into_waypoints(), Some(wp.clone()), "{status:?}");
        }
        for status in [
            PathStatus::NoStartPoly,
            PathStatus::NoEndPoly,
            PathStatus::NoCorridor,
        ] {
            let o = PathOutcome::failed(status, None, None);
            assert_eq!(o.into_waypoints(), None, "{status:?}");
        }
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(PathStatus::Partial.label(), "partial");
        assert_eq!(PathStatus::NoStartPoly.label(), "no_start_poly");
        assert_eq!(PathStatus::NoEndPoly.label(), "no_end_poly");
        assert_eq!(PathStatus::NoCorridor.label(), "no_corridor");
        assert_eq!(PathStatus::StraightenFailed.label(), "straighten_failed");
    }
}
