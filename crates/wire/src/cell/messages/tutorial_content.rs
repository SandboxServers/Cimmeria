//! The content engine's `show_tutorial` action (Class Start v6, CS-03).
//!
//! The cell sends [`RecordTutorialShown`] for a player who has not seen the
//! tutorial yet; the base inserts `(player_id, tutorial_id)` into
//! `sgw_player_tutorials` with `ON CONFLICT DO NOTHING` and answers with
//! [`TutorialRecorded`]. The cell displays the tutorial dialog only on
//! [`TutorialRecordOutcome::First`], so the database decides "first time"
//! and a relog or world change can never show it again.

/// `CellToBaseMsg::RecordTutorialShown`: record that `player_id` has been
/// shown `tutorial_id`, once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordTutorialShown {
    pub entity_id: u32,
    /// The character the cell resolved. The base writes only when the
    /// entity's session still plays it.
    pub player_id: i32,
    /// The chain that fired, for the telemetry and the reply.
    pub chain_id: i64,
    /// The tutorial's `resources.dialogs.dialog_id`.
    pub tutorial_id: i32,
}

/// What the base's insert did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TutorialRecordOutcome {
    /// The row is new: this is the first time, so the cell shows it.
    First,
    /// The row already existed (a relog raced the cell's set, or a second
    /// request was already in flight). The cell shows nothing.
    AlreadyShown,
    /// Nothing was written: no database, no session for the entity, the
    /// session plays another character, or the insert failed. The cell
    /// shows nothing and forgets its optimistic mark, so a later trigger
    /// may try again.
    Refused,
}

impl TutorialRecordOutcome {
    /// The `decision_outcome` label for telemetry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::AlreadyShown => "already_shown",
            Self::Refused => "refused",
        }
    }
}

/// `BaseToCellMsg::TutorialRecorded`: the base's answer to one
/// [`RecordTutorialShown`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TutorialRecorded {
    pub entity_id: u32,
    /// The character the request named. The cell ignores the answer when
    /// `entity_id` now plays another character.
    pub player_id: i32,
    pub chain_id: i64,
    pub tutorial_id: i32,
    pub outcome: TutorialRecordOutcome,
}
