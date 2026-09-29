//! The special-ammo reserve round trip (ammo campaign AM-02, issue #1026;
//! D-AM05): `AmmoReserveRequest` (cell to base, carried by
//! `CellToBaseMsg::AmmoReserve`) and `AmmoReserveAnswer` (base to cell,
//! carried by `BaseToCellMsg::AmmoReserve`).
//!
//! The cell owns the clip; the base owns the bag stacks. Each request is one
//! base transaction that moves rounds between the two and writes the
//! weapon's `sgw_inventory.ammo` / `cur_ammo_type` in the same commit, so no
//! round exists in both places or in neither, whatever happens to the cell
//! entity before the answer arrives.
//!
//! Every field is server state the cell read from its own entity (slot,
//! instance, clip, the validated ammo type), never a client payload. The
//! cell flushes the slot's clip (`BandolierAmmoUpdate`) ahead of each
//! request on the same ordered channel, and the base then counts rounds from
//! the weapon row it locks, with the clip size from `resources.items`. So a
//! duplicated request finds a full clip (draw) or a switched row (return)
//! and moves nothing: no round is minted or spent twice.

/// Reserve requests from the cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmmoReserveRequest {
    /// A reload with a special type loaded: draw `clip_size - clip_before`
    /// rounds from the bags and load them into the weapon.
    ReloadDraw {
        entity_id: u32,
        player_id: i32,
        /// Bandolier slot (0-4) of the weapon being reloaded.
        slot_id: i32,
        /// The weapon's `sgw_inventory.item_id`: the row guard.
        instance_id: i32,
        /// `EAmmoType` ordinal being loaded (special).
        ammo_type: i32,
        /// Rounds in the clip when the reload started.
        clip_before: i32,
    },
    /// An ammo-type switch away from a special type: put the clip's unfired
    /// rounds back in the bags and, if they all fit, persist the new type.
    SwitchReturn {
        entity_id: u32,
        player_id: i32,
        slot_id: i32,
        instance_id: i32,
        /// The special type in the clip.
        from_ammo_type: i32,
        /// The type the player picked (already validated by the cell).
        to_ammo_type: i32,
        /// Unfired rounds of `from_ammo_type`.
        rounds: i32,
    },
}

/// Why a reserve request moved no rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveRefusal {
    /// The bags hold no round of the type (`reasons::STACK_EMPTY`).
    StackEmpty,
    /// The weapon row is no longer in that bandolier slot, or (for a switch
    /// return) no longer holds the type being returned.
    WeaponChanged,
    /// No database, or a query failed; nothing was committed.
    DbError,
}

impl ReserveRefusal {
    /// Stable `reason=` string.
    pub fn reason(self) -> &'static str {
        match self {
            ReserveRefusal::StackEmpty => "stack_empty",
            ReserveRefusal::WeaponChanged => "weapon_changed",
            ReserveRefusal::DbError => "db_error",
        }
    }
}

/// Reserve answers from the base, one per request, sent after the commit
/// (or the rollback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmmoReserveAnswer {
    /// The answer to `ReloadDraw`. On `Ok`, `drawn > 0` rounds were removed
    /// from the bags and the weapon row now holds `clip_before + drawn`.
    ReloadDrawn {
        entity_id: u32,
        player_id: i32,
        slot_id: i32,
        instance_id: i32,
        ammo_type: i32,
        drawn: i32,
        /// Rounds of the type left in the bags.
        stack_after: i32,
        result: Result<(), ReserveRefusal>,
    },
    /// The answer to `SwitchReturn`. `rounds` is what the weapon row held
    /// (on `Err`, the request's value); `returned + remainder == rounds` on
    /// `Ok`. `remainder == 0` means the switch took effect (the weapon row
    /// holds 0 rounds of `to_ammo_type`); otherwise the bags were full, the
    /// weapon keeps `remainder` rounds of `from_ammo_type` and the switch
    /// did not happen (D-AM05: never deleted, never relabelled). On `Err`
    /// nothing moved.
    SwitchReturned {
        entity_id: u32,
        player_id: i32,
        slot_id: i32,
        instance_id: i32,
        from_ammo_type: i32,
        to_ammo_type: i32,
        rounds: i32,
        returned: i32,
        remainder: i32,
        result: Result<(), ReserveRefusal>,
    },
}

impl AmmoReserveRequest {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            AmmoReserveRequest::ReloadDraw { .. } => "reload_draw",
            AmmoReserveRequest::SwitchReturn { .. } => "switch_return",
        }
    }
}

impl AmmoReserveAnswer {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            AmmoReserveAnswer::ReloadDrawn { .. } => "reload_drawn",
            AmmoReserveAnswer::SwitchReturned { .. } => "switch_returned",
        }
    }
}
