//! The *why* behind a navmesh containment decision.
//!
//! [`super::NavMesh::is_point_valid`] answers a bool, which is all the
//! movement validator needs to decide whether to snap a client back. It is
//! not enough to decide whether the **mesh** is wrong.
//!
//! Three days of production logs in September 2026 carried 146,760
//! `movement.validation_reject` rows with `reason = "navmesh"` and no
//! further detail. "Off the mesh" covers a player standing in a hole the
//! mesh builder left, a player clipped under the floor, a jump one unit
//! past the tolerance, and a player nowhere near any polygon at all —
//! four completely different bugs with four different owners.
//! [`PointVerdict`] names which one fired, and how far off the point was,
//! so a SigNoz query grouped by `gate` separates them without a repro.

/// Which containment gate rejected a point.
///
/// Ordered as the checks run in
/// [`super::NavMesh::classify_containment`]: a point can fail more than
/// one of these, and the first one that fires is the one reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavGate {
    /// Neither search phase found any polygon inside its search box. The
    /// point is not merely off the walkable surface, it is nowhere near
    /// the mesh — a mesh hole, a room the builder never covered, or a
    /// position in a completely different part of the world.
    NoPolyInExtents,
    /// A polygon was found, but the point is further than
    /// `agent_radius * 2` from it horizontally — off the edge of the
    /// walkable surface.
    Horizontal,
    /// Within the horizontal gate, but more than `agent_radius * 2`
    /// *below* the surface. Floor-clip / under-terrain.
    BelowSurface,
    /// Within the horizontal gate, but more than
    /// [`super::JUMP_HEIGHT_TOLERANCE`] *above* the surface. Higher than
    /// the client's own jump physics can produce.
    AboveJumpTolerance,
}

impl NavGate {
    /// Stable low-cardinality token for logs and metric labels. Treat
    /// these as API: a SigNoz saved view and the
    /// `movement_validation_rejects_total{gate}` counter both pin them.
    pub fn label(self) -> &'static str {
        match self {
            NavGate::NoPolyInExtents => "no_poly_in_extents",
            NavGate::Horizontal => "horizontal",
            NavGate::BelowSurface => "below_surface",
            NavGate::AboveJumpTolerance => "above_jump_tolerance",
        }
    }
}

/// The full result of a containment test: the same boolean
/// [`super::NavMesh::is_point_valid`] returns, plus the reason and the
/// distances behind it.
///
/// `horizontal_dist` and `dy` are measured against the nearest polygon
/// point either search phase found, and are `None` exactly when
/// [`NavGate::NoPolyInExtents`] fired (there was no polygon to measure
/// against). When `valid` is `true` they are still populated — the
/// accepted-position sampler reports `dy` as the player's height above
/// the walkable surface.
#[derive(Debug, Clone, Copy)]
pub struct PointVerdict {
    /// Exactly what `is_point_valid` returns for this point.
    pub valid: bool,
    /// `None` when `valid`; otherwise the first gate that failed.
    pub gate: Option<NavGate>,
    /// X/Z distance from the point to the nearest polygon point.
    pub horizontal_dist: Option<f32>,
    /// Signed `pos.y - closest.y`. Positive = above the surface.
    pub dy: Option<f32>,
}

impl PointVerdict {
    /// A point with no polygon anywhere near it.
    pub(super) fn no_poly() -> Self {
        Self {
            valid: false,
            gate: Some(NavGate::NoPolyInExtents),
            horizontal_dist: None,
            dy: None,
        }
    }

    /// Stable token for the gate, or `None` when the point was accepted.
    /// Convenience so callers can hand `Option<&'static str>` straight to
    /// `tracing` (which omits the field entirely for `None`).
    pub fn gate_label(&self) -> Option<&'static str> {
        self.gate.map(NavGate::label)
    }
}
