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

    /// Fires when a player successfully dials a stargate — after address
    /// validation, when the four-second gate-open timer is armed.
    /// `destination_world` filters by the destination world's name
    /// (`resources.worlds.world`, e.g. `"Harset"`); `None` matches any
    /// destination.
    OnStargateDialed { destination_world: Option<String> },

    /// Fires when a player steps through an open stargate, immediately
    /// before the world transition tears the cell entity down. Same
    /// `destination_world` filter as [`Self::OnStargateDialed`].
    OnStargateCrossed { destination_world: Option<String> },

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

    /// Fires immediately after a mission is abandoned — after the instance
    /// has been removed from the player's tracker, so a chain gated on
    /// `mission_status <id> eq not_active` sees the post-removal state.
    ///
    /// Abandoning returns a mission to not-active with the player already
    /// past every edge that would normally set the scene up: the offer gate
    /// reopens and nothing repaints it, and whatever dialog-set binding the
    /// mission installed is stranded on its NPC. Both self-heal on the next
    /// world transition, which is why the gap went unreported. This is the
    /// third form of playtest finding H9, after `enter_region` (closed in
    /// the engine by H52) and `player_loaded` (closed in seed).
    ///
    /// Fires from every abandon path — the client-callable `abandonMission`
    /// cell method, the `abandon_mission` chain action, and
    /// `gmMissionClear` / `gmMissionAbandon` — and only when a mission was
    /// actually removed.
    OnMissionAbandoned { mission_id: i32 },

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

    /// Fires when a tagged entity's health crosses **downward** through
    /// `pct` percent of its maximum, as the result of a single damaging
    /// hit. The acting player is the attacker, so mission/step context
    /// comes from the attacker's entity (same frame of reference as
    /// [`Self::OnEntityDeath`]).
    ///
    /// Matching is stateless: the runtime event carries the pre-hit and
    /// post-hit percentages and the match is
    /// `pct_before > pct && pct_after <= pct`. Consequences, all
    /// intentional:
    ///
    /// - Fires **once per crossing** — a follow-up hit that lands while
    ///   the entity is already at or below `pct` has `pct_before <= pct`
    ///   and does not match.
    /// - Fires **again** if the entity is healed back above `pct` and
    ///   then crossed a second time.
    /// - Two chains on the same tag at different thresholds (`:50` and
    ///   `:30`) each fire on their own crossing; a single big hit that
    ///   spans both fires both.
    /// - A **killing blow never fires this trigger** — the damage path
    ///   routes an alive→dead transition to [`Self::OnEntityDeath`]
    ///   instead, so a chain can rely on the two being mutually
    ///   exclusive. Note that this is a guarantee of the *dispatch site*
    ///   (`fire_health_below_for_hit` in `cimmeria-services` suppresses
    ///   any hit ending at zero health), not of the predicate below: the
    ///   band test alone would match a `31% → 0%` hit against a `:30`
    ///   chain.
    ///
    /// Seed form: `event_type = 'entity_health_below'`,
    /// `event_key = "<tag>:<pct>"` (e.g. `"Rinla_Malac:30"`).
    ///
    /// `pct` is `1..=99` (`loader::trigger::HEALTH_PCT_RANGE`); the
    /// loader drops the trigger row otherwise. 100 is excluded because
    /// the band test's upper half is strict, so a full-health entity
    /// (`before == 100`) can never satisfy `before > 100`.
    OnEntityHealthBelow { entity_tag: String, pct: i32 },

    /// Fires when an NPC currently occupying a cover slot is flanked —
    /// the top-threat target moved outside the cover's defensive arc
    /// (cover orientation ± π/2). Used by encounter authors who want to
    /// react to "you outflanked the guard" (the AI itself already
    /// repositions; this is just an authoring hook).
    OnNpcFlanked { npc_template: Option<String> },

    /// Player-perspective twin of [`Trigger::OnNpcFlanked`]: fires from the
    /// same AI decision, but the chain's actions execute against the
    /// **flanking player** (with that player's mission context), not the
    /// NPC. `OnNpcFlanked` runs its actions on the NPC with player id 0, so
    /// it cannot advance a mission objective; this variant exists for
    /// "you outflanked the guard" objectives (Castle Cellblock C06,
    /// objectives 2725/2731). Only fires when the top-threat is a player.
    OnPlayerFlankedNpc { npc_template: Option<String> },
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
    StargateDialed,
    StargateCrossed,
    EffectInit,
    EffectPulseBegin,
    EffectPulseEnd,
    EffectRemoved,
    MissionCompleted,
    MissionAbandoned,
    DialogSetOpen,
    MissionAccepted,
    PlayerEnteredCover,
    PlayerLeftCover,
    PlayerInCoverDuration,
    EntityHealthBelow,
    NpcFlanked,
    PlayerFlankedNpc,
}
