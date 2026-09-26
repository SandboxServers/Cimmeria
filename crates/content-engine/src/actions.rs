//! Action executors for chain effects.
//!
//! Actions are the "do" part of a chain -- they modify game state when a
//! trigger fires and all conditions pass. The action types cover the breadth
//! of the original Python scripting layer: XP/item grants, effects,
//! teleportation, spawning, dialog, missions, loot, timers, and extensibility
//! hooks.
//!
//! Actions are resolved by the engine but executed by the caller (CellService),
//! which has access to game state. The `execute()` method on each action is
//! preserved for backward compatibility but should not be called for DB-driven
//! chains — use `ChainEngine::resolve_event()` instead.

use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// Serde default for [`Action::StartMinigame`]'s `difficulty`.
///
/// Must stay equal to the DB-row loader's default in
/// `loader/action.rs`; the two are the same contract reached by two paths.
fn default_minigame_difficulty() -> u32 {
    1
}

/// An action to execute when a chain's trigger fires and conditions pass.
// `PartialEq` (but not `Eq` — several variants carry `f32`) so chain-replay
// tests can assert the **exact resolved action list** as a single
// `assert_eq!` against a `vec![..]` literal. That is the campaign's stated
// acceptance shape ("asserts the exact resolved action list", Harset
// work-packets.md "Common Acceptance"), and the alternative — a chain of
// per-index `matches!` arms — cannot bind runtime values (a dsm id or a
// template slot coming from a table-driven test case), so those tests were
// silently weaker than the ones written against literals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // ── Original generic actions ──────────────────────────────────────────
    /// Award experience points to the source entity.
    GrantXP { amount: u64 },

    /// Add items to the source entity's inventory.
    GrantItem {
        item_id: i32,
        count: i32,
        #[serde(default)]
        container_id: Option<i32>,
    },

    /// Remove items from the source entity's inventory.
    RemoveItem { item_id: i32, count: i32 },

    /// Apply a timed or permanent effect to the source entity.
    ApplyEffect {
        effect_id: i32,
        duration_secs: Option<f32>,
    },

    /// Remove an active effect from the source entity.
    RemoveEffect { effect_id: i32 },

    /// Teleport the source entity to a position in a target space.
    Teleport { space_id: i32, position: [f32; 3] },

    /// Cross-world teleport: tear down the player's cell entity on this
    /// world and re-create on `world_name` at `position` via the
    /// `CellToBaseMsg::GateTravel` pipeline (same flow used by stargate
    /// dial). `destination_ring_id` is `None` so base does not send
    /// `BaseToCellMsg::AdvanceRingDestination` — there's no destination
    /// ring FSM to advance. Use this for chain-driven cross-world hops
    /// where a ring ceremony is unnecessary or unavailable (e.g., the
    /// mission 688 armory exit, where the Castle map has no ring
    /// transporter prefab actor in its kismet).
    CrossWorldTeleport {
        world_name: String,
        position: [f32; 3],
    },

    /// Spawn a mission-scoped NPC from an `entity_templates` row into the
    /// **acting player's current space**, tagged so `entity_dead_tag` /
    /// `interact_tag` chains can find it again.
    ///
    /// The space is never named by the seed row: a chain authored for a
    /// per-player instance (Harset Market / Storage, Castle Cellblock) has
    /// no way to know which instance the firing player is in, and naming a
    /// world would let the same row populate somebody else's copy. The
    /// executor reads the space off the triggering entity instead.
    ///
    /// `allow_shared` is the escape hatch for the refusal that follows from
    /// that: spawning into a *non-instanced* world drops a mission NPC into
    /// the shared hub where every other player sees it (and, for a hostile,
    /// gets attacked by it). Mission content must not be able to do that by
    /// accident, so the executor refuses unless the author opts in.
    ///
    /// `is_stationary` / `aggression` complete the spawn descriptor, the
    /// runtime twins of `spawnlist.is_stationary` /
    /// `spawnlist.aggression_override`. `is_stationary: None` means off.
    /// `aggression` is an `EMobAggressionLevel` override (1 hostile ... 5
    /// default, 0 the pre-NA13 "passive" = neutral); `None` means the
    /// faction reaction decides, so a faction-10 template is hostile on
    /// sight unless the action says otherwise.
    ///
    /// There is deliberately **no `respawn_secs`**. A content-scoped spawn
    /// is always one-shot (the respawn tick has no instance-lifetime
    /// awareness, and a revived mission NPC would re-fire its
    /// `entity_dead_tag` chain), so the field was parsed, carried here,
    /// threaded through the executor and then unconditionally discarded. A
    /// seed row that supplies it still loads; the loader warns once with
    /// `reason = "respawn_secs_not_honoured"` (PR #662 review, finding 5).
    SpawnEntity {
        template_id: i32,
        position: [f32; 3],
        /// Yaw in radians, matching `spawnlist.heading`.
        heading: f32,
        /// `spawnlist.tag` equivalent. Mandatory: an untagged mission spawn
        /// can never be despawned, killed-by-tag or interacted with, and the
        /// idempotence guard keys on it.
        tag: String,
        is_stationary: Option<bool>,
        aggression: Option<i32>,
        /// Opt in to spawning into a non-instanced (shared) world.
        allow_shared: Option<bool>,
    },

    /// Despawn the tagged entity: fan `LeftAoI` to every current witness,
    /// scrub the witness sets, then destroy. The counterpart to
    /// [`Action::SpawnEntity`]; [`Action::DestroyTaggedEntity`] is the
    /// older spelling of the same behaviour and routes identically.
    DespawnEntity { entity_tag: String },

    /// Open a dialog set for the source entity (player).
    StartDialog { dialog_set_id: i32 },

    /// Advance the specified mission to its next step (legacy; prefer AcceptMission/AdvanceStep).
    AdvanceMission { mission_id: i32 },

    /// Mark the specified mission as complete.
    CompleteMission { mission_id: i32 },

    /// Play an animation on the source or target entity.
    PlayAnimation { animation: String },

    /// Play a sound effect at the source entity's position.
    PlaySound { sound: String },

    /// Send a text message on a named channel.
    SendMessage { channel: String, message: String },

    /// Modify a property on the source entity.
    ModifyProperty {
        property: String,
        operation: PropertyOp,
        value: serde_json::Value,
    },

    /// Roll a loot table and grant results.
    RollLootTable { table_id: i32 },

    /// Spawn a loot bag entity at the given position.
    SpawnLootBag { position: [f32; 3] },

    /// Start a named timer.
    StartTimer {
        name: String,
        duration_secs: f32,
        repeat: bool,
    },

    /// Cancel a running named timer.
    CancelTimer { name: String },

    /// Trigger another chain by ID.
    TriggerChain { chain_id: i64 },

    /// Execute a custom handler function.
    ExecuteCustom {
        handler: String,
        params: serde_json::Value,
    },

    // ── DB-driven action types ────────────────────────────────────────────
    /// Accept and start tracking a mission.
    AcceptMission { mission_id: i32 },

    /// Display a specific dialog to the player.
    DisplayDialog { dialog_id: i32 },

    /// Add a dialog set entry to an NPC.
    AddDialogSet {
        dialog_set_id: i32,
        slot: i32,
        mission_id: Option<i32>,
    },

    /// Remove a dialog set entry from an NPC.
    RemoveDialogSet { dialog_set_id: i32, slot: i32 },

    /// Play a cinematic sequence/cutscene.
    PlaySequence { sequence_id: i32 },

    /// Advance a mission to a specific step.
    AdvanceStep { mission_id: i32, step_id: i32 },

    /// Set or modify interaction type flags on a tagged entity.
    SetInteractionType {
        entity_tag: String,
        operation: String,
        mask: i64,
    },

    /// Start a minigame for the player.
    StartMinigame {
        minigame_type: String,
        /// Difficulty tier handed to the SWF in the `joinOK` game params.
        /// The original client asserted 1-5; the loader range-checks and
        /// defaults to 1 when the seed row omits it.
        ///
        /// The serde default mirrors that loader default so the two agree.
        /// `Action` is `Deserialize`, and this field was added after the
        /// variant shipped — without the default, any previously serialized
        /// payload fails to deserialize on a missing key rather than taking
        /// the same 1 the DB-row path would give it.
        #[serde(default = "default_minigame_difficulty")]
        difficulty: u32,
        on_victory_chains: Vec<i64>,
    },

    /// Set the aggression override on a tagged NPC (`EMobAggressionLevel`:
    /// 1 hostile ... 5 default; 0 is the pre-NA13 "passive" and maps to
    /// neutral). Only hostile aggroes on sight.
    SetAggression { entity_tag: String, level: i32 },

    /// Push a tagged NPC into `AiState::Investigating` with the given
    /// world-space point of interest. The NPC pathfinds to the POI,
    /// dwells `INVESTIGATE_DWELL_SECS` (hardcoded 5 seconds, defined
    /// in `crates/services/src/cell/service/npc_ai.rs`), and returns
    /// to `AiState::Idle`. Future variations on the dwell would lift
    /// it to a template column.
    ///
    /// Threat preemption converts Investigating → Fighting. The POI
    /// field persists on the entity post-fight but doesn't auto-route
    /// back — only Patrol and Wander auto-resume from their per-state
    /// scratch on Fighting → Leashing → Idle. Content authors who
    /// want a continued investigation after a fight must fire a
    /// fresh `SetNpcPoi`.
    SetNpcPoi {
        entity_tag: String,
        x: f32,
        y: f32,
        z: f32,
    },

    /// Set or clear the follow target for a tagged NPC. When
    /// `target_tag` resolves to an entity, the NPC transitions to
    /// `AiState::Follow` and maintains the distance band defined by
    /// `follow_min_distance` / `follow_max_distance` on the template.
    ///
    /// `target_tag = None` (or a value the runtime can't resolve) is
    /// treated as "clear": the follow state drops to Idle and
    /// `follow_target_id` is cleared. Pin the unresolvable case so a
    /// typo or a removed tag doesn't leave the follower in a half
    /// state.
    ///
    /// `use_player: Some(true)` resolves the follow target to the
    /// entity that triggered the chain instead of doing a `target_tag`
    /// lookup — mirrors `MoveEntity::use_player`. This is the only way
    /// to make an NPC follow a player: player entities carry no `tag`
    /// (tags only come from `spawnlist.tag` at NPC spawn), so
    /// `find_entity_by_tag` can never resolve one. When `use_player` is
    /// set but the triggering entity isn't a player (e.g. a cover-node
    /// chain firing with an NPC as the source entity), the follow
    /// target is left unresolved rather than silently following the
    /// wrong entity.
    ///
    /// Threat preemption converts a mob's Follow → Fighting; the follow
    /// target persists on the entity, and the leash that ends the fight
    /// resets the follower where it stands and puts it back in Follow
    /// while the target is still in the space (NA42). A target that has
    /// left is cleared and the follower goes Idle; re-fire the action to
    /// re-arm it. A `being` (Col Marsh) never enters combat at all.
    SetFollowTarget {
        entity_tag: String,
        target_tag: Option<String>,
        use_player: Option<bool>,
    },

    /// Push a tagged NPC into a specific AI state. Supports the
    /// terminal / scripted states: `Despawning`, `Submit`, `Error`,
    /// and `Idle` (for cleanup). Other states should be reached via
    /// their behavior-specific actions (`SetNpcPoi` for Investigating,
    /// `SetFollowTarget` for Follow, etc.) so the per-state scratch
    /// fields are populated correctly.
    ///
    /// - `Despawning` → AI tick removes the entity from the space
    ///   on the next pass. Witnesses get an AoI-left event.
    /// - `Submit` → clears combat state, broadcasts movement-type
    ///   None; NPC sits inert until destroyed or transitioned.
    /// - `Error` → halts AI ticking on the NPC, logs the inconsistency.
    ///   Used by `enterErrorAIState` slash commands and by the AI tick
    ///   itself when it detects unrecoverable state.
    /// - `Idle` → clean fallback that lets the AI tick re-route.
    ///
    /// Other state values (Fighting/Leashing/etc.) are rejected with
    /// a warn log — those are owned by the runtime, not content.
    SetNpcAiState {
        entity_tag: String,
        state: NpcAiStateAction,
    },

    /// Destroy a tagged entity (remove from world). Alias of
    /// [`Action::DespawnEntity`] — both execute the same witness-scrubbing
    /// despawn. Kept as a separate variant because `destroy_entity` is the
    /// spelling already in the seed.
    DestroyTaggedEntity { entity_tag: String },

    /// Activate a transporter to move the player to a region.
    TriggerTransporter { region_id: i32 },

    /// Send a system message to the player.
    SystemMessage { message_id: i32 },

    /// Speak one line of NPC dialogue into the triggering player's chat
    /// window **without** opening a dialog window.
    ///
    /// The 2009 client's dialog module has no non-modal path at all — its
    /// lowest screen type registers the modal Blurb window under a "TEMP
    /// HACK" comment — so a companion combat line ("Let's move out!")
    /// cannot be a dialog. The grounded non-modal route is the chat
    /// message the server already drives successfully:
    /// `onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)`
    /// (`entities/defs/interfaces/Communicator.def`).
    ///
    /// - `screen_id` names a `resources.dialog_screens` row. The executor
    ///   resolves the line text from the cell's startup catalogue so
    ///   content authors never retype 2009 text and never drift from it.
    /// - `speaker` is an explicit param rather than a `speakers` lookup
    ///   because the companion-bark screens carry `speaker_id = 0`.
    /// - `channel` is the `EChannel` wire byte. Only `CHAN_say` (0) is
    ///   accepted today; `CHAN_splash` is unverified in the client.
    ///
    /// This deliberately does **not** route through [`Action::SystemMessage`],
    /// whose wire format is still unknown.
    NpcBark {
        screen_id: i32,
        speaker: String,
        channel: u8,
    },

    /// Apply QR combat damage to a stat.
    QrCombatDamage {
        stat_id: i32,
        source_id: i32,
        amount_nvp: String,
    },

    /// Change a stat on the entity.
    ///
    /// Application order in the executor: `min` / `max` (bounds), then
    /// `set_to_max` (sets `cur` to the new `max`), then `amount`
    /// (additive delta, clamped to `[min, max]`). `amount` is the
    /// "delta" path consumables use (e.g. Health Slappack TC1: +500
    /// HP); the bounds-modifying fields are for buffs/debuffs that
    /// shift the cap.
    ChangeStat {
        stat_id: i32,
        min: Option<i32>,
        max: Option<i32>,
        use_ammo_stat: Option<bool>,
        set_to_max: Option<bool>,
        /// Additive delta applied to `cur` after bounds adjustments.
        /// Positive heals, negative damages. Clamped via `Stat::change`.
        amount: Option<i32>,
    },

    /// Abandon an active mission.
    AbandonMission { mission_id: i32 },

    /// Fail a specific objective within a mission.
    FailObjective { mission_id: i32, objective_id: i32 },

    /// Increment a named counter.
    IncrementCounter { counter_name: String, amount: i32 },

    /// Reset a named counter to zero.
    ResetCounter { counter_name: String },

    /// Complete a specific objective within a mission.
    CompleteObjective { mission_id: i32, objective_id: i32 },

    /// Set the visibility of a tagged entity.
    SetVisible { entity_tag: String, visible: bool },

    /// Move a tagged entity or the player to a destination.
    MoveEntity {
        entity_tag: Option<String>,
        destination: [f32; 3],
        world: Option<String>,
        use_player: Option<bool>,
    },

    // ── Space script action types ────────────────────────────────────────
    /// Snap a tagged NPC to a destination — an instant server-side position
    /// write, not a path or a walk animation. `speed` is parsed from the
    /// seed row but the executor does not use it; this variant exists as
    /// the space-script spelling for a scripted reposition.
    MoveWaypoint {
        entity_tag: String,
        destination: [f32; 3],
        speed: f32,
    },

    /// Equip an item to an equipment slot (typically Bandolier bag_id=3).
    SetActiveSlot { bag_id: i32, slot: i32 },

    /// Force-fire an ability on an entity (or self if entity_tag is None).
    LaunchAbility {
        ability_id: i32,
        entity_tag: Option<String>,
    },

    /// Map a dialog set to an NPC entity template (archetype-conditional dialog).
    AddDialog {
        dialog_set_id: i32,
        entity_template: Option<i32>,
        mission_id: Option<i32>,
    },

    /// Generate threat/aggro on a target entity from the instigator.
    GenerateThreat {
        entity_tag: Option<String>,
        threat_level: i32,
    },

    /// Teach the acting player a stargate address
    /// (`resources.stargates.stargate_id`), so their DHD will offer it and
    /// the server's dial gate will accept it.
    ///
    /// This is the port of the 2009 Atrea authoring node
    /// `Act_StargateAddress`
    /// (`deprecated/entities-editor/editor/Nodes.xml:2428`), which called
    /// `SGWPlayer.addStargateAddress`. Addresses were authored content:
    /// that node and the GM `giveaddress` console command
    /// (`deprecated/python/cell/commands/Player.py:74`) were its only two
    /// callers, so without this action no chain can unlock a destination.
    ///
    /// Grant-only. 2009's node also had a `Remove` port
    /// (`removeStargateAddress`), and no shipped content used it; a
    /// `revoke_stargate_address` verb can be added when a chain needs one.
    GrantStargateAddress { stargate_id: i32 },
}

/// Arithmetic/assignment operation for [`Action::ModifyProperty`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyOp {
    Set,
    Add,
    Subtract,
    Multiply,
}

/// Subset of [`cimmeria_entity::cell_entity::AiState`] reachable from
/// content actions. Other states (Fighting/Leashing/Patrol/Wander/
/// Investigating/Follow/Dead/Spawning) are owned by the runtime and
/// must be reached via their behavior-specific paths so the per-state
/// scratch fields are populated correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcAiStateAction {
    Idle,
    Despawning,
    Submit,
    Error,
}

/// Result of executing a single action.
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// The action completed successfully.
    Success,
    /// The action failed with a descriptive error message.
    Error(String),
    /// The action requests that another chain be evaluated.
    ChainTrigger(i64),
}

impl Action {
    /// Execute this action against the given execution context.
    ///
    /// Most variants are `todo!()` stubs — real execution happens in the
    /// CellService via `resolve_event()` + `execute_actions()`.
    pub fn execute(&self, ctx: &mut ExecutionContext) -> ActionResult {
        let _ = ctx;
        match self {
            Action::TriggerChain { chain_id } => ActionResult::ChainTrigger(*chain_id),
            _ => {
                // All other actions are executed by the CellService via resolve_event().
                // Calling execute() directly on them is not supported for DB-driven chains.
                ActionResult::Error(format!(
                    "Action {:?} must be executed via resolve_event()",
                    self
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod tests;
