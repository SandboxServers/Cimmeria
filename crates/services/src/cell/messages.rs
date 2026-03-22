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

/// NPC-specific data included in AoI enter events.
///
/// Carries template-driven values that the client needs for correct rendering
/// and interaction display. Only populated for NPC entities (not players).
///
/// Mirrors the full `createOnClient()` cascade from the Python scripts:
/// `SGWSpawnableEntity.createOnClient()` → `SGWBeing.createOnClient()`.
#[derive(Debug, Clone, Default)]
pub struct NpcAoIData {
    /// Localized name string ID from `entity_templates.name_id`.
    pub name_id: Option<i32>,
    /// Faction ID (0=neutral, 1=Tau'ri, 3=SGC, 10=hostile).
    pub faction: u8,
    /// Alignment ID.
    pub alignment: u8,
    /// Entity flags from `entity_templates.flags`.
    pub entity_flags: u64,
    /// Interaction type flags (UINT64 bitmask for cursor/interaction UI).
    pub interaction_type: i64,
    /// Speaker ID for `onEntityProperty(GENERICPROPERTY_DatabaseId, speakerId)`.
    pub speaker_id: Option<i32>,
    /// Kismet event set ID for `onKismetEventSetUpdate`.
    pub event_set_id: Option<i32>,
    /// Static mesh name (for `onStaticMeshNameUpdate` — non-humanoid entities).
    pub static_mesh: Option<String>,
    /// Body set name (for `BeingAppearance` — humanoid entities, or `onStaticMeshNameUpdate`).
    pub body_set: Option<String>,
    /// Body components (for `BeingAppearance` — humanoid entities with body parts).
    pub components: Vec<String>,
}

/// A saved mission loaded from the database for re-login.
#[derive(Debug, Clone)]
pub struct SavedMission {
    pub mission_id: i32,
    pub status: i8,
    pub current_step_id: Option<i32>,
    pub completed_step_ids: Vec<i32>,
    pub completed_objective_ids: Vec<i32>,
    pub active_objective_ids: Vec<i32>,
    pub failed_objective_ids: Vec<i32>,
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
        /// Saved missions loaded from DB, to be restored before content engine fires.
        saved_missions: Vec<SavedMission>,
    },

    /// Reload the content engine from the database (triggered by admin API / Content Editor).
    ReloadContentEngine,

    /// Minigame result callback (forwarded from BaseApp after minigame server reports).
    MinigameResult {
        entity_id: u32,
        result_code: u8,
        on_victory_chains: Vec<i64>,
    },
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
        /// Entity level (for `onLevelUpdate`). Defaults to 1.
        level: u32,
        /// NPC-specific data (faction, alignment, flags, name). None for players.
        npc_data: Option<NpcAoIData>,
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
    /// World entry flow (teardown -> create player -> enter world).
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

    /// Send a ghost entity method call to a specific witness player.
    ///
    /// Used for broadcasting property updates (InteractionType, SetVisible, etc.)
    /// to players who have the entity in their AoI. The `entity_id` is the ghost
    /// entity the method is called on; `witness_id` is the player to send to.
    WitnessEntityMethod {
        witness_id: u32,
        entity_id: u32,
        method_index: u16,
        args: Vec<u8>,
    },

    /// Start a minigame session for a player (Cell → Base).
    StartMinigame {
        entity_id: u32,
        player_id: i32,
        game_name: String,
        difficulty: u32,
        on_victory_chains: Vec<i64>,
    },

    /// Minigame result callback (minigame server → Cell via Base).
    MinigameResult {
        entity_id: u32,
        result_code: u8,
        on_victory_chains: Vec<i64>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_mission_debug_prints() {
        let m = SavedMission {
            mission_id: 622,
            status: 1,
            current_step_id: Some(700),
            completed_step_ids: vec![],
            completed_objective_ids: vec![],
            active_objective_ids: vec![800, 801],
            failed_objective_ids: vec![],
        };
        let debug = format!("{:?}", m);
        assert!(debug.contains("622"));
        assert!(debug.contains("800"));
    }

    #[test]
    fn saved_mission_clone() {
        let m = SavedMission {
            mission_id: 622,
            status: 1,
            current_step_id: Some(700),
            completed_step_ids: vec![699],
            completed_objective_ids: vec![801],
            active_objective_ids: vec![800],
            failed_objective_ids: vec![],
        };
        let cloned = m.clone();
        assert_eq!(cloned.mission_id, 622);
        assert_eq!(cloned.status, 1);
        assert_eq!(cloned.current_step_id, Some(700));
        assert_eq!(cloned.completed_step_ids, vec![699]);
        assert_eq!(cloned.completed_objective_ids, vec![801]);
        assert_eq!(cloned.active_objective_ids, vec![800]);
        assert!(cloned.failed_objective_ids.is_empty());
    }

    #[test]
    fn saved_mission_completed_status() {
        let m = SavedMission {
            mission_id: 622,
            status: 2, // MISSION_COMPLETED
            current_step_id: None,
            completed_step_ids: vec![700],
            completed_objective_ids: vec![800, 801],
            active_objective_ids: vec![],
            failed_objective_ids: vec![],
        };
        assert_eq!(m.status, 2);
        assert!(m.current_step_id.is_none());
        assert_eq!(m.completed_step_ids.len(), 1);
        assert_eq!(m.completed_objective_ids.len(), 2);
    }

    #[test]
    fn npc_aoi_data_default() {
        let data = NpcAoIData::default();
        assert_eq!(data.faction, 0);
        assert_eq!(data.alignment, 0);
        assert_eq!(data.entity_flags, 0);
        assert_eq!(data.interaction_type, 0);
        assert!(data.name_id.is_none());
        assert!(data.components.is_empty());
    }
}
