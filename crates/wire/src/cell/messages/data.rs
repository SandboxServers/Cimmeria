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
    /// Live `CellEntity::state_field`. Python `SGWBeing.createOnClient`
    /// sends `onStateFieldUpdate(self.stateField)`
    /// (`deprecated/python/cell/SGWBeing.py:507`): a mob that is already a
    /// corpse when a witness first sees it (it walked back into range, it
    /// relogged, it reanchored after its own death) must arrive dead, or the
    /// client builds a standing, full-health guard that never fights back.
    pub state_field: u32,
    /// Live HEALTH/FOCUS (`SGWBeing.sendStats` sends live values); `None`
    /// keeps the cascade's template defaults. Boxed so the deferred-AoI
    /// buffer enum stays under clippy's `large_enum_variant` bound.
    pub vitals: Option<Box<NpcVitals>>,
    /// Owning player's entity id when the NPC is a pet (class 0x05). The
    /// cascade sends it as `onEntityProperty(GENERICPROPERTY_PetOwnerId,
    /// owner)` to every witness, since `SGWPet.ownerID` is CELL_PUBLIC.
    pub pet_owner_id: Option<u32>,
}

/// Live HEALTH and FOCUS of an NPC, each as `[min, cur, max]` — the
/// `StatUpdate` field order (`entities/defs/alias.xml`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NpcVitals {
    pub health: [i32; 3],
    pub focus: [i32; 3],
}

impl NpcAoIData {
    /// Snapshot everything the NPC `createOnClient()` cascade needs from a
    /// live cell entity — the template-driven identity plus the live
    /// state/health a witness must be introduced with.
    pub fn from_entity(entity: &cimmeria_entity::cell_entity::CellEntity) -> Self {
        use cimmeria_entity::stats::{FOCUS, HEALTH};
        let triple = |id| {
            entity
                .stats
                .get(id)
                .map_or([0; 3], |s| [s.min, s.cur, s.max])
        };
        Self {
            name_id: entity.name_id,
            faction: entity.faction,
            alignment: entity.alignment,
            entity_flags: entity.entity_flags,
            // Send the BASE interaction type in the cascade (not merged).
            // Dynamic per-player flags are sent as a separate
            // InteractionType update, matching the C++ server's
            // createOnClient(base) → dynamicUpdate(merged) flow.
            interaction_type: entity.interaction_type_flags,
            speaker_id: entity.speaker_id,
            event_set_id: entity.event_set_id,
            static_mesh: entity.static_mesh.clone(),
            body_set: entity.body_set.clone(),
            components: entity.components.clone(),
            state_field: entity.state_field,
            vitals: Some(Box::new(NpcVitals {
                health: triple(HEALTH),
                focus: triple(FOCUS),
            })),
            pet_owner_id: entity.pet.as_ref().map(|p| p.owner_id),
        }
    }
}

/// Live cell-side state of a **player** included in AoI enter events.
///
/// The player-ghost half of the `createOnClient()` cascade that only the
/// cell knows: the values `SGWBeing.createOnClient()` /
/// `SGWPlayer.createOnClient()` read off the live entity
/// (`deprecated/python/cell/SGWBeing.py:499-514`, `SGWPlayer.py:575-581`).
/// The identity half — name, level, archetype, alignment, `BeingAppearance`
/// and tint args — lives on the base session and is joined in at emit time
/// (`base::world_entry::cell_dispatch::player_ghost`), because the base owns
/// the appearance cache that holster / equip / bandolier changes keep fresh.
///
/// Only populated for player entities; NPCs carry [`NpcAoIData`] instead.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerAoIData {
    /// `CellEntity::state_field` — a player who is dead, crouched or in
    /// combat when they enter view must be introduced that way.
    pub state_field: u32,
    /// Currently selected target entity id, `0` for none (`onTargetUpdate`).
    pub target_id: i32,
    /// `StatList::serialize_public()` — the witness-visible stat subset,
    /// already in `onStatUpdate` wire form. Python's `sendStats` sends a
    /// non-owning mailbox `publicStats` only, never the full list.
    pub stat_update: Vec<u8>,
    /// `StatList::serialize_public_base()` for `onStatBaseUpdate`.
    pub stat_base_update: Vec<u8>,
    /// Ammo type of the active bandolier item, `0` when the slot is empty
    /// (`onEntityProperty(GENERICPROPERTY_AmmoTypeId, …)`).
    pub ammo_type_id: i32,
}

impl PlayerAoIData {
    /// Snapshot the witness-visible live state of a player cell entity.
    pub fn from_entity(entity: &cimmeria_entity::cell_entity::CellEntity) -> Self {
        Self {
            state_field: entity.state_field,
            target_id: entity.current_target_id.unwrap_or(0),
            stat_update: entity.stats.serialize_public(),
            stat_base_update: entity.stats.serialize_public_base(),
            ammo_type_id: entity.active_ammo_type(),
        }
    }
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
    /// Cumulative completion+failure count from `sgw_mission.repeats`. Must
    /// be restored onto `MissionInstance.repeats` so a relog after N completions
    /// of a repeatable mission correctly gates re-acceptance against
    /// `mission.numRepeats` rather than appearing to reset to 0.
    pub repeats: i32,
}
