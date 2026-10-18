//! Reads the actor and the target of a successful interaction into an
//! [`InteractionPose`]. Read-only: no logging and no throttle.

use super::geometry::{pose_geometry, wrap_deg_180, wrap_deg_360, PoseGeometry};
use crate::cell::space_manager::SpaceManager;

/// The `world` value when the actor's space has no loaded world name.
const UNKNOWN_WORLD: &str = "unknown";

/// The state of a player and of an interaction target, read at one successful interaction.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionPose {
    /// The acting player's entity id.
    pub entity_id: u32,
    /// The acting player's account id, if known.
    pub account_id: Option<u32>,
    /// The acting player's player id, if known.
    pub player_id: Option<i32>,
    /// The acting player's session kind, if known.
    pub session_kind: Option<&'static str>,
    /// The acting player's space instance id, if loaded.
    pub space_id: Option<u32>,
    /// The acting player's `resources.worlds` id, if stamped.
    pub world_id: Option<i32>,
    /// The acting player's world name, or `"unknown"`.
    pub world: String,
    /// The acting player's position `[x, y, z]`.
    pub player_pos: [f32; 3],
    /// The acting player's yaw in degrees, in `[0, 360)`.
    pub player_yaw_deg: f32,
    /// The acting player's pitch in degrees, in `(-180, 180]`. Meaning: PC-06.
    pub wire_pitch_deg: f32,
    /// The interaction target's entity id.
    pub target_entity_id: u32,
    /// The target's template id, if it has one.
    pub target_template_id: Option<i32>,
    /// The target's tag, if it has one.
    pub target_tag: Option<String>,
    /// The target's yaw in degrees, in `[0, 360)`.
    pub target_yaw_deg: f32,
    /// The player's geometry relative to the target.
    pub geometry: PoseGeometry,
}

/// Captures the pose of player `entity_id` relative to `target_entity_id`.
///
/// Returns `None` when both ids are equal, when either entity is missing, or
/// when the actor is not a player.
pub fn capture_interaction_pose(
    mgr: &SpaceManager,
    entity_id: u32,
    target_entity_id: u32,
) -> Option<InteractionPose> {
    if entity_id == target_entity_id {
        return None;
    }
    let actor = mgr.get_entity(entity_id)?;
    if !actor.is_player {
        return None;
    }
    let target = mgr.get_entity(target_entity_id)?;
    let id = mgr.player_identity(entity_id);
    let space_id = mgr.get_entity_space_id(entity_id);
    let player_pos = [actor.position.x, actor.position.y, actor.position.z];
    let target_pos = [target.position.x, target.position.y, target.position.z];
    Some(InteractionPose {
        entity_id,
        account_id: id.account_id,
        player_id: id.player_id,
        session_kind: actor.session_kind,
        space_id,
        world_id: mgr.get_entity_world_id(entity_id),
        world: space_id
            .and_then(|s| mgr.world_name_for_space(s))
            .unwrap_or(UNKNOWN_WORLD)
            .to_owned(),
        player_pos,
        player_yaw_deg: wrap_deg_360(actor.direction.y.to_degrees()),
        wire_pitch_deg: wrap_deg_180(actor.direction.x.to_degrees()),
        target_entity_id,
        target_template_id: target.template_id,
        target_tag: target.tag.clone(),
        target_yaw_deg: wrap_deg_360(target.direction.y.to_degrees()),
        geometry: pose_geometry(player_pos, actor.direction.y, target_pos),
    })
}
