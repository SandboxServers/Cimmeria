//! Bounds layer of movement validation: the per-space AABB and the
//! finite-and-contained point check that catches NaN/infinity overflows
//! and the Z-axis floor-clip exploit.

use cimmeria_common::Vector3;

/// World-space axis-aligned bounding box for a single space.
///
/// `min` and `max` are world units; each axis of `min` must be <= the
/// same axis of `max`. The constructor does not assert this — callers
/// supplying navmesh `bmin`/`bmax` already get a well-formed pair from
/// the XRC reader, and a malformed fallback would reject every position
/// equally rather than corrupt state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl SpaceBounds {
    /// Construct a bounds AABB from explicit min/max corners.
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Generous fallback used for spaces whose navmesh failed to load.
    ///
    /// **Rationale.** This 20 km × 12 km × 20 km AABB exists for spaces
    /// that have no canonical bounds source today — legacy zones whose
    /// navmesh extractor hasn't been run, dev maps without authored
    /// `spaces.xml` bounds, and any space that ships before its navmesh
    /// is wired. Per the design doc, do **not** use this as the primary
    /// source; it is the safety net for navmesh-less spaces.
    ///
    /// **Permissive by design.** Wider than any legitimate world by an
    /// order of magnitude. The cost of a false-positive snapback (a
    /// legitimate exploring player getting yanked back to spawn) is
    /// vastly worse for the player experience than the cost of letting
    /// a cheater roam within a 20 km box — the cheater is already
    /// inside the bounds layer's blind spot regardless of how tight we
    /// crank it, and the *next* validator layers (speed, teleport
    /// detection, navmesh containment) are what actually constrain
    /// where a cheater can move. The bounds layer's job is to catch
    /// absurd values (NaN, infinity, 1e9 coordinate overflows), not to
    /// be the gate against in-bounds cheating.
    ///
    /// **Tightening path.** When a previously navmesh-less zone gets
    /// its navmesh wired, the per-space navmesh `bmin`/`bmax` becomes
    /// the source automatically (see `apply_client_position_update` in
    /// `crates/services/src/cell/space_manager/entities.rs`). A later
    /// validator layer narrows the in-bounds-cheating window by
    /// recording authorized server-side teleports (ring transport,
    /// respawn, content-engine teleport) via a `note_authorized_teleport`
    /// hook so the teleport-detection layer can distinguish legitimate
    /// large jumps from tampered ones — see
    /// `.claude/agent-memory/movement-teleport-advisor/authorized-teleport-paths.md`.
    pub const FALLBACK: Self = Self {
        min: [-10_000.0, -2_000.0, -10_000.0],
        max: [10_000.0, 10_000.0, 10_000.0],
    };
}

/// Validate that `pos` lies inside the inclusive `[min, max]` AABB.
///
/// Checks all three axes (X, Y, **and Z**). Z omission is the
/// floor-clip exploit pattern — a position update with a valid X/Y but
/// a Z far below the terrain lets a tampered client warp through the
/// world floor — and the bounds check is the only layer in PR1 that
/// catches it.
///
/// Returns `true` if the point is contained; `false` if any axis is
/// out of range. Non-finite coordinates (`NaN`, `+Infinity`,
/// `-Infinity`) are rejected up front via `is_finite()`. Relying on
/// the bounds comparisons alone is fragile — `NaN` happens to fail
/// every `>=`/`<=` (because NaN comparisons are always false), and
/// `±Infinity` happens to fail one side of a finite AABB, but a
/// future contributor widening the fallback toward `f32::MAX` /
/// `f32::MIN` could silently let infinities through. The explicit
/// gate is cheap and removes that footgun.
pub fn position_within_bounds(pos: Vector3, bounds: &SpaceBounds) -> bool {
    if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
        return false;
    }
    pos.x >= bounds.min[0]
        && pos.x <= bounds.max[0]
        && pos.y >= bounds.min[1]
        && pos.y <= bounds.max[1]
        && pos.z >= bounds.min[2]
        && pos.z <= bounds.max[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_bounds() -> SpaceBounds {
        SpaceBounds::new([-100.0, -50.0, -100.0], [100.0, 200.0, 100.0])
    }

    #[test]
    fn position_at_origin_is_within_unit_bounds() {
        assert!(position_within_bounds(Vector3::zero(), &unit_bounds()));
    }

    #[test]
    fn position_on_min_corner_is_within_inclusive_bounds() {
        // Inclusive bounds — exact min corner accepted, off-by-one
        // floats at level seams should not snap back.
        let pos = Vector3::new(-100.0, -50.0, -100.0);
        assert!(position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_on_max_corner_is_within_inclusive_bounds() {
        let pos = Vector3::new(100.0, 200.0, 100.0);
        assert!(position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_outside_x_min_rejected() {
        let pos = Vector3::new(-100.1, 0.0, 0.0);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_outside_x_max_rejected() {
        let pos = Vector3::new(100.1, 0.0, 0.0);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_outside_y_min_rejected() {
        let pos = Vector3::new(0.0, -50.1, 0.0);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_outside_y_max_rejected() {
        let pos = Vector3::new(0.0, 200.1, 0.0);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    /// The Z-axis check is the documented floor-clip exploit guard.
    /// A position with valid X/Y but Z below the floor lets a tampered
    /// client warp under the terrain; PR1 catches it via the bounds
    /// layer alone (PR4 will add the navmesh height check).
    #[test]
    fn position_outside_z_min_rejected() {
        let pos = Vector3::new(0.0, 0.0, -100.1);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn position_outside_z_max_rejected() {
        let pos = Vector3::new(0.0, 0.0, 100.1);
        assert!(!position_within_bounds(pos, &unit_bounds()));
    }

    #[test]
    fn nan_position_rejected_on_every_axis() {
        // Every comparison against NaN is false; the `is_finite()`
        // gate up front catches it explicitly regardless of which axis
        // carries the NaN.
        let nan = f32::NAN;
        assert!(!position_within_bounds(
            Vector3::new(nan, 0.0, 0.0),
            &unit_bounds()
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, nan, 0.0),
            &unit_bounds()
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, 0.0, nan),
            &unit_bounds()
        ));
    }

    /// Regression guard: `+f32::INFINITY` on any axis must be rejected
    /// **even against an unbounded-max AABB**.
    ///
    /// The shape of the regression we're guarding: a future
    /// contributor replaces the `is_finite()` gate with a NaN-only
    /// check. Against a finite-max AABB (like `unit_bounds`), the
    /// rejection still works incidentally — `+Inf <= 100.0` is false,
    /// so the conjunction collapses. But against a wide-open AABB
    /// (e.g. `max = f32::INFINITY`, which a contributor widening
    /// `SpaceBounds::FALLBACK` toward "infinitely permissive" might
    /// reach for), `+Inf <= +Inf` is true and infinity slips through.
    /// Pinning the test against an inf-max AABB fires on revert of
    /// `is_finite()` → `is_nan()`.
    #[test]
    fn position_with_infinity_axis_rejected() {
        let inf = f32::INFINITY;
        let neg_inf = f32::NEG_INFINITY;
        // Wide-open AABB: every comparison against +Inf coordinates
        // succeeds on the max side (`+Inf <= +Inf`). The only thing
        // that can reject `+Inf` here is the explicit `is_finite()`
        // gate up front.
        let unbounded = SpaceBounds::new([neg_inf, neg_inf, neg_inf], [inf, inf, inf]);
        assert!(!position_within_bounds(
            Vector3::new(inf, 0.0, 0.0),
            &unbounded
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, inf, 0.0),
            &unbounded
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, 0.0, inf),
            &unbounded
        ));
    }

    /// Symmetric regression guard for the `-Infinity` side of the
    /// `is_finite()` gate, tested against a wide-open AABB so that
    /// the explicit `is_finite()` check is the only thing rejecting
    /// the position (see `position_with_infinity_axis_rejected` for
    /// the full rationale).
    #[test]
    fn position_with_neg_infinity_axis_rejected() {
        let inf = f32::INFINITY;
        let neg_inf = f32::NEG_INFINITY;
        let unbounded = SpaceBounds::new([neg_inf, neg_inf, neg_inf], [inf, inf, inf]);
        assert!(!position_within_bounds(
            Vector3::new(neg_inf, 0.0, 0.0),
            &unbounded
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, neg_inf, 0.0),
            &unbounded
        ));
        assert!(!position_within_bounds(
            Vector3::new(0.0, 0.0, neg_inf),
            &unbounded
        ));
    }

    #[test]
    fn fallback_bounds_accept_typical_castle_cellblock_position() {
        // Castle_CellBlock spaces.xml AABB is [-800, 0, -800] to
        // [800, ?, 800] (Y bounds aren't in spaces.xml but legitimate
        // positions are far inside the fallback). Pin that the
        // fallback covers any plausible legitimate position so a
        // missing-navmesh space doesn't snap-fest legitimate players.
        let pos = Vector3::new(500.0, 50.0, -500.0);
        assert!(position_within_bounds(pos, &SpaceBounds::FALLBACK));
    }

    #[test]
    fn fallback_bounds_reject_absurd_overflow_position() {
        // 1e9 from a tampered client must still snap; the fallback is
        // generous but not unbounded.
        let pos = Vector3::new(1.0e9, 0.0, 0.0);
        assert!(!position_within_bounds(pos, &SpaceBounds::FALLBACK));
    }
}
