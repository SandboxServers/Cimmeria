//! The named positions where the base fires plugin hooks.
//!
//! As on the cell (`cimmeria-cell-world`'s `cell::plugin::hook_points`),
//! each variant sits at exactly the line a feature's inline call occupies,
//! so moving the feature does not move its work relative to the sends
//! around it (plugin ADR §2, C6). A variant is added only when a feature
//! needs it; say where it fires and what still exists there.
//!
//! The first set is the session lifecycle crafting's inductions and
//! options follow (#962 step 5): the three places a player's crafting queue
//! is dropped, the gate-travel reset of the stations in reach, and the
//! world-entry login sync.

/// A per-session lifecycle position that names the player entity. Hooks
/// here are synchronous: two of the three sites hold no async context the
/// hook could use, and the third runs between two sends it must not
/// reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionHookPoint {
    /// `logOff`, both variants (`cimmeria-base`'s `dispatch::session::
    /// handle_log_off`): after the cell was told to disconnect and destroy
    /// the player entity and the entity left `entity_to_addr`, before the
    /// user chat channels are left. The session itself survives a return to
    /// character select. Crafting drops the player's inductions here
    /// (`log_off`).
    LogOffAfterEntityUnmapped,
    /// The disconnect, timeout and duplicate-login teardown
    /// (`helpers::destroy_client_entities`): the session has left the
    /// connected map and the player entity has left `entity_to_addr`; the
    /// user chat channels and the cell's `DisconnectEntity` come after.
    /// Crafting drops the player's inductions here (with the disconnect
    /// reason as the cause).
    DisconnectAfterEntityUnmapped,
    /// Gate travel (`cimmeria-base-world-entry`'s `gate_travel::
    /// handle_gate_travel`): after `RESET_ENTITIES` is decided for the world
    /// change and before the fail-closed active-character check, so it runs
    /// even for a transfer that is then refused. The entity id survives the
    /// world change. Crafting drops the player's queue here
    /// (`gate_travel`).
    GateTravelBeforeActiveCharacterCheck,
}

/// A position that hands the hook the player's session state, under the
/// connected-map lock. Hooks here are synchronous and must not block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionStateHookPoint {
    /// Gate travel, after the active-character check passed and before the
    /// cell is asked to create the destination entity. State tied to the
    /// origin world is dropped here: crafting forgets the stations in reach
    /// and holds its option sends until the destination's login send.
    GateTravelBeforeCreateEntity,
}

/// A position in world entry, after the client created the player entity.
/// Hooks here are async and may send to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldEntryHookPoint {
    /// `onClientReady` (`cimmeria-base-world-entry`'s
    /// `world_entry_appearance::client_ready`), after the organization
    /// restore and before the Ignore-list resync. Runs on every world
    /// entry, gate travel included. Crafting's login sync (disciplines,
    /// paradigm levels, blueprints, ASP, crafting options) runs here.
    ClientReadyAfterOrgRestore,
}
