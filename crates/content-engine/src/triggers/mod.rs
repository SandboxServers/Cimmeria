//! Trigger definitions and event matching.
//!
//! Triggers define _when_ a chain activates. Each chain has exactly one trigger.
//! When a game event fires, the chain engine matches it against registered
//! triggers to determine which chains should be evaluated.
//!
//! The enum definitions live here; the discriminant + matching logic lives in
//! [`matching`].

mod matching;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use cimmeria_common::EntityId;

/// A trigger attached to a chain, defining when the chain activates.
///
/// Each variant may carry optional filter fields that narrow the match. For
/// example, `OnEntityCreated { entity_type: Some("SGWMob") }` only fires when
/// an `SGWMob` is created, while `OnEntityCreated { entity_type: None }` fires
/// for every entity creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// Fires when an entity is created in the world.
    OnEntityCreated { entity_type: Option<String> },

    /// Fires when an entity is destroyed/removed from the world.
    OnEntityDestroyed { entity_type: Option<String> },

    /// Fires when an entity dies (health reaches zero).
    /// `entity_tag` filters by a specific tagged NPC (DB `entity_dead_tag`).
    OnEntityDeath {
        entity_type: Option<String>,
        #[serde(default)]
        entity_tag: Option<String>,
    },

    /// Fires when an ability is used by any entity.
    OnAbilityUsed { ability_id: Option<i32> },

    /// Fires when a player interacts with an entity (NPC dialog, object use).
    OnInteraction { interaction_type: Option<String> },

    /// Fires when an entity enters a spatial region.
    /// Uses string keys like `"Castle_Cellblock.Region2"`.
    OnRegionEnter { region_key: String },

    /// Fires when an entity exits a spatial region.
    OnRegionExit { region_key: String },

    /// Fires when a mission reaches a specific step.
    OnMissionStep { mission_id: i32, step: i32 },

    /// Fires when an item is acquired by an entity.
    OnItemAcquired { item_id: Option<i32> },

    /// Fires when a named timer expires.
    OnTimer { timer_name: String },

    /// Fires on an arbitrary named event (extensibility hook).
    OnCustomEvent { event_name: String },

    /// Fires when a player completes world entry (mapLoaded).
    OnPlayerLoaded { world_name: Option<String> },

    /// Fires when the server sends `onDialogDisplay` to a player.
    OnDialogOpen { dialog_id: i32 },

    /// Fires when a player selects a choice/button in a dialog.
    OnDialogChoice { dialog_id: i32 },

    /// Fires when a player interacts with a tagged NPC or object.
    OnInteractTag { entity_tag: String },

    /// Fires when a player interacts with an entity from a named template.
    OnInteractTemplate { template_name: String },

    /// Fires when a player uses an inventory item.
    OnItemUse { item_id: i32 },

    /// Fires when a player equips an inventory item — i.e., moves a stack
    /// into the bandolier (`container_id = 3`) from any other container.
    /// `item_id` is the design / `type_id`, not the inventory instance id.
    OnItemEquipped { item_id: Option<i32> },

    /// Fires when a player arrives via teleporter at a destination region.
    OnTeleportIn { region_id: i32 },

    /// Fires when an effect is first initialized on an entity.
    OnEffectInit,

    /// Fires at the start of each pulse of a periodic effect.
    OnEffectPulseBegin,

    /// Fires at the end of each pulse of a periodic effect.
    OnEffectPulseEnd,

    /// Fires when an effect is removed from an entity.
    OnEffectRemoved,

    /// Fires when a mission is completed.
    OnMissionCompleted { mission_id: i32 },

    /// Fires when a dialog set is opened for a player.
    OnDialogSetOpen { dialog_set_name: String },

    /// Fires immediately after a mission has been accepted, after the
    /// mission/step/objective state has been updated. Used by chains that
    /// need to perform setup work tied to mission start (e.g., highlighting
    /// quest objects, granting starter items) without coupling that work
    /// to the chain that did the accepting.
    OnMissionAccepted { mission_id: i32 },

    /// Fires when a player enters proximity of a cover set (a chunk of
    /// cover-prefab nodes — see `resources.cover_sets`). The same player
    /// can be in multiple cover sets at once; one event per set on entry.
    /// Filter by `cover_set_id` to gate on a specific chunk (e.g. the
    /// Castle Cellblock med-bay cover set for the drone-attack chain).
    /// Wildcard (None) fires for any cover-set entry.
    OnPlayerEnteredCover { cover_set_id: Option<i32> },

    /// Fires when a player leaves a cover set's proximity. Paired with
    /// [`Self::OnPlayerEnteredCover`] for symmetry; useful for chains
    /// that want to react to "player no longer behind cover".
    OnPlayerLeftCover { cover_set_id: Option<i32> },

    /// Fires once when the player has continuously been in a cover set
    /// for at least `seconds`. Debounced — leaving and re-entering resets
    /// the timer. Useful for "stay in cover for 5 seconds" objectives.
    OnPlayerInCoverDuration {
        cover_set_id: Option<i32>,
        seconds: u32,
    },

    /// Fires when an NPC currently occupying a cover slot is flanked —
    /// the top-threat target moved outside the cover's defensive arc
    /// (cover orientation ± π/2). Used by encounter authors who want to
    /// react to "you outflanked the guard" (the AI itself already
    /// repositions; this is just an authoring hook).
    OnNpcFlanked { npc_template: Option<String> },
}

/// Runtime event payload passed to the chain engine when a game event occurs.
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// Discriminant identifying which trigger type this event corresponds to.
    pub trigger_type: TriggerType,

    /// The entity that caused the event.
    pub source_entity: Option<EntityId>,

    /// The entity the event targets.
    pub target_entity: Option<EntityId>,

    /// Event-specific parameters.
    pub params: HashMap<String, serde_json::Value>,
}

/// Discriminant enum for trigger types, used as a grouping key in the chain
/// engine's index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TriggerType {
    EntityCreated,
    EntityDestroyed,
    EntityDeath,
    AbilityUsed,
    Interaction,
    RegionEnter,
    RegionExit,
    MissionStep,
    ItemAcquired,
    Timer,
    CustomEvent,
    PlayerLoaded,
    DialogOpen,
    DialogChoice,
    InteractTag,
    InteractTemplate,
    ItemUse,
    ItemEquipped,
    TeleportIn,
    EffectInit,
    EffectPulseBegin,
    EffectPulseEnd,
    EffectRemoved,
    MissionCompleted,
    DialogSetOpen,
    MissionAccepted,
    PlayerEnteredCover,
    PlayerLeftCover,
    PlayerInCoverDuration,
    NpcFlanked,
}
