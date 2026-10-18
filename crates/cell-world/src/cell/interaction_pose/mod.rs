//! `interaction.pose`: where a player stood, and which way they faced, when an
//! interaction succeeded. Feeds the lab's stand-off poses (D-SV3) from real play.
//! Campaign: docs/analysis/player-capture-bots/.
#[cfg(test)]
mod approach_emit_tests;
mod capture;
mod emit;
#[cfg(test)]
mod emit_tests;
mod geometry;
#[cfg(test)]
mod geometry_tests;

pub use capture::{capture_interaction_pose, InteractionPose};
pub use emit::{emit_interaction_pose, emit_interaction_pose_at, PoseExtra, POSE_MIN_INTERVAL};
pub use geometry::{
    pose_geometry, wrap_deg_180, wrap_deg_360, InteractionKind, InteractionRoute, PoseGeometry,
};
