//! Message types for Base↔Cell inter-service communication.
//!
//! In the C++ architecture, BaseApp and CellApp communicated over Mercury UDP.
//! In our single-process Rust server, they use `tokio::mpsc` channels with these
//! typed message enums.

/// Mail operation types forwarded from CellService to BaseApp for DB execution.
#[derive(Debug)]
pub enum MailOp {
    /// Request mail headers (inbox or archive).
    RequestHeaders { b_archive: u8 },
    /// Request a specific mail body.
    RequestBody { mail_id: i32 },
    /// Delete a mail message.
    Delete { mail_id: i32 },
    /// Archive a mail message.
    Archive { mail_id: i32 },
}

/// Messages sent from BaseApp to CellApp.
// Cannot derive Debug because oneshot::Sender doesn't implement Debug.
// Manual impl would be possible but not worth the boilerplate.
pub enum BaseToCellMsg {
    /// Create a cell entity in the named world at the given position/rotation.
    /// The `reply_tx` oneshot returns the resolved `space_id` so the caller
    /// can `.await` it before building the world-entry wire packet.
    CreateEntity {
        entity_id: u32,
        world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
        reply_tx: tokio::sync::oneshot::Sender<u32>,
    },

    /// Destroy a cell entity (player left, entity despawned).
    DestroyEntity {
        entity_id: u32,
    },

    /// Mark an entity as having a client controller (player).
    /// Sent after world entry packets are delivered to the client.
    ConnectEntity {
        entity_id: u32,
    },

    /// Remove client controller from an entity (player disconnected).
    DisconnectEntity {
        entity_id: u32,
    },

    /// Client position/movement update forwarded from `avatarUpdateExplicit`.
    EntityMove {
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    },

    /// Client→server cell entity method call forwarded from BaseApp.
    ///
    /// `method_index` is the flattened EXPOSED CellMethod index for the
    /// SGWPlayer entity type (0 = setTargetID, 1 = setMovementType, etc.).
    /// `args` contains the raw method arguments (after entity_id extraction).
    CellMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Chat message from a player, forwarded from BaseApp for spatial distribution.
    ///
    /// The CellService broadcasts to witnesses based on channel type:
    /// say/emote (nearby), yell (wider range).
    ChatMessage {
        entity_id: u32,
        speaker_name: String,
        speaker_flags: u8,
        channel: u8,
        text: String,
    },

    /// Initialize player state after world entry (missions, etc.).
    /// Sent after ConnectEntity so the CellService can populate per-player data.
    InitPlayerState {
        entity_id: u32,
        player_id: i32,
        world_name: String,
    },

    /// Reload the content engine from the database (triggered by admin API / Content Editor).
    ReloadContentEngine,
}

/// Messages sent from CellApp to BaseApp.
#[derive(Debug)]
pub enum CellToBaseMsg {
    /// Notification that a space exists (sent at startup and on dynamic creation).
    SpaceData {
        space_id: u32,
        world_name: String,
    },

    /// Response to `CreateEntity` — entity placed in a space.
    EntityCreated {
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
    },

    /// An entity has moved (for relaying volatile updates to a specific witness via BaseApp).
    EntityMoved {
        witness_id: u32,
        entity_id: u32,
        space_id: u32,
        position: [f32; 3],
        direction: [f32; 3],
        velocity: [f32; 3],
    },

    /// A new entity entered a witness's Area of Interest.
    EnteredAoI {
        witness_id: u32,
        entity_id: u32,
        space_id: u32,
        class_id: u8,
        position: [f32; 3],
        direction: [f32; 3],
    },

    /// An entity left a witness's Area of Interest.
    LeftAoI {
        witness_id: u32,
        entity_id: u32,
    },

    /// Send a server→client entity method call to the entity's client.
    ///
    /// The BaseApp looks up the entity's client address and sends the method
    /// call using `append_entity_method` encoding.
    EntityMethodCall {
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Request mail headers for a player entity (forwarded to BaseApp for DB query).
    MailRequest {
        entity_id: u32,
        /// Mail operation type.
        op: MailOp,
    },

    /// Request a gate travel (world transition) for a player entity.
    ///
    /// The CellService has already validated the stargate address and removed
    /// the entity from the old space. BaseApp must send RESET_ENTITIES to the
    /// client, then re-create the entity in the new world via the standard
    /// Phase 5a/5b world entry flow.
    GateTravel {
        entity_id: u32,
        target_world_name: String,
        position: [f32; 3],
        rotation: [f32; 3],
    },

    /// Persist a mission state change to the database.
    ///
    /// Uses `INSERT ... ON CONFLICT DO UPDATE` on `sgw_mission`.
    MissionUpdate {
        player_id: i32,
        mission_id: i32,
        status: i8,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
    },

    /// Grant XP to a player entity (from a mob kill).
    ///
    /// The CellService computes the XP amount from the mob's level and sends
    /// this to BaseApp, which updates the player's XP/level and sends client
    /// notifications.
    GrantXP {
        entity_id: u32,
        xp_amount: u64,
    },

    /// Grant an item to a player and persist to `sgw_inventory`.
    GrantItem {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        container_id: i32,
        count: i32,
    },
}
