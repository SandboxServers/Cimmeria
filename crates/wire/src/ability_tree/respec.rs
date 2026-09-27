//! Trainer respec (`resetMyAbilities`, cell method 72): the price, the
//! base's verdict, and the rejection payload (AT-08, D-AT03, D-AT10).
//!
//! The cell gates the request at a pinned trainer, the base applies the one
//! guarded `UPDATE`, and the cell answers the client from the base's
//! [`RespecOutcome`]. Both sides read the price from here, and so does the
//! `CostToRespec` field of `onTrainerOpen`, so the window shows what the
//! respec charges.

/// Naquadah charged by a respec. D-AT10 keeps the placeholder the Python
/// trainer shipped (`AbilityTrainer.py:42`) until a sourced price exists.
pub const RESPEC_COST_NAQUADAH: i32 = 1000;

/// What the base did with one respec request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespecOutcome {
    /// The row was reset and charged.
    Reset {
        /// The trainer-bought abilities that were removed and refunded
        /// (`trained_abilities` before the `UPDATE`).
        refunded: Vec<i32>,
        /// `training_points` after the refund.
        training_points: i32,
        /// `naquadah` after the charge.
        naquadah: i32,
    },
    /// Nothing was trainer-bought, so nothing changed and nothing was
    /// charged. A replayed respec lands here.
    NothingToReset,
    /// Something was trainer-bought, but the character cannot pay.
    /// Nothing changed.
    NotEnoughNaquadah { naquadah: i32 },
}

// `EConditionHandlerFeedback` values (`entities/defs/enumerations.xml`) for
// a refused respec. The 2009 enum has no respec tokens, so each is the
// closest existing code, the same ones `trainAbility` uses for the same
// remedy (AT-E1 mapping, `docs/analysis/ability-trees/worknotes/at-e1.md`).

/// `CONDITION_FEEDBACK_OutsideDistanceCheck`: not at a trainer, or out of
/// its range. Close fit, as for `trainAbility`.
pub const RESPEC_FEEDBACK_NOT_AT_TRAINER: u16 = 43;
/// `CONDITION_FEEDBACK_EntityDoesNotHaveAbility`: no trainer-bought
/// ability to reset. A documented REUSE: the entity has none of the
/// abilities a respec would remove.
pub const RESPEC_FEEDBACK_NOTHING_TRAINED: u16 = 167;
/// `CONDITION_FEEDBACK_StatValueLessThan`: too little naquadah. A
/// documented REUSE of the generic "value below threshold" comparator, as
/// `trainAbility` uses it for training points; the enum has no currency
/// token.
pub const RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH: u16 = 35;

/// `EErrorCodeSystem::ERRORCODE_SYSTEM_Ability`, the only system the enum
/// defines.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;

/// The seven `onErrorCode` argument bytes for a refused respec:
/// `UINT8 SystemID = 0, INT32 InstanceID = 0, UINT16 ErrorCodeID`,
/// little-endian. `InstanceID` is 0 because a respec names no ability.
pub fn respec_error_code_args(code: u16) -> Vec<u8> {
    let mut args = Vec::with_capacity(7);
    args.push(ERRORCODE_SYSTEM_ABILITY);
    args.extend_from_slice(&0i32.to_le_bytes());
    args.extend_from_slice(&code.to_le_bytes());
    args
}
