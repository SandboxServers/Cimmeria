//! The hostile NPCs standing near a player at a successful interaction, for the
//! `interaction_hostiles` row on the `interaction.pose` target. Read-only apart
//! from the one row; no world state changes.
//!
//! Only ids, distances and bearings are written. No names, no chat, no cursor data.

use super::capture::InteractionPose;
use super::geometry::pose_geometry;
use crate::cell::combat::is_hostile_to_players;
use crate::cell::space_manager::SpaceManager;
use cimmeria_wire::state_field::BSF_DEAD;

/// Radius around the player within which hostiles are listed, in metres.
pub const HOSTILE_CONTEXT_RADIUS_M: f32 = 20.0;

/// The most hostiles one `interaction_hostiles` row lists, nearest first.
pub const HOSTILE_CONTEXT_MAX: usize = 3;

/// One living hostile NPC near the acting player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearbyHostile {
    /// The hostile's entity id.
    pub entity_id: u32,
    /// Horizontal distance from the player to the hostile, in metres.
    pub dist_m: f32,
    /// `atan2(pz - tz, px - tx)` from the hostile to the player, in degrees, in `[0, 360)`.
    pub bearing_deg: f32,
}

/// Lists up to [`HOSTILE_CONTEXT_MAX`] living hostiles within
/// [`HOSTILE_CONTEXT_RADIUS_M`] of the player, nearest first. The interaction
/// target is never listed, and neither are dead NPCs or non-hostile ones.
pub fn nearby_hostiles(mgr: &SpaceManager, pose: &InteractionPose) -> Vec<NearbyHostile> {
    let player_yaw_rad = mgr
        .get_entity(pose.entity_id)
        .map_or(0.0, |e| e.direction.y);
    let mut found: Vec<NearbyHostile> = mgr
        .npc_ids_near(pose.entity_id, HOSTILE_CONTEXT_RADIUS_M)
        .into_iter()
        .filter(|&id| id != pose.target_entity_id)
        .filter_map(|id| {
            let npc = mgr.get_entity(id)?;
            if npc.state_field & BSF_DEAD != 0 || !is_hostile_to_players(npc) {
                return None;
            }
            let npc_pos = [npc.position.x, npc.position.y, npc.position.z];
            let geometry = pose_geometry(pose.player_pos, player_yaw_rad, npc_pos);
            Some(NearbyHostile {
                entity_id: id,
                dist_m: geometry.radius_m,
                bearing_deg: geometry.bearing_deg,
            })
        })
        .collect();
    found.sort_by(|a, b| a.dist_m.total_cmp(&b.dist_m));
    found.truncate(HOSTILE_CONTEXT_MAX);
    found
}

/// Writes one `interaction_hostiles` row for the nearby hostiles of `pose`,
/// keyed by the pose row's `pose_seq`. No row when no hostile is near.
pub fn emit_nearby_hostiles(mgr: &SpaceManager, pose: &InteractionPose, pose_seq: u64) {
    let hostiles = nearby_hostiles(mgr, pose);
    let Some(first) = hostiles.first() else {
        return;
    };
    let second = hostiles.get(1);
    let third = hostiles.get(2);
    tracing::info!(
        target: "interaction.pose",
        event = "interaction_hostiles",
        entity_id = pose.entity_id, // nt:id-only privacy: no personal names (D-PC2)
        pose_seq = pose_seq,
        count = hostiles.len(),
        hostile_1_id = first.entity_id, // nt:id-only joins the entity snapshot by id; row stays name-free (D-PC2)
        hostile_1_dist_m = first.dist_m,
        hostile_1_bearing_deg = first.bearing_deg,
        hostile_2_id = second.map(|h| h.entity_id), // nt:id-only joins the entity snapshot by id; row stays name-free (D-PC2)
        hostile_2_dist_m = second.map(|h| h.dist_m),
        hostile_2_bearing_deg = second.map(|h| h.bearing_deg),
        hostile_3_id = third.map(|h| h.entity_id), // nt:id-only joins the entity snapshot by id; row stays name-free (D-PC2)
        hostile_3_dist_m = third.map(|h| h.dist_m),
        hostile_3_bearing_deg = third.map(|h| h.bearing_deg),
        "interaction_hostiles: living hostiles near the player at an interaction"
    );
}
