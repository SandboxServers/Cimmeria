//! Cell-side entity representation.
//!
//! A `CellEntity` lives on the CellApp and owns the spatial state of a game
//! entity: its position, direction, area-of-interest radius, and the set of
//! other entities that can currently "see" it (witnesses). Property changes
//! on the cell entity may be broadcast to witnesses depending on distribution
//! flags.
//!
//! This corresponds to the C++ `CellEntity` / `Entity` classes in
//! `src/server/CellApp/`.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use crate::abilities::AbilityManager;
use crate::base_entity::PropertyValue;
use crate::missions::MissionManager;
use crate::stats::StatList;

/// One active pulsing-effect instance on an entity (DoT, HoT, timed
/// debuff). Created by `cimmeria_services::cell::effects::register_active_effect`
/// after the initial pulse fires on a `pulse_count > 1` effect, then
/// scheduled per-pulse by `effect_pulse_tick`.
///
/// Lives here (not in services) so the entity field can be typed
/// directly without a circular-dep mirror struct. The scheduling
/// fields are plain stdlib types so no services-side types leak into
/// the entity crate.
#[derive(Debug, Clone)]
pub struct ActiveEffectInstance {
    /// Effect template id (services-side `space_mgr.effect_defs` lookup).
    pub effect_id: i32,
    /// Owning ability id — used for observability and replay.
    pub ability_id: i32,
    /// Entity that applied the effect (drives source attribution).
    pub invoker_id: u32,
    /// Pulses left to fire. Decremented after each pulse; instance is
    /// removed when this hits zero.
    pub remaining_pulses: i32,
    /// Original `pulse_count` from the effect def (observability only).
    pub total_pulses: i32,
    /// When the next pulse should fire (server-local clock).
    pub next_pulse_at: std::time::Instant,
    /// Seconds between pulses. Cached at registration so the per-tick
    /// fire path doesn't re-look up the effect def just to reschedule.
    pub pulse_interval_secs: f32,
    /// Invoker's position at the moment this channel was registered.
    /// `Some` only for channelled effects (`pulse_count == 0`); finite
    /// DoTs leave this `None` because they don't interrupt on caster
    /// movement. The per-tick channel-interrupt sweep diffs this against
    /// `invoker.position` and cancels the channel if the caster moved
    /// more than `CHANNEL_INTERRUPT_DISTANCE` from this anchor.
    pub invoker_position_at_register: Option<Vector3>,
}

mod bandolier;
mod state_flags;
mod system_options;

pub use system_options::SystemOptions;

#[cfg(test)]
mod tests;

/// Filter the active bandolier weapon visual out of a component list
/// when the player is holstered. Returns `components.to_vec()` unchanged
/// when `holstered = false` or when no weapon visual is set.
///
/// **Invariant:** when `weapon_visual` is `Some(v)`, `v` should also
/// appear in `components`. The two are populated by the same DB loader
/// to stay in sync — divergence indicates a data bug upstream (case
/// drift, whitespace, a stale row). We can't fix the upstream data
/// from here, but the filter surfaces the violation: when the
/// post-filter length equals the input length even though we asked
/// for a filter, a `warn`-level log fires so the operator knows the
/// holster filter did not take effect, and a `debug_assert!` panics
/// in debug builds so test runs catch it loudly. Production builds
/// degrade gracefully: the wire `ComponentList` carries the weapon
/// entry through, the client renders armed-pose instead of holstered,
/// and the visual is wrong but nothing crashes.
///
/// Used by both [`CellEntity::appearance_components`] and (via the
/// `cimmeria_services` crate) `PlayerLoadData::appearance_components`
/// so the wire-format holster filter has one implementation, not two.
pub fn filter_holstered_weapon(
    components: &[String],
    weapon_visual: Option<&str>,
    holstered: bool,
) -> Vec<String> {
    let Some(weapon) = weapon_visual.filter(|_| holstered) else {
        return components.to_vec();
    };
    let filtered: Vec<String> = components
        .iter()
        .filter(|c| c.as_str() != weapon)
        .cloned()
        .collect();
    if filtered.len() == components.len() {
        tracing::warn!(
            weapon = %weapon,
            ?components,
            "filter_holstered_weapon: weapon_visual not found in components \
             — invariant violated, holster filter is a no-op for this emit \
             (check DB-side string normalization between weapon_visual and \
             the components query)",
        );
        debug_assert!(
            false,
            "weapon_visual {weapon:?} not found in components {components:?} \
             — invariant violated"
        );
    }
    filtered
}

/// Types of NPC interaction available when a player clicks on this entity.
///
/// Maps to the static interaction set IDs from `python/common/Constants.py`.
#[derive(Debug, Clone, PartialEq)]
pub enum NpcInteractionType {
    /// Dialog NPC — shows `onDialogDisplay` with a dialog tree.
    Dialog { dialog_id: i32 },
    /// Vendor NPC — opens `onStoreOpen` with buy/sell lists.
    Vendor,
    /// Ability trainer — opens `onTrainerOpen` with trainable abilities.
    Trainer { archetype_id: i32 },
    /// Lootable entity — opens `onLootDisplay`.
    Loot,
}

/// The cell-side half of a game entity.
///
/// Manages spatial position, orientation, area-of-interest, and the witness
/// set. Created when a base entity requests a cell presence via
/// `BaseApp::createCellEntity`.
pub struct CellEntity {
    /// Runtime entity ID (matches the corresponding `BaseEntity`).
    pub entity_id: EntityId,

    /// The space (world/zone) this entity currently inhabits.
    pub space_id: SpaceId,

    /// World-space position.
    pub position: Vector3,

    /// Facing direction (unit vector or yaw/pitch/roll encoded).
    pub direction: Vector3,

    /// Current velocity in world units per second.
    /// Sent to clients for interpolation between position updates.
    pub velocity: [f32; 3],

    /// Whether the entity is currently on the ground (affects movement mode).
    pub is_on_ground: bool,

    /// Cell-local property values (CELL_PUBLIC, CELL_PRIVATE, etc.).
    pub properties: HashMap<String, PropertyValue>,

    /// Set of entity IDs that currently have this entity in their AoI.
    ///
    /// When a property with `OTHER_CLIENTS` or `ALL_CLIENTS` distribution
    /// changes, updates are sent to all witnesses.
    pub witnesses: HashSet<EntityId>,

    /// Area-of-interest radius in world units. Other entities within this
    /// radius may become witnesses.
    pub aoi_radius: f32,

    /// Whether this entity has a client controller (i.e., is a player).
    /// Only player entities generate AoI notifications.
    pub is_player: bool,

    /// Entity class ID for CREATE_ENTITY wire format (0x02 = SGWPlayer, 0x04 = SGWMob).
    pub class_id: u8,

    /// Combat stats for this being entity.
    /// Initialized from `SGWBeing.statsTemplate` defaults, overwritten by
    /// archetype values for player entities.
    pub stats: StatList,

    /// Ability manager: known abilities, cooldowns, auto-cycle state.
    pub abilities: AbilityManager,

    /// NPC interaction type (what happens when a player interacts with this entity).
    /// None = no interaction. Only meaningful for NPC entities.
    pub interaction_type: Option<NpcInteractionType>,

    /// Display name for NPCs (sent in dialog headers, etc.).
    pub npc_name: Option<String>,

    /// Mission tracking for player entities.
    pub missions: MissionManager,

    /// Database player_id (for persistence operations). Only set for player entities.
    pub player_id: Option<i32>,

    /// Archetype ID for content engine conditions. Set from character data on connect.
    pub archetype_id: Option<i32>,

    /// Entity level (for XP calculations on kill). Default 1.
    pub level: u32,

    // ── Template-driven fields (populated from DB spawnlist + entity_templates) ──
    /// Source template ID from `entity_templates.template_id`.
    pub template_id: Option<i32>,

    /// Spawn tag from `spawnlist.tag` — used by content chains to target this entity
    /// (e.g., `"ArmYourself_FrostBody"`, `"Preparation_Terminal"`).
    pub tag: Option<String>,

    /// Localized name string ID from `entity_templates.name_id`.
    pub name_id: Option<i32>,

    /// Speaker ID for dialog from `entity_templates.speaker_id`.
    pub speaker_id: Option<i32>,

    /// Event set ID for behavior triggers from `entity_templates.event_set_id`.
    pub event_set_id: Option<i32>,

    /// Raw INT_* interaction type bitfield from `entity_templates.interaction_type`.
    /// Modified at runtime by `SetInteractionType` content actions (OR/AND-NOT).
    pub interaction_type_flags: i64,

    /// Entity flags from `entity_templates.flags`.
    pub entity_flags: u64,

    /// Faction ID from `entity_templates.faction` (0=neutral, 1=Tau'ri, 3=SGC, 10=hostile).
    pub faction: u8,

    /// Alignment ID from `entity_templates.alignment`.
    pub alignment: u8,

    /// Always-available interaction set IDs from `entity_templates.static_interaction_sets`.
    pub static_interaction_sets: Vec<i32>,

    /// Whether space scripts modify this entity's properties dynamically.
    pub has_dynamic_properties: bool,

    /// Per-player available interactions: template_id → Vec<(dialog_set_map_id, dialog_id, interaction_flags)>.
    /// Populated by `add_dialog_set` content action. Only used for player entities.
    pub available_interactions: HashMap<i32, Vec<(i32, i32, i64)>>,

    /// Static mesh path for non-humanoid entities (e.g., `"CA-Props.CA-PrisonerCorpse00"`).
    pub static_mesh: Option<String>,

    /// Body set path from `entity_templates.body_set`.
    pub body_set: Option<String>,

    /// Visual component paths from `entity_templates.components`.
    /// For players this is the FULL merged list — base equipment + active
    /// bandolier slot weapon. `weapon_visual` (below) names the entry that
    /// represents the active weapon so [`Self::appearance_components`] can
    /// filter it out when the player is holstered. NPC entities never have
    /// `weapon_visual` set; their `components` go out as-is.
    pub components: Vec<String>,

    /// The visual component string for the player's active bandolier-slot
    /// weapon, if any. Used as a filter target by
    /// [`Self::appearance_components`] when [`Self::weapon_holstered`] is true.
    ///
    /// Set at player-load time from the bandolier-slot row of the
    /// equipment-visual query. `None` for: NPCs, players with an empty
    /// active bandolier slot, players who haven't completed world entry yet.
    ///
    /// **Why a separate field rather than e.g. a tagged index into
    /// `components`:** weapon swaps (active bandolier slot change) and
    /// inventory edits both can reshuffle `components`. A by-value filter
    /// target is stable across those mutations as long as the underlying
    /// visual path string doesn't collide with another component (and
    /// visual paths under `AR_*` are weapon-specific by convention).
    pub weapon_visual: Option<String>,

    /// Whether the active weapon is currently holstered (hidden from
    /// the wire `ComponentList`).
    ///
    /// Drives the visual stance the client renders: the SGW BigWorld
    /// client's `CompositedAppearanceProxy::ApplyToPawn`
    /// (`ghidra://SGW.exe@0x00ec0840`) writes `entity+0x3D2` with a
    /// weapon-category code derived from whichever weapon-shaped entry it
    /// finds in `BeingAppearance.ComponentList`, defaulting to
    /// `WEAP_Melee = 4` when none is present. So toggling this bool +
    /// rebroadcasting `BeingAppearance` is what flips the animation
    /// system between armed-stance and holstered-stance pose.
    ///
    /// `BSF_Holster` (bit 8 of `state_field`) is **not** read by the
    /// client — verified statically against `GameBeing_OnStateFieldUpdate`
    /// at `ghidra://SGW.exe@0x00e01c90`, which dispatches only on bits
    /// 0/1/2/3/4/5/6/7. The dead bit was removed; this bool replaces it
    /// as the canonical server-side holster state.
    pub weapon_holstered: bool,

    // ── Being state ─────────────────────────────────────────────────────────
    /// State field bitfield (EStateField flags from Atrea.enums).
    /// Bit 0: BSF_Dead, Bit 1: BSF_AutoCycling, Bit 2: BSF_Crouching,
    /// Bit 3: BSF_InCombat, Bit 4: BSF_PlayingMinigame, Bit 5: BSF_InStealth,
    /// Bit 6: BSF_MovementLock, Bit 7: BSF_Walking.
    ///
    /// **Bit 8 was BSF_Holster and is now unused.** The SGW client
    /// doesn't test bit 8 anywhere — verified statically against
    /// `GameBeing_OnStateFieldUpdate` at `ghidra://SGW.exe@0x00e01c90`,
    /// which dispatches only on bits 0-7. Server-side holster state
    /// lives on `weapon_holstered` (below) and drives
    /// `BeingAppearance.ComponentList` instead. See
    /// `docs/architecture/state-field-bits.md` for the verified bit
    /// layout.
    ///
    /// **Read** this field directly for serialization, AoI updates, and
    /// `is_dead` checks. **Writes** depend on the flag's source model — see
    /// the helper-block doc on [`Self::set_state_flag`] for the cutoff
    /// between ref-counted flags (multi-source: BSF_DEAD, BSF_MOVEMENT_LOCK)
    /// and raw-bitmask flags (idempotent input or externally deduped).
    pub state_field: u32,

    /// Per-flag set/unset counter, keyed by the bit *mask* (e.g. `1 << 6`
    /// for BSF_MovementLock — same shape as the constants in
    /// `crate::cell::combat::state`). Only flags managed by
    /// [`Self::set_state_flag`] / [`Self::unset_state_flag`] populate this
    /// map; flags written via raw bitmask ops never touch it.
    ///
    /// When two sources both set a counted flag, the counter holds 2;
    /// only the second `unset` actually clears the bit on `state_field`.
    /// Matches python's `addMovementLock` (`SGWBeing.py:770-787`) and the
    /// generic `combatantStates` map (`SGWBeing.py:697-734`).
    ///
    /// Single-bit masks only — multi-bit masks would conflate counts across
    /// independent flags. Helpers debug-assert single-bit invariant.
    pub state_flag_counts: HashMap<u32, u32>,

    /// NPC entity IDs that currently have this player on their threat list.
    /// Player entities only — `BSF_InCombat` (bit 3 of `state_field`) is set
    /// while this is non-empty and cleared when it drains. Mirrors the
    /// `threatenedMobs` list on `python/cell/SGWPlayer.py:944-965`.
    pub threatened_mobs: HashSet<u32>,

    /// When `Some(t)`, the player left combat at `t` and the deferred
    /// re-holster scan should fire Phase 1 of the holster transition
    /// (`Item_Unequip` animation) once
    /// `Instant::now() - t >= OOC_HOLSTER_DELAY`. Cleared by
    /// `enter_player_combat` (re-aggro within the grace window cancels
    /// the pending holster).
    ///
    /// Why deferred: chaining mobs (kill A, immediately aggro B 50ms
    /// later) would otherwise produce a visible weapon-down → weapon-up
    /// flicker on every gap. The 10-second window is a UX choice, not
    /// a wire-format constraint — see `cell::combat::threat::OOC_HOLSTER_DELAY`.
    pub combat_exit_at: Option<std::time::Instant>,

    /// When `Some(t)`, Phase 1 of the OOC holster transition has fired
    /// (the `Item_Unequip` animation started playing); Phase 2 (flip
    /// `weapon_holstered = true` + rebroadcast `BeingAppearance` to
    /// remove the weapon mesh) is scheduled for `t`. The two-phase
    /// split exists so the hand-authored holster animation has time to
    /// play with the weapon mesh attached — without it, the mesh
    /// snaps away while/before the animation runs, and the visible
    /// result is "weapon vanishes mid-motion."
    ///
    /// Cleared by `enter_player_combat` (re-aggro mid-animation
    /// cancels both Phase 1 and Phase 2 in lockstep with
    /// `combat_exit_at`) and by any "redraw the weapon" path
    /// (`UpdateBandolierItem`, reload-while-holstered) to prevent a
    /// stale Phase 2 from yanking the mesh away after a re-equip.
    pub holster_animation_complete_at: Option<std::time::Instant>,

    // ── Ammo state ────────────────────────────────────────────────────────────
    //
    // Per-slot ammo lives on `BandolierItem` (`current_ammo`, `cur_ammo_type`)
    // and on the `Stat[AMMO_SLOT_1+slot]` map. Use `active_ammo()`,
    // `active_clip_size()`, `active_ammo_type()`, and `set_slot_ammo()` to
    // read and write — never re-introduce a shadow scalar.
    /// When `Some(t)`, a reload is in progress and the magazine is not yet
    /// available; fire paths must reject until `Instant::now() >= t`. The
    /// reload-completion tick (cell::service::reload_completion_tick) refills
    /// the slot pinned by `reload_slot_id` and clears both fields.
    pub reload_complete_at: Option<std::time::Instant>,
    /// The bandolier slot that initiated the in-flight reload. Pinning to
    /// this slot (rather than `active_bandolier_slot` at completion time)
    /// prevents a mid-reload weapon swap from refilling the wrong magazine.
    /// `None` whenever `reload_complete_at` is `None`.
    pub reload_slot_id: Option<i32>,
    /// When `Some(t)`, the player pressed reload while holstered. Phase A
    /// of the reload (redraw + Item_Equip draw animation) has already
    /// been dispatched; the actual reload start (cooldown timer +
    /// `Item_Reload` animation + ammo refill schedule) is deferred until
    /// `Instant::now() >= t` so the draw animation has time to play out.
    /// The `pending_reload_tick` consumes this stamp by re-invoking the
    /// reload handler, which then runs the normal reload-start path
    /// against an already-drawn weapon.
    pub pending_reload_at: Option<std::time::Instant>,

    /// When `Some(t)`, the player tried to fire an ability while
    /// holstered. The unholster animation (Item_Equip) is playing;
    /// the actual ability dispatch is queued for `t` and is
    /// re-invoked by `pending_attack_tick` with the stashed
    /// `pending_attack_ability_id` / `pending_attack_target_id`.
    ///
    /// Subsequent attack inputs during the draw window are rejected
    /// (the first press locks in the queue) — this matches the user
    /// playtest spec: "should queue the first shot, then it should
    /// proceed normally."
    pub pending_attack_at: Option<std::time::Instant>,
    /// Ability id queued by the attack-while-holstered path. Cleared
    /// by `pending_attack_tick` when it dispatches the deferred call.
    pub pending_attack_ability_id: Option<i32>,
    /// Target id queued by the attack-while-holstered path. `0` is a
    /// valid "self-cast or no-target" sentinel — match the wire-arg
    /// semantics of `useAbility(abilityId, targetId)`.
    pub pending_attack_target_id: Option<i32>,

    /// When `Some(t)`, the player pressed F1/F2/F3/F4 to swap to a
    /// different bandolier slot while their current slot also had a
    /// weapon. The holster animation (Item_Unequip) for the old
    /// weapon is playing; the actual slot change (active slot
    /// update + Item_Equip for the new weapon) is deferred until
    /// `Instant::now() >= t` so the holster motion plays out before
    /// the unholster.
    ///
    /// `pending_slot_swap_tick` consumes this stamp by re-invoking
    /// `handle_request_active_slot_change` with the target slot,
    /// which then runs the normal slot-change path (the choreography
    /// branch is gated by `pending_slot_swap_at.is_some()` so the
    /// re-entry skips it).
    pub pending_slot_swap_at: Option<std::time::Instant>,
    /// Target bandolier slot queued by the swap choreography. Cleared
    /// by `pending_slot_swap_tick` when it finalizes the swap.
    pub pending_slot_swap_target: Option<i32>,

    /// Entity ids that died as cone-AoE secondaries during the most
    /// recent `handle_use_ability` call on this attacker. Drained
    /// immediately afterward by `handle_use_ability_with_kill_credit`
    /// to fire `entity_death` content events for each. Kept on the
    /// attacker (instead of returned through the call signature) so
    /// `handle_use_ability` can stay infallible-friendly for the many
    /// callers that don't care about kill credit (NPC AI fire, etc.).
    pub last_aoe_deaths: Vec<u32>,

    /// Pulsing effects currently active on this entity (DoT, HoT,
    /// timed debuffs). Each instance carries its own scheduling, so
    /// the per-cell `effect_pulse_tick` walks this Vec to fire due
    /// pulses and remove completed instances. Empty Vec for the vast
    /// majority of entities (only those who've been hit with a
    /// `pulse_count > 1` effect carry any). See
    /// [`ActiveEffectInstance`] for the per-instance state.
    pub active_effects: Vec<ActiveEffectInstance>,

    // ── NPC AI state ──────────────────────────────────────────────────────────
    /// AI state for NPC entities. See [`AiState`] for the full 12-state
    /// machine. Defaults to `Idle`; only a subset is currently driven by
    /// the runtime (`Idle`, `Fighting`, `Leashing`, `Dead`). The other
    /// variants are entry points for future content hooks and AI tick
    /// extensions.
    pub ai_state: AiState,
    /// Threat list: entity_id → accumulated threat value.
    pub threat_list: HashMap<u32, f32>,
    /// Position where this NPC was spawned (for leashing).
    pub spawn_position: Option<Vector3>,
    /// Direction (`[pitch, yaw, roll]` in radians) the NPC was facing at
    /// spawn time. Captured from `spawnlist.heading` for record-spawned
    /// NPCs, defaulted to `Vector3::zero()` for the bare-spawn path.
    /// Restored on respawn so the NPC doesn't snap to facing-north every
    /// time it comes back — combat sets `direction` to face the target,
    /// and without this snapshot the respawn would lose the original
    /// heading.
    pub spawn_direction: Option<Vector3>,
    /// Ticks until next AI action (count-down from ai tick interval).
    pub ai_cooldown_ticks: u32,
    /// Deadline for the next AI fight tick on this NPC, in service-local
    /// `Instant`. `Some(t)` means "run `npc_ai_fight` on this NPC the
    /// moment `now >= t`, even if the natural 2-second AI cadence hasn't
    /// fired yet." Cleared by the retry-sweep when it consumes the entry,
    /// or set to `Some(now + 500ms)` by `npc_ai_fight` when
    /// `handle_use_ability` returns false (target dead between range
    /// check and launch, animation lock, etc.). Mirrors the
    /// `Atrea.addTimer(t + 0.5, doAiAction)` pattern from the Python
    /// fork: the original 2-second cadence sometimes leaves the NPC
    /// visibly idle for a full tick after a launch failure; the retry
    /// closes that "dead frame" gap. None = follow the natural cadence
    /// only (default for healthy NPCs).
    pub ai_retry_at: Option<std::time::Instant>,
    /// Navmesh path waypoints the NPC is currently following.
    /// Empty = not moving. Each tick pops the next waypoint off the front.
    /// Stored as `VecDeque` so per-tick `pop_front` is O(1) instead of the
    /// O(n) shift `Vec::remove(0)` would do.
    pub nav_path: VecDeque<Vector3>,
    /// Movement speed in world units per tick.
    pub move_speed: f32,
    /// Pin this NPC to its spawn position. AI will attack when the target
    /// is in range + LOS but never pathfind. Loaded from `spawnlist.is_stationary`.
    pub is_stationary: bool,
    /// NPC behavior-aggression level. `0` = passive (only fights back when
    /// threatened, the default). `≥1` = hostile-on-sight (the AI idle tick
    /// scans the NPC's witnesses for opposing-faction players and seeds
    /// threat to wake the NPC up). Set by the `set_aggression` content
    /// action when a mission chain wants an NPC to start hunting — e.g.,
    /// chain 1032 flips the Castle_CellBlock prisoner-retrieval drone to
    /// `1` when the player grabs the Ambernol vial. Persists across kills
    /// so a stationary drone re-aggros the next player who walks in.
    ///
    /// Distinct from any UI-layer aggression-override (nameplate color);
    /// this field drives combat behavior, not the friend/foe indicator.
    pub aggression: i32,
    /// Last `MobMovementType` broadcast to AoI witnesses via
    /// `setMovementType`. `None` = nothing broadcast yet (initial state)
    /// or last broadcast was a "clear" (entering Idle / Dead /
    /// Despawning). Cached so re-entering the same state from the AI
    /// tick doesn't re-spam the wire — only state *changes* fan out.
    ///
    /// **Ownership**: this cache is written exclusively by
    /// [`cell::abilities::messaging::broadcast_movement_type`]. The two
    /// legitimate call sites are (a) the NPC AI tick, which broadcasts
    /// on every state transition (Fighting entry, Leashing entry,
    /// Idle entry), and (b) the inbound `setMovementType` cell-method
    /// handler, which routes the inbound byte through the same helper
    /// so the dedup is consistent in both directions. **Server-side
    /// callers that want to set a movement type must go through the
    /// helper, not write this field directly** — direct writes bypass
    /// the AoI broadcast and the dedup, producing a server-thinks-A /
    /// client-thinks-B divergence.
    ///
    /// Server-side only; never persisted, never restored across login.
    pub last_movement_type: Option<MobMovementType>,
    /// Resolved respawn delay for this NPC, in seconds.
    ///
    /// Loaded at spawn time from
    /// `spawnlist.respawn_secs` (per-spawn override) ?? `entity_templates.respawn_secs`
    /// (per-template default). `None` on both → one-shot mob: corpse
    /// stays on the ground, the spawn position never repopulates.
    /// Player entities and all non-spawned NPCs keep `None`.
    pub respawn_secs: Option<u32>,
    /// Deadline at which the `npc_respawn_tick` should bring this NPC
    /// back to life. `Some(t)` is set by `damage_apply` at the kill
    /// site when `respawn_secs.is_some()`. Cleared by the respawn tick
    /// once consumed, or by a manual GM revive command (future).
    ///
    /// Storing this on `CellEntity` rather than a separate respawn
    /// queue means there's no chance of a queue/entity divergence
    /// (e.g. queue still holds an NPC the space removed via despawn).
    pub respawn_at: Option<std::time::Instant>,
    /// Snapshot of `interaction_type_flags` at spawn time. The death
    /// path OR-merges `INT_NormalLoot` into the live flags so the
    /// corpse becomes lootable; on respawn we have to restore the
    /// pre-death value rather than carry the OR-merged bits forward
    /// (otherwise a respawned mob would still show the loot
    /// interaction cursor and try to hand out a fresh loot table on
    /// right-click). Set once at spawn, read once by `npc_respawn_tick`.
    pub original_interaction_type_flags: i64,
    /// Ordered patrol waypoints for this NPC. Loaded from
    /// `entity_templates.patrol_path_id` → `point_set_points` at
    /// spawn time. Empty for non-patrolling NPCs (the common case).
    ///
    /// The patrol AI handler advances `patrol_next_index` modulo
    /// `patrol_path.len()` so the route loops. Threat preemption
    /// (NPC enters Fighting) does NOT reset the index — when the
    /// fight ends and the NPC re-enters Patrol, it resumes from
    /// where it left off.
    pub patrol_path: Vec<Vector3>,
    /// Index of the next waypoint this NPC should walk to. Always
    /// `< patrol_path.len()` when the patrol path is non-empty;
    /// wraps modulo the path length when the last waypoint is
    /// consumed. Persists across Patrol → Fighting → Patrol
    /// transitions so re-engagement resumes mid-route.
    pub patrol_next_index: usize,
    /// Dwell deadline: when `Some(t)`, the NPC is paused at the
    /// most-recently-reached waypoint until `now >= t`. Set when
    /// the path-follower pops the last `nav_path` entry equal to
    /// the waypoint position. Cleared by the patrol handler when
    /// it pushes the next waypoint into `nav_path`.
    pub patrol_dwell_until: Option<std::time::Instant>,
    /// Per-template patrol dwell duration. Copied from
    /// `entity_templates.patrol_point_delay` (defaulted to 2.0
    /// when NULL). Read by the patrol handler when it stamps a
    /// fresh `patrol_dwell_until` deadline.
    pub patrol_point_delay_secs: f32,
    /// Wander radius in world units. `0.0` → NPC doesn't wander.
    /// Positive values opt the NPC into `AiState::Wander` from
    /// Idle when it has no patrol_path and no positive aggression.
    /// Loaded from `entity_templates.wander_radius`.
    pub wander_radius: f32,
    /// Random-dwell lower bound between successive wander hops,
    /// in seconds. The handler samples a uniform value in
    /// `[min, max]` and stamps `wander_next_at = now + sample`
    /// when it queues a fresh waypoint.
    pub wander_min_dwell_secs: f32,
    /// Random-dwell upper bound between successive wander hops,
    /// in seconds. The DB CHECK guarantees `min <= max`.
    pub wander_max_dwell_secs: f32,
    /// Deadline at which the wander handler may pick a fresh
    /// waypoint. `None` on first entry (handler picks immediately
    /// and stamps a deadline for the next pass).
    pub wander_next_at: Option<std::time::Instant>,
    /// Investigating-state point of interest in world coordinates.
    /// Set by the `SetNpcPoi` content action; consumed by the
    /// `npc_ai_investigate` handler which pathfinds to the POI,
    /// dwells, then returns to Idle (clearing the field).
    pub poi: Option<Vector3>,
    /// Deadline at which the investigation dwell ends and the NPC
    /// returns to Idle. Set by the investigate handler when it
    /// pops the last nav_path entry at the POI.
    pub investigate_until: Option<std::time::Instant>,
    /// Entity ID of the player or NPC this entity is currently
    /// following. Set by the `SetFollowTarget` content action;
    /// cleared when the target disappears or by an explicit
    /// `SetFollowTarget` with a None target.
    pub follow_target_id: Option<u32>,
    /// Lower bound of the follow distance band, in world units.
    /// Loaded from `entity_templates.follow_min_distance`.
    pub follow_min_distance: f32,
    /// Upper bound of the follow distance band, in world units.
    /// Loaded from `entity_templates.follow_max_distance`. The
    /// follow handler walks toward the target whenever the
    /// distance exceeds this.
    pub follow_max_distance: f32,

    // ── Saved mission state (for re-login) ────────────────────────────────────
    /// Saved missions loaded from DB, to be populated before content engine fires.
    pub saved_missions_loaded: bool,

    // ── Loot state ────────────────────────────────────────────────────────────
    /// Loot table ID from `entity_templates.loot_table_id`.
    pub loot_table_id: Option<i32>,
    /// Generated loot items on this corpse (populated on NPC death).
    pub loot: Vec<LootItem>,
    /// Next loot index counter (matches Python `Lootable.nextLootIndex`).
    pub next_loot_index: i32,
    /// Entity ID of the NPC this player is currently looting (only for player entities).
    /// Set when the player interacts with a lootable corpse, cleared on loot window close.
    /// Reference: `python/cell/SGWPlayer.py:setLooting()`
    pub looting_entity: Option<u32>,

    /// Entity ID of the NPC the player most recently interacted with
    /// (player entities only).
    ///
    /// Set on every successful `handle_interact`. Read by paths that need
    /// to know "which NPC is on the other side of this conversation" but
    /// don't get that ID through their own wire frame — notably
    /// `handle_initial_response` (the client sends only
    /// `interaction_set_map_id`, mirroring python's
    /// `SGWPlayer.initialResponse` which used `self.lastInteractionTarget`)
    /// and content-engine `display_dialog` / `start_dialog` actions fired
    /// from chains that don't carry `target_entity_id` in params (e.g. a
    /// follow-up dialog triggered by `OnDialogChoice`).
    ///
    /// The wire `EntityId` field of `onDialogDisplay` must be the NPC's
    /// entity ID, not the player's — the client looks it up in the AoI
    /// listener table via `LookupEntityListenerEntry` to bind the dialog
    /// portrait actor (see
    /// `docs/reverse-engineering/findings/dialog-portrait-lookup.md`).
    pub last_interaction_target: Option<u32>,

    /// Entity ID of the currently-open vendor (only for player entities).
    pub vendor_entity: Option<u32>,

    /// Entity ID of the player's currently-selected target on the client
    /// (player entities only).
    ///
    /// Source of truth: written by the `setTargetID` cell method (index 0
    /// on the `SGWBeing` interface) every time the client's targeting
    /// reticle moves to a new entity. `None` when the player has no
    /// target selected (the client sends `setTargetID(0)` to deselect).
    ///
    /// Used by the auto-cycle loop driver as the live re-fire target so
    /// the player can switch targets mid-loop without breaking it (and
    /// so the loop pauses when the player deselects). Mirrors python's
    /// `self.entity().targetId` live read inside `abilityCooledDown`.
    /// Reference: `python/cell/SGWBeing.py:setTarget`.
    pub current_target_id: Option<i32>,

    /// Currently-active bandolier slot (0-based index).
    pub active_bandolier_slot: i32,

    // ── Ring transporter state ──────────────────────────────────────────────
    /// Region ID of the ring pad the player is currently interacting with.
    /// Set when the player triggers a ring switch (Python `interact()`),
    /// cleared when they pick a destination (Python `selectDestination`).
    /// `None` whenever no ring UI is active for this player.
    pub ring_source_id: Option<i32>,
    /// Destination ring region ID after a successful selection — used by the
    /// destination ring's `playerLoaded` callback to route the loaded player
    /// to the correct waiting list. Cleared in `playerLoaded`.
    pub destination_ring_id: Option<i32>,

    /// Player's bandolier items (quick-access equipment slots).
    pub bandolier_items: HashMap<i32, BandolierItem>,

    /// Slot ids whose `current_ammo`/`cur_ammo_type` have changed since the
    /// last persistence flush. Stage A wires this in but doesn't drain it —
    /// stages B/C/D read/clear this on reload completion, slot swap, ammo
    /// change, and logout.
    pub bandolier_ammo_dirty: HashSet<i32>,

    /// Per-entity content-engine counter values, keyed by counter name.
    /// Mutated by `Action::IncrementCounter` / `Action::ResetCounter` and
    /// read by `Condition::Counter` (populated into the chain context as
    /// `counter_<name>` by `populate_mission_context`). Used to track
    /// kill counts and similar progression state — e.g., the Mess Hall
    /// (target_value 2) completion chain reads
    /// `counter_messhall_kills >= 1` to fire on the kill that brings
    /// the counter to 2 alongside the increment chain. The `target - 1`
    /// threshold is load-bearing because chain conditions evaluate
    /// before sibling actions execute (see
    /// `executor.rs::IncrementCounter` for the full ordering note).
    ///
    /// Not persisted: counters are mission-scoped and intended to be
    /// transient. The chain that reaches the threshold also explicitly
    /// resets the counter via `Action::ResetCounter`.
    pub counters: HashMap<String, i32>,

    /// Per-session client option state populated by `updateSystemOptions`
    /// (player method index 93). Defaults to `SystemOptions::default()` on
    /// entity construction, then overwritten by either of two paths:
    ///
    /// - **Hydrate-on-login** — `InitPlayerState` carries the persisted
    ///   values loaded from `sgw_player.auto_reload` and
    ///   `sgw_player.reload_on_activate`.
    /// - **Live update** — the client's `updateSystemOptions` push, which
    ///   also fires a `CellToBaseMsg::SystemOptionsUpdate` so the row is
    ///   written back to the DB on every toggle.
    ///
    /// Currently honours `autoReload` and `reloadOnActivate` — the only
    /// two options the 2009 XML marks `serverSynch='true'`. Extending to
    /// new server-synced options is mechanical — see
    /// [`SystemOptions::apply`].
    pub system_options: SystemOptions,
}

/// An item in a dead NPC's loot list, ready for display to players.
///
/// Reference: `python/cell/interactions/Lootable.py:LootableItem`
#[derive(Debug, Clone)]
pub struct LootItem {
    /// Item design ID, or None for naquadah (cash).
    pub design_id: Option<i32>,
    /// Quantity of this item/cash.
    pub quantity: i32,
    /// Unique index within this loot list (1-based, sent to client).
    pub index: i32,
}

/// An item slot in the player's bandolier (quick-access equipment bar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandolierItem {
    /// Item design ID.
    pub item_id: i32,
    /// Magazine/clip size for weapons.
    pub clip_size: i32,
    /// Default ammo type for weapons.
    pub default_ammo_type: i32,
    /// Remaining ammo in this slot's magazine. Per-slot, persisted as
    /// `sgw_inventory.ammo`.
    pub current_ammo: i32,
    /// Currently selected ammo subtype (defaults to `default_ammo_type`).
    /// Persisted per-slot — players can pick a non-default subtype per weapon.
    pub cur_ammo_type: i32,
}

/// NPC AI state machine. Discriminants match `Atrea.enums.AI_STATE_*` in
/// `deprecated/python/Atrea/enums.py:228-239` so an `as u8` cast yields
/// the same byte the original SGW server would have produced — used by
/// the `setMovementType` broadcast helper for the subset of states with
/// a corresponding `EMobMovementType` value.
///
/// # Wire animation mapping
///
/// State transitions broadcast `setMovementType` to AoI witnesses so
/// the client picks the matching animation. Mapping:
///
/// | Server state       | Wire byte                        | Client animation    | Driven? |
/// |--------------------|----------------------------------|---------------------|---------|
/// | `Fighting` (entry) | `MobMovementType::CombatAdvance` | Combat-stance walk  | Yes     |
/// | `Leashing` (entry) | `MobMovementType::Leash`         | Leash-back trot     | Yes     |
/// | `Idle` (entry)     | None (clears cached state)       | (client keeps prev) | Yes     |
/// | `Patrol` (entry)   | `MobMovementType::Patrol`        | Patrol walk         | No (Phase 2) |
/// | `Wander` (entry)   | `MobMovementType::Wander`        | Wander idle-walk    | No (Phase 3) |
/// | `Follow` (entry)   | `MobMovementType::Follow`        | Follow gait         | No (Phase 6) |
/// | `Dead` / `Spawning` / `Investigating` / `Despawning` / `Submit` / `Error` | None | (client keeps prev) | (no transition) |
///
/// "Driven? = No" rows are encoded but their AI tick handlers haven't
/// landed yet — broadcasting on entry is the planned mapping; the
/// state transitions themselves don't fire today. The respawn tick
/// clears `last_movement_type` on Dead → Idle so the next Fighting
/// entry re-broadcasts CombatAdvance.
///
/// `Spawning` is preserved as a variant for completeness with the source
/// enum but is **never entered at runtime in Rust**. The Python original
/// used it to seed weapon ammo before the first Idle tick; the Rust
/// fire-gate short-circuits on `!is_player`, so ammo seeding is moot and
/// new NPCs start at `Idle`. Future spawn-VFX hooks (e.g., Goa'uld
/// ribbon-device reveal) have a clean place to plug in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AiState {
    Spawning = 0,
    Idle = 1,
    Investigating = 2,
    Fighting = 3,
    Leashing = 4,
    Dead = 5,
    Despawning = 6,
    Follow = 7,
    Patrol = 8,
    Wander = 9,
    Submit = 10,
    Error = 11,
}

/// Mob movement-type byte broadcast via `setMovementType` (`SGWBeing`
/// interface, method index 1). Discriminants match
/// `entities/defs/enumerations.xml:1593-1604` (`EMobMovementType`) so an
/// `as u8` cast yields the byte the client expects on the wire.
///
/// The client uses this **purely for animation selection** (run vs walk
/// vs combat-stance vs leashed-trot) — gameplay-side movement is fully
/// server-authoritative. Confirmed by Ghidra: `FUN_00deb660` in SGW.exe
/// switches on this byte to format the debug labels "Entity: %d is
/// patroling", "...leashing", etc. (strings at `019d2ca4`..`019d2e20`).
///
/// Not all `AiState` values have a movement type — Idle / Spawning /
/// Dead / Despawning / Submit / Error broadcast `None` (which the
/// helper translates to "clear cached, no wire send" — the client
/// defaults to the appearance the entity already had).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MobMovementType {
    Cover = 0,
    CombatAdvance = 1,
    Patrol = 2,
    Follow = 3,
    Wander = 4,
    Leash = 5,
    Avoid = 6,
}

impl CellEntity {
    /// Create a new cell entity at the given position in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId, position: Vector3) -> Self {
        Self {
            entity_id,
            space_id,
            position,
            direction: Vector3::zero(),
            velocity: [0.0; 3],
            is_on_ground: true,
            properties: HashMap::new(),
            witnesses: HashSet::new(),
            aoi_radius: 100.0, // Default AoI radius (matches grid_vision_distance)
            is_player: false,
            class_id: 0x02, // SGWPlayer by default
            stats: StatList::new(),
            abilities: AbilityManager::new(),
            interaction_type: None,
            npc_name: None,
            missions: MissionManager::new(),
            player_id: None,
            archetype_id: None,
            level: 1,
            template_id: None,
            tag: None,
            name_id: None,
            speaker_id: None,
            event_set_id: None,
            interaction_type_flags: 0,
            entity_flags: 0,
            faction: 0,
            alignment: 0,
            static_interaction_sets: Vec::new(),
            has_dynamic_properties: false,
            available_interactions: HashMap::new(),
            static_mesh: None,
            body_set: None,
            components: Vec::new(),
            weapon_visual: None,
            weapon_holstered: true,
            state_field: 0,
            state_flag_counts: HashMap::new(),
            threatened_mobs: HashSet::new(),
            combat_exit_at: None,
            reload_complete_at: None,
            reload_slot_id: None,
            pending_reload_at: None,
            pending_attack_at: None,
            pending_attack_ability_id: None,
            pending_attack_target_id: None,
            pending_slot_swap_at: None,
            pending_slot_swap_target: None,
            last_aoe_deaths: Vec::new(),
            active_effects: Vec::new(),
            holster_animation_complete_at: None,
            ai_state: AiState::Idle,
            threat_list: HashMap::new(),
            spawn_position: None,
            spawn_direction: None,
            ai_cooldown_ticks: 0,
            ai_retry_at: None,
            nav_path: VecDeque::new(),
            move_speed: 0.6, // ~0.6 world units per 100ms tick = 6 units/sec
            is_stationary: false,
            aggression: 0,
            last_movement_type: None,
            respawn_secs: None,
            respawn_at: None,
            original_interaction_type_flags: 0,
            patrol_path: Vec::new(),
            patrol_next_index: 0,
            patrol_dwell_until: None,
            patrol_point_delay_secs: 2.0,
            wander_radius: 0.0,
            wander_min_dwell_secs: 3.0,
            wander_max_dwell_secs: 8.0,
            wander_next_at: None,
            poi: None,
            investigate_until: None,
            follow_target_id: None,
            follow_min_distance: 2.0,
            follow_max_distance: 5.0,
            saved_missions_loaded: false,
            loot_table_id: None,
            loot: Vec::new(),
            next_loot_index: 1,
            looting_entity: None,
            last_interaction_target: None,
            vendor_entity: None,
            current_target_id: None,
            active_bandolier_slot: 0,
            bandolier_items: HashMap::new(),
            bandolier_ammo_dirty: HashSet::new(),
            ring_source_id: None,
            destination_ring_id: None,
            counters: HashMap::new(),
            system_options: SystemOptions::default(),
        }
    }

    /// Update the entity's world-space position.
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// Get the entity's current world-space position.
    pub fn get_position(&self) -> &Vector3 {
        &self.position
    }

    /// Add an entity to the witness set (it can now see this entity).
    pub fn add_witness(&mut self, entity_id: EntityId) {
        self.witnesses.insert(entity_id);
    }

    /// Remove an entity from the witness set (it can no longer see this entity).
    pub fn remove_witness(&mut self, entity_id: EntityId) {
        self.witnesses.remove(&entity_id);
    }

    /// Get the current set of witness entity IDs.
    pub fn get_witnesses(&self) -> &HashSet<EntityId> {
        &self.witnesses
    }

    /// Returns `true` if the given position is within this entity's AoI radius.
    ///
    /// Uses squared distance comparison to avoid a square root.
    pub fn is_in_aoi(&self, other_pos: &Vector3) -> bool {
        self.position.distance_squared_to(other_pos) <= self.aoi_radius * self.aoi_radius
    }

    /// Build the `ComponentList` that should go out in `BeingAppearance`,
    /// applying the current holster state.
    ///
    /// Returns `components` unchanged when not holstered, or `components`
    /// with the `weapon_visual` entry filtered out when holstered. The
    /// SGW BigWorld client's appearance compositor picks weapon-stance
    /// vs. holstered-stance from whichever weapon-shaped entry it finds
    /// in this list (`ghidra://SGW.exe@0x00ec0840`), so omitting the
    /// weapon visual is the wire-format-correct way to render the
    /// holstered pose — it falls back to `WEAP_Melee = 4` and plays the
    /// unarmed-stance animation blend.
    ///
    /// Callers should use this in place of `&entity.components` at every
    /// `BeingAppearance`-emit site. Reading `components` directly is fine
    /// for non-wire purposes (debug logs, AoI propagation copies, NPC
    /// templates that don't have a holster concept).
    pub fn appearance_components(&self) -> Vec<String> {
        filter_holstered_weapon(
            &self.components,
            self.weapon_visual.as_deref(),
            self.weapon_holstered,
        )
    }

    /// Toggle the holster state. Returns `true` if the state actually
    /// changed (the caller should rebroadcast `BeingAppearance`), `false`
    /// if it was already in the requested state.
    ///
    /// The return value gates on state-change ONLY. The `weapon_visual`
    /// check that used to live here was wrong: in production the cell
    /// entity's `weapon_visual` is always `None` (only the base side
    /// populates it from `PlayerLoadData`), so the gate silently
    /// suppressed every holster broadcast. The base-side
    /// `RefreshAppearance` handler has its own change-detection
    /// (`weapon_holstered != cached` short-circuit), so redundant calls
    /// are already a free no-op there — gating here just hid bugs.
    pub fn set_weapon_holstered(&mut self, holstered: bool) -> bool {
        if self.weapon_holstered == holstered {
            return false;
        }
        self.weapon_holstered = holstered;
        true
    }

    /// Lockstep the holster state with `BSF_InCombat`: in-combat = drawn,
    /// out-of-combat = holstered. Returns `true` when the caller should
    /// rebroadcast `BeingAppearance` (state actually flipped AND there's
    /// a `weapon_visual` whose presence in the wire list will change).
    ///
    /// This is the canonical entry point from `enter_player_combat` /
    /// `exit_player_combat`. Keeping the policy here (rather than
    /// duplicating `!in_combat` at every caller) means future tweaks —
    /// e.g. "stay drawn for 5s after leaving combat" — happen in one
    /// place.
    pub fn sync_holster_to_combat(&mut self, in_combat: bool) -> bool {
        self.set_weapon_holstered(!in_combat)
    }

    /// Clear every pending weapon-action timer in one shot.
    ///
    /// Called when the entity transitions to a state where running any of
    /// the deferred weapon flows would be wrong. The current call site is
    /// player death: the cell-side entity survives same-world respawn
    /// (`ReanchorPlayer` keeps it intact), so stale `pending_*_at` fields
    /// would otherwise fire deferred ticks during the Defeat Window or
    /// post-respawn — surfacing as phantom reload animations, queued
    /// attacks against unrelated targets, or slot-swap choreography
    /// running on a corpse.
    ///
    /// Specifically resets:
    /// - `pending_reload_at` — reload-while-holstered Phase A draw window.
    /// - `reload_complete_at` + `reload_slot_id` — reload-while-drawn
    ///   Phase B warmup deadline + pinned slot.
    /// - `pending_attack_at` + `pending_attack_ability_id` +
    ///   `pending_attack_target_id` — fire-while-holstered queued attack.
    /// - `pending_slot_swap_at` + `pending_slot_swap_target` — bandolier
    ///   slot-swap choreography (`Item_Unequip` → wait → `Item_Equip`).
    /// - `holster_animation_complete_at` — OOC holster Phase 2 mesh-drop
    ///   deadline.
    /// - `combat_exit_at` — OOC holster Phase 1 deadline. Combat already
    ///   ended (death cleared `BSF_IN_COMBAT` via the threat fan-out), so
    ///   this is redundant with that path but keeps the surface clean.
    ///
    /// Does NOT clear:
    /// - `weapon_holstered` — the death broadcast path and the respawn
    ///   `ReanchorPlayer` handle the appearance side.
    /// - `threatened_mobs` — held separately and cleared by the
    ///   same-world respawn handler (see
    ///   `cell_methods/player/combat/respawn.rs`).
    /// - `ai_retry_at` / `pending_ai_retries` — NPC-AI fields; the call
    ///   site is gated on `is_player`, so these are structurally
    ///   unreachable here. If the gate is ever relaxed, audit NPC AI
    ///   timers separately rather than bolting them onto this helper.
    pub fn clear_weapon_action_state(&mut self) {
        self.pending_reload_at = None;
        self.reload_complete_at = None;
        self.reload_slot_id = None;
        self.pending_attack_at = None;
        self.pending_attack_ability_id = None;
        self.pending_attack_target_id = None;
        self.pending_slot_swap_at = None;
        self.pending_slot_swap_target = None;
        self.holster_animation_complete_at = None;
        self.combat_exit_at = None;
    }
}

impl std::fmt::Debug for CellEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellEntity")
            .field("entity_id", &self.entity_id)
            .field("space_id", &self.space_id)
            .field("position", &self.position)
            .field("is_on_ground", &self.is_on_ground)
            .field("witness_count", &self.witnesses.len())
            .field("aoi_radius", &self.aoi_radius)
            .field("property_count", &self.properties.len())
            .field("stats", &self.stats)
            .field("known_abilities", &self.abilities.known_count())
            .field("template_id", &self.template_id)
            .field("tag", &self.tag)
            .finish()
    }
}
