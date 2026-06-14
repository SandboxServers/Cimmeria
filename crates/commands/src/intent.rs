//! Parsed, authorized GM command intents.
//!
//! A [`GmCommandIntent`] is the typed result of parsing + authorizing a
//! `/`-prefixed chat command on the **base** side. The base never mutates
//! world state directly: it produces one of these intents and ships it to the
//! cell, which owns the spatial simulation and all client-method sends. See
//! `crates/services/src/cell/gm_command.rs` for the executor.

use cimmeria_common::math::Vector3;

/// A parsed, authorized GM command for the cell to execute against world state.
#[derive(Debug, Clone, PartialEq)]
pub enum GmCommandIntent {
    /// Spawn `count` copies of the NPC template named `moniker` near the caller.
    Spawn { moniker: String, count: u32 },
    /// Teleport the caller to an absolute world coordinate.
    GotoCoords(Vector3),
    /// Teleport the caller to the named player (same space).
    GotoPlayer(String),
    /// Kill an NPC: the named target, or — when `None` — the caller's current
    /// target. Players are never killable via this path.
    Kill { target: Option<String> },
    /// Grant `count` of item design id `item_id` to the caller (self).
    Give { item_id: i32, count: i32 },
    /// Dump info about the caller's current target (or self, if no target).
    Info,
    /// List the player entities in the caller's space.
    Who,
}
