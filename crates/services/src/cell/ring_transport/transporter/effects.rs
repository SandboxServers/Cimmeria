//! Side-effects produced by the ring-transporter FSM.
//!
//! Split out of the FSM body so the state machine in [`super`] stays
//! readable. Nothing here touches the world — [`super::super::dispatch`]
//! is the only consumer.

/// Effects produced by FSM transitions. The state machine never touches the
/// world directly; the high-level executor consumes these and dispatches them
/// (sends wire methods, teleports entities, fires chain events).
///
/// Mirrors the Python side-effects in order — each variant maps 1:1 to a
/// specific call site (e.g. `Effect::PlaySequence` ⇆ `player.playSequence(...)`,
/// `Effect::TeleportPlayer` ⇆ `player.teleportTo(...)`).
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Play a Kismet sequence on the given player (used for both Teleport_Out
    /// and Teleport_In — Python only fires it on the first player to keep the
    /// matinee from desyncing).
    PlaySequence {
        entity_id: u32,
        event_set_id: i32,
        region_event: RegionEvent,
    },
    /// Fire `teleport::out` script event on the given player.
    /// Currently no chain triggers register on this in our content engine, but
    /// we emit it for parity in case content scripts add one later.
    OnTeleportOut {
        entity_id: u32,
        region_id: i32,
        destination_id: i32,
    },
    /// Set `BSF_MovementLock` on the player and broadcast onStateFieldUpdate.
    LockMovement { entity_id: u32 },
    /// Clear `BSF_MovementLock` on the player and broadcast onStateFieldUpdate.
    UnlockMovement { entity_id: u32 },
    /// Send `onVisible(false)` to make the player invisible.
    HidePlayer { entity_id: u32 },
    /// Send `onVisible(true)` to restore the player.
    ShowPlayer { entity_id: u32 },
    /// Move player to the destination ring's position. Same world (cross-world
    /// is not yet supported — see [`Effect::TeleportCrossWorld`]).
    TeleportPlayer {
        entity_id: u32,
        position: [f32; 3],
        world_name: String,
        destination_region_id: i32,
    },
    /// Cross-world teleport — currently unimplemented; emitted as a warn so
    /// future work can wire it up without changing the FSM.
    TeleportCrossWorld {
        entity_id: u32,
        position: [f32; 3],
        world_name: String,
        destination_region_id: i32,
    },
    /// Fire the `teleport_in` content engine event with `region_id` as the key.
    /// Chain 1044 (Castle_CellBlock) hooks this to complete mission 640.
    FireTeleportIn { entity_id: u32, region_id: i32 },
    /// Send `onRingTransporterList` to the player's client.
    SendDestinationList {
        entity_id: u32,
        source_region_id: i32,
        destinations: Vec<i32>,
    },
}

/// Which Kismet sequence to look up in the region's event set.
///
/// Matches `Atrea.enums.Region_Teleport_Out` (8000) and `Region_Teleport_In`
/// (8001) — see `docs/gameplay/ring-transport-system.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionEvent {
    TeleportOut,
    TeleportIn,
}

impl RegionEvent {
    /// Numeric event ID used in the `event_sets_sequences` lookup.
    pub fn event_id(self) -> i32 {
        match self {
            RegionEvent::TeleportOut => 8000,
            RegionEvent::TeleportIn => 8001,
        }
    }
}
