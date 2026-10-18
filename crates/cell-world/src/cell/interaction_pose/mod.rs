//! `interaction.pose`: where a player stood, and which way they faced, when an
//! interaction succeeded. Feeds the lab's stand-off poses (D-SV3) from real play.
//! Campaign: docs/analysis/player-capture-bots/.
mod geometry;
#[cfg(test)]
mod geometry_tests;

pub use geometry::{
    pose_geometry, wrap_deg_180, wrap_deg_360, InteractionKind, InteractionRoute, PoseGeometry,
};
