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

use serde::{Deserialize, Serialize};

use cimmeria_common::Vector3;

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

mod appearance;
mod bandolier;
mod construction;
mod entity_struct;
mod state_flags;
mod system_options;
mod weapon_action;
mod witness_aoi;

pub use appearance::filter_holstered_weapon;
pub use entity_struct::CellEntity;
pub use system_options::SystemOptions;

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
    /// The `sgw_inventory.item_id` value — the per-row *instance* id of the
    /// physical item row backing this slot. It's a server-allocated surrogate
    /// (sequence default, never reused) and the table's declared primary key
    /// (`sgw_inventory_pkey PRIMARY KEY (item_id)`, `db/sgw/_primary_keys.sql`;
    /// the child table carries its own PK because a parent PK doesn't span
    /// `INHERITS` children). The `local_id_check` CHECK pins real rows to
    /// `item_id >= 10000`, so the optimistic-grant `instance_id: 0` sentinel
    /// can never collide with a live row. This is the TOCTOU guard for
    /// ammo persistence: it flows into `BandolierAmmoUpdate.expected_instance_id`
    /// and the WHERE clause of `update_bandolier_ammo`. Two physical items of
    /// the same weapon *design* share `item_id` (the design id below) but have
    /// distinct `instance_id`, so keying the persist guard on this — not the
    /// design id — is what closes the same-type-swap dupe window.
    pub instance_id: i32,
    /// Item *design* ID (`resources.items.item_id` / `sgw_inventory.type_id`).
    /// Used for design lookups (clip size, abilities, holster animation, ammo
    /// type). NOT unique per physical item — do NOT use as the ammo-persist
    /// TOCTOU guard; use `instance_id` for that.
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
/// | Server state            | Wire byte                        | Client animation    |
/// |-------------------------|----------------------------------|---------------------|
/// | `Fighting` (entry)      | `MobMovementType::CombatAdvance` | Combat-stance walk  |
/// | `Leashing` (entry)      | `MobMovementType::Leash`         | Leash-back trot     |
/// | `Patrol` (entry)        | `MobMovementType::Patrol`        | Patrol walk         |
/// | `Wander` (entry)        | `MobMovementType::Wander`        | Wander idle-walk    |
/// | `Follow` (entry)        | `MobMovementType::Follow`        | Follow gait         |
/// | `Investigating` (entry) | `MobMovementType::CombatAdvance` | Alert advance (closest match) |
/// | `Idle` / `Submit` / `Despawning` (entry) | None (clears cache) | (client keeps prev) |
/// | `Dead` / `Spawning` / `Error` | None | (client keeps prev — no transition fires from these states) |
///
/// `Investigating` uses `CombatAdvance` because no dedicated
/// investigate byte exists in `EMobMovementType`; the alert-advance
/// animation it implies is the closest semantic match.
/// The respawn tick clears `last_movement_type` on Dead → Idle so
/// the next behavior-state entry re-broadcasts cleanly.
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
