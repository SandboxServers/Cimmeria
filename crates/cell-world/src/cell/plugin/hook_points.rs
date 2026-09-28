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
}
