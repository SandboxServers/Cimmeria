//! Pure geometry for an `interaction.pose` row. No world state, no logging.
//!
//! Conventions: server yaw is `atan2(dx, dz)`, measured from +z toward +x, in
//! radians on the entity and in degrees on the row. Bearing (D-SV3) is
//! `atan2(pz - tz, px - tx)`, wrapped to `[0, 360)`. A positive facing error
//! means the target is clockwise of the heading, seen from above with +y up.

/// Which interaction the player performed. Label: `"interact"` or `"use_item"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    /// A plain world interaction (click, use object).
    Interact,
    /// Using an item on a target.
    UseItem,
}

impl InteractionKind {
    /// The stable label written to the row's `kind` field.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Interact => "interact",
            Self::UseItem => "use_item",
        }
    }
}

/// How the interaction target was resolved. Label is one of `"trainer"`,
/// `"chain_tag"`, `"chain_template"`, `"fallback"`, `"item_used"` or
/// `"hostile_attack"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionRoute {
    /// Resolved through a trainer.
    Trainer,
    /// Resolved through a chain keyed by tag.
    ChainTag,
    /// Resolved through a chain keyed by template.
    ChainTemplate,
    /// Resolved through the generic fallback.
    Fallback,
    /// Resolved as an item use.
    ItemUsed,
    /// A right-click on a living hostile NPC, rerouted to combat.
    HostileAttack,
}

impl InteractionRoute {
    /// The stable label written to the row's `route` field.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trainer => "trainer",
            Self::ChainTag => "chain_tag",
            Self::ChainTemplate => "chain_template",
            Self::Fallback => "fallback",
            Self::ItemUsed => "item_used",
            Self::HostileAttack => "hostile_attack",
        }
    }
}

/// Geometry of a player relative to an interaction target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseGeometry {
    /// Horizontal distance from the player to the target, in metres.
    pub radius_m: f32,
    /// Full distance including height, in metres. Never emitted on the row.
    pub dist_3d_m: f32,
    /// `atan2(pz - tz, px - tx)` in degrees, in `[0, 360)` (D-SV3).
    pub bearing_deg: f32,
    /// Player y minus target y, in metres.
    pub dy_m: f32,
    /// Heading the player would need to face the target, in `[0, 360)`.
    /// Never emitted on the row.
    pub yaw_to_target_deg: f32,
    /// Signed angle from the player's heading to the target, in `(-180, 180]`.
    /// Positive means the target is clockwise of the heading.
    pub facing_error_deg: f32,
}

/// Wraps an angle in degrees into `[0, 360)`.
pub fn wrap_deg_360(deg: f32) -> f32 {
    let w = deg.rem_euclid(360.0);
    if w >= 360.0 {
        0.0
    } else {
        w
    } // rem_euclid can round up to 360.0
}

/// Wraps an angle in degrees into `(-180, 180]`.
pub fn wrap_deg_180(deg: f32) -> f32 {
    let w = wrap_deg_360(deg);
    if w > 180.0 {
        w - 360.0
    } else {
        w
    }
}

/// Computes the geometry of `player` facing `player_yaw_rad` relative to `target`.
/// Both positions are world coordinates `[x, y, z]`; the yaw is the server yaw in radians.
pub fn pose_geometry(player: [f32; 3], player_yaw_rad: f32, target: [f32; 3]) -> PoseGeometry {
    let (dx, dy, dz) = (
        player[0] - target[0],
        player[1] - target[1],
        player[2] - target[2],
    );
    let radius_m = (dx * dx + dz * dz).sqrt();
    let yaw_to_target_deg = wrap_deg_360((-dx).atan2(-dz).to_degrees()); // atan2(tx-px, tz-pz)
    PoseGeometry {
        radius_m,
        dist_3d_m: (dx * dx + dy * dy + dz * dz).sqrt(),
        bearing_deg: wrap_deg_360(dz.atan2(dx).to_degrees()), // D-SV3
        dy_m: dy,
        yaw_to_target_deg,
        facing_error_deg: wrap_deg_180(yaw_to_target_deg - player_yaw_rad.to_degrees()),
    }
}
