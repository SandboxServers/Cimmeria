//! Shared data structs used by both `BaseToCellMsg` and `CellToBaseMsg`.

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
