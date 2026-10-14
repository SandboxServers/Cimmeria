//! Internal session binding for the Frost letter turn-in.

use std::fmt;
use std::time::Instant;

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
