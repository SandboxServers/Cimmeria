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

#[cfg(test)]
mod tests;

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
    pub components: Vec<String>,

    // ── Being state ─────────────────────────────────────────────────────────
    /// State field bitfield (EStateField flags from Atrea.enums).
    /// Bit 0: BSF_Dead, Bit 1: BSF_AutoCycling, Bit 2: BSF_Crouching,
    /// Bit 3: BSF_InCombat, Bit 4: BSF_PlayingMinigame, Bit 5: BSF_InStealth,
    /// Bit 6: BSF_MovementLock, Bit 7: BSF_Walking, Bit 8: BSF_Holster.
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

    // ── NPC AI state ──────────────────────────────────────────────────────────
    /// AI state for NPC entities (Idle, Fighting, Dead, Leashing).
    pub ai_state: AiState,
    /// Threat list: entity_id → accumulated threat value.
    pub threat_list: HashMap<u32, f32>,
    /// Position where this NPC was spawned (for leashing).
    pub spawn_position: Option<Vector3>,
    /// Ticks until next AI action (count-down from ai tick interval).
    pub ai_cooldown_ticks: u32,
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

    /// Entity ID of the currently-open vendor (only for player entities).
    pub vendor_entity: Option<u32>,

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

/// NPC AI state machine.
///
/// Reference: `python/Atrea/enums.py:228-239`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Fighting,
    Dead,
    Leashing,
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
            state_field: 0,
            state_flag_counts: HashMap::new(),
            threatened_mobs: HashSet::new(),
            reload_complete_at: None,
            reload_slot_id: None,
            ai_state: AiState::Idle,
            threat_list: HashMap::new(),
            spawn_position: None,
            ai_cooldown_ticks: 0,
            nav_path: VecDeque::new(),
            move_speed: 0.6, // ~0.6 world units per 100ms tick = 6 units/sec
            is_stationary: false,
            saved_missions_loaded: false,
            loot_table_id: None,
            loot: Vec::new(),
            next_loot_index: 1,
            looting_entity: None,
            vendor_entity: None,
            active_bandolier_slot: 0,
            bandolier_items: HashMap::new(),
            bandolier_ammo_dirty: HashSet::new(),
            ring_source_id: None,
            destination_ring_id: None,
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

    // ── State-field flag helpers ─────────────────────────────────────────────
    //
    // Mirror python's per-flag counter pattern (`SGWBeing.py:697-734` for the
    // generic `combatantStates` map; `:770-787` for the dedicated movement-lock
    // counter). Two stuns from different abilities both bump the BSF_MovementLock
    // counter to 2; clearing one drops it to 1 and the bit STAYS set until the
    // second clear drains the counter.
    //
    // **When to use these helpers vs raw bitmask ops:**
    //
    //   - **Use the helpers** for flags with multiple potential set/unset
    //     sources that don't coordinate, where a single source clearing
    //     would silently drop the others' refs. Today: `BSF_DEAD` (death/
    //     respawn pair), `BSF_MOVEMENT_LOCK` (death + future stun/cast/fear).
    //
    //   - **Raw `|=` / `&=` is fine** for flags that are either (a) driven
    //     by an idempotent player input (BSF_CROUCHING, BSF_HOLSTER from
    //     `requestHolsterWeapon` — clicking twice should set, not bump), or
    //     (b) externally managed via a separate dedup mechanism (BSF_IN_COMBAT
    //     gated on `threatened_mobs` non-empty in `combat::threat`).
    //
    // Mixing the two patterns on the same flag will desync the counter from
    // the bit: a raw `|=` doesn't bump the counter, so the next `unset_*`
    // helper sees count==0, takes the no-op branch, and **does not clear the
    // bit** — a real production hazard. If you migrate a flag to the helpers,
    // migrate ALL its writers in the same change, and force-reset via
    // `clear_all_state_flags` on any hard-reset path (respawn, world entry).

    /// Increment the per-flag counter and set the bit on a 0->1 transition.
    /// Returns `true` when the bit transitioned (caller should send
    /// `onStateFieldUpdate`); `false` when the flag was already set by a
    /// prior source.
    pub fn set_state_flag(&mut self, mask: u32) -> bool {
        debug_assert!(
            mask.count_ones() == 1,
            "state_flag helpers require single-bit masks (got {mask:#x}) — multi-bit masks would conflate counts across independent flags"
        );
        let count = self.state_flag_counts.entry(mask).or_insert(0);
        *count += 1;
        if self.state_field & mask == 0 {
            self.state_field |= mask;
            true
        } else {
            false
        }
    }

    /// Decrement the per-flag counter and clear the bit on a 1->0 transition.
    /// Returns `true` when the bit transitioned (caller should send
    /// `onStateFieldUpdate`); `false` when other sources are still holding
    /// the flag set, when no source has set it, or when the bit is clear.
    ///
    /// **A best-effort clear (no prior `set_state_flag`) is a silent no-op.**
    /// The earlier version warned + inserted a 0-entry into the counter map
    /// on every stray clear, which (a) leaked map entries on hot paths like
    /// `npc_ai_leash` that defensively unset flags they may not own, and
    /// (b) buried real desync warnings under the noise. If a caller mixes
    /// raw `|=` with `unset_state_flag`, the bit stays stuck and the
    /// debug_assert on misuse below isn't enough to catch it — that's a
    /// project-policy issue documented in the helper-block doc above.
    pub fn unset_state_flag(&mut self, mask: u32) -> bool {
        debug_assert!(
            mask.count_ones() == 1,
            "state_flag helpers require single-bit masks (got {mask:#x})"
        );
        // Use `get_mut` so missing keys aren't materialized — best-effort
        // clears on flags this entity has never owned should be a no-op,
        // not a map-growth event.
        let Some(count) = self.state_flag_counts.get_mut(&mask) else {
            return false;
        };
        if *count == 0 {
            return false;
        }
        *count -= 1;
        if *count == 0 {
            // Drained the last ref — drop the entry rather than leaving a
            // 0-count straggler so the map stays bounded by the set of
            // flags currently held, not the set ever touched.
            self.state_flag_counts.remove(&mask);
            if self.state_field & mask != 0 {
                self.state_field &= !mask;
                return true;
            }
        }
        false
    }

    /// Force-clear all state flags and counters. Used by respawn paths
    /// where the entity returns to a known-clean state regardless of how
    /// many sources had previously set things. Bypasses ref-counting on
    /// purpose — respawn is a hard reset, not a per-source unwind.
    pub fn clear_all_state_flags(&mut self) {
        self.state_field = 0;
        self.state_flag_counts.clear();
    }

    /// Convenience read: is the given flag bit set?
    pub fn has_state_flag(&self, mask: u32) -> bool {
        self.state_field & mask != 0
    }

    // ── Bandolier ammo helpers ───────────────────────────────────────────────
    //
    // Per-slot ammo lives on `BandolierItem.current_ammo` and is mirrored to
    // the `Stat[AMMO_SLOT_1+slot]` map. These helpers are the read/write path
    // for fire, reload, slot swap, and ammo-change; the shadow scalars that
    // used to live on `CellEntity` were removed in Stage C.

    /// Read the active slot's current ammo, or 0 if no item equipped.
    pub fn active_ammo(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.current_ammo)
    }

    /// Read the active slot's clip size, or 0 if no item equipped.
    pub fn active_clip_size(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.clip_size)
    }

    /// Read the active slot's selected ammo type, or 0 if no item equipped.
    pub fn active_ammo_type(&self) -> i32 {
        self.bandolier_items
            .get(&self.active_bandolier_slot)
            .map_or(0, |i| i.cur_ammo_type)
    }

    /// Set ammo for a slot, mirroring to the AmmoSlot{N} stat. Returns the
    /// clamped value, or `None` if the slot is unequipped.
    ///
    /// Marks the slot dirty in `bandolier_ammo_dirty` for batched persistence.
    pub fn set_slot_ammo(&mut self, slot_id: i32, current: i32) -> Option<i32> {
        let item = self.bandolier_items.get_mut(&slot_id)?;
        item.current_ammo = current.clamp(0, item.clip_size);
        let clamped = item.current_ammo;
        let stat_id = crate::stats::AMMO_SLOT_1 + slot_id;
        if let Some(stat) = self.stats.get_mut(stat_id) {
            stat.set_current(clamped);
        }
        self.bandolier_ammo_dirty.insert(slot_id);
        Some(clamped)
    }

    /// Refill the active slot's magazine to its `clip_size`. Returns the new
    /// ammo value, or `None` if no slot is equipped.
    pub fn refill_active_slot(&mut self) -> Option<i32> {
        let slot = self.active_bandolier_slot;
        let max = self.bandolier_items.get(&slot).map(|i| i.clip_size)?;
        self.set_slot_ammo(slot, max)
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
