//! Internal session binding for the Frost letter turn-in.

use std::fmt;
use std::time::Instant;

use super::data::SavedMission;

/// Process-local world-entry incarnation, strictly increasing on each entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LetterTurnInEntryEpoch(u64);

impl LetterTurnInEntryEpoch {
    /// Construct an epoch assigned by the base world-entry counter.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the value for monotonic comparisons.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A truncated, domain-separated digest of the connected session key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LetterTurnInToken([u8; 16]);

impl LetterTurnInToken {
    /// Construct a token from an already-derived digest prefix.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the digest prefix for internal comparison and boundary encoding.
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for LetterTurnInToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LetterTurnInToken([REDACTED])")
    }
}

/// One server-created turn-in attempt. Retained across uncertain retries.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LetterTurnInAttempt([u8; 16]);

impl LetterTurnInAttempt {
    /// Construct an opaque server-created attempt identifier.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the identifier for the persistence boundary.
    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Debug for LetterTurnInAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LetterTurnInAttempt([REDACTED])")
    }
}

/// A cell-verified interaction request; no client supplied item or mission fields.
#[derive(Clone, Copy, Debug)]
pub struct LetterTurnInRequest {
    /// Cell entity that owns the accepted interaction.
    pub entity_id: u32,
    /// Durable player identity for the transaction.
    pub player_id: i32,
    /// Stable identifier reused for an uncertain retry.
    pub attempt: LetterTurnInAttempt,
    /// Opaque connected-session digest.
    pub token: LetterTurnInToken,
    /// Login incarnation timestamp.
    pub generation: Instant,
    /// Monotonic world-entry incarnation.
    pub epoch: LetterTurnInEntryEpoch,
}

/// Exact persisted postimage and inventory delta from A1.
#[derive(Clone, Debug)]
pub struct CommittedLetterTurnIn {
    /// Terminal mission row to install on the cell.
    pub mission: SavedMission,
    /// Consumed inventory instance identifier.
    pub item_id: i32,
    /// Container that held the consumed instance.
    pub container_id: i32,
    /// Stack size before the commit.
    pub stack_before: i32,
    /// Stack size after the commit.
    pub stack_after: i32,
}

/// Definite transaction refusal reasons safe to show as feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetterTurnInRefusal {
    /// Receipt attempt or session correlation was invalid.
    InvalidCorrelation,
    /// Player row did not exist.
    NoPlayer,
    /// Mission was already completed without this receipt.
    AlreadyCompleted,
    /// No accessible Frost letter remained in inventory.
    NoAccessibleLetter,
    /// Mission and step were not eligible.
    WrongMissionState,
    /// Concurrent state change prevented a commit.
    StateChanged,
}

/// Durable transaction result delivered to the matching cell entry.
#[derive(Clone, Debug)]
pub enum LetterTurnInOutcome {
    /// This attempt committed a new receipt.
    Committed(CommittedLetterTurnIn),
    /// This attempt already has a committed receipt.
    AlreadyCommitted(CommittedLetterTurnIn),
    /// No receipt was committed for a definite reason.
    Refused(LetterTurnInRefusal),
    /// The transaction or delivery may have committed; retry the same attempt.
    UnknownRetryable,
}

/// Correlated base-to-cell answer for one accepted request.
#[derive(Clone, Debug)]
pub struct LetterTurnInResult {
    /// Original request used to match cell pending state.
    pub request: LetterTurnInRequest,
    /// Transaction and client sync verdict.
    pub outcome: LetterTurnInOutcome,
}

/// Live player's token bound to one base-session generation.
#[derive(Clone, Copy, Debug)]
pub struct LetterTurnInSession {
    /// The owning player ID, checked against the current cell entity.
    pub player_id: i32,
    /// Redacted session-key digest.
    pub token: LetterTurnInToken,
    /// Phase-3 login instant; stable during travel.
    pub generation: Instant,
    /// This world-entry incarnation, including same-session gate travel.
    pub epoch: LetterTurnInEntryEpoch,
}
