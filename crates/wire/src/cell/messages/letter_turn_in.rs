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
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

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
    pub entity_id: u32,
    pub player_id: i32,
    pub attempt: LetterTurnInAttempt,
    pub token: LetterTurnInToken,
    pub generation: Instant,
    pub epoch: LetterTurnInEntryEpoch,
}

#[derive(Clone, Debug)]
pub struct CommittedLetterTurnIn {
    pub mission: SavedMission,
    pub item_id: i32,
    pub container_id: i32,
    pub stack_before: i32,
    pub stack_after: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetterTurnInRefusal {
    InvalidCorrelation,
    NoPlayer,
    AlreadyCompleted,
    NoAccessibleLetter,
    WrongMissionState,
    StateChanged,
}

#[derive(Clone, Debug)]
pub enum LetterTurnInOutcome {
    Committed(CommittedLetterTurnIn),
    AlreadyCommitted(CommittedLetterTurnIn),
    Refused(LetterTurnInRefusal),
    /// The transaction or delivery may have committed; retry the same attempt.
    UnknownRetryable,
}

#[derive(Clone, Debug)]
pub struct LetterTurnInResult {
    pub request: LetterTurnInRequest,
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
