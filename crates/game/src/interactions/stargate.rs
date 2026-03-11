use serde::{Deserialize, Serialize};

use cimmeria_common::types::SpaceId;

/// A stargate connection between two worlds.
///
/// Corresponds to the stargates table in the database schema.
/// Each stargate links a source space to a destination space with
/// a gate address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StargateLink {
    pub gate_id: i32,
    pub source_space_id: SpaceId,
    pub destination_space_id: SpaceId,
    pub gate_address: String,
    pub destination_x: f32,
    pub destination_y: f32,
    pub destination_z: f32,
}

/// Initiate a stargate dial sequence for a player.
///
/// Validates the gate address, checks if the destination is available,
/// and begins the travel transition.
pub fn dial_stargate(_player_entity_id: i32, _gate_id: i32) -> GateResult {
    todo!("dial_stargate - validate address, begin travel sequence")
}

/// Complete the gate travel, moving the player entity to the destination space.
pub fn complete_gate_travel(_player_entity_id: i32, _link: &StargateLink) -> GateResult {
    todo!("complete_gate_travel - teleport player to destination space + position")
}

/// Handle ring transport (short-range teleport within a space).
pub fn ring_transport(_player_entity_id: i32, _region_id: i32) -> GateResult {
    todo!("ring_transport - move player to paired ring platform")
}

/// Result of a gate/transport operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Success,
    InvalidAddress,
    DestinationUnavailable,
    AlreadyInTransit,
    GateLocked,
}
