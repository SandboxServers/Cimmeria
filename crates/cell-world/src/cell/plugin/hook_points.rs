//! The named positions where core fires plugin hooks.
//!
//! Each variant sits at exactly the line a feature's inline call occupied
//! before the feature became a plugin, so moving the feature does not move
//! its work within the tick (the wire order the client applies depends on
//! it). A variant is added only when a feature moves; say where it fires.

/// A position in the cell loop's AoI tick (`cimmeria-cell`'s
/// `message_loop`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TickStage {
    /// After the gate-crossing hold tick (`gate_travel::crossing_tick`),
    /// before the auto-cycle tick. The duel tick runs here: challenge
    /// expiry, the countdown end and the safety ends.
    AfterGateCrossing,
    /// After the ring-transporter timers, before the deployable tick. The
    /// owner-teardown sweeps run here (the pet owner sweep).
    AfterRingTransport,
    /// After the timed stat-buff tick, before NPC movement. Sends that must
    /// follow the AoI tick's `CREATE_ENTITY` run here (the pet arrival VFX).
    AfterStatBuffs,
}

/// A per-entity position in core's lifecycle handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityHookPoint {
    /// The base's `DestroyEntity` for this entity, after the bandolier-ammo
    /// flush and before `SpaceManager::destroy_entity`
    /// (`base_messages::lifecycle::flush_and_destroy`). The entity still
    /// exists and the channel to the base is open.
    BeforeBaseDestroy,
    /// `SpaceManager::disconnect_entity` for this entity, after the
    /// per-player scrubs (vault session, gate dial, crossing hold) and before
    /// the pet and AoI teardown. The entity still exists, in its space, and
    /// the channel to the base is open. The duel's disconnect end runs here.
    BeforeDisconnectTeardown,
    /// Every cell path that sends `TeleportPlayer` or `GateTravel` for this
    /// player (a teleport in its space, a space transfer, gate travel, a
    /// ring transport, a respawn), just before that send is built. The
    /// movement validator's snap-back is not a travel site. The duel's
    /// travel end runs here; `every_travel_site_fires_the_travel_hook`
    /// (`cimmeria-cell-duel`) scans for the call.
    BeforeTravelSend,
}

/// A position in the death resolver for a killed entity, with its killer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeathHookPoint {
    /// The death resolver (`cimmeria-cell-combat`'s `abilities::death`) for
    /// a player target, after the NPC threat purge and before the owner's
    /// pets leave. The victim is dead and still exists. The duel's death end
    /// (a duelist killed by anyone but the partner loses) runs here.
    AfterPlayerThreatPurge,
}
