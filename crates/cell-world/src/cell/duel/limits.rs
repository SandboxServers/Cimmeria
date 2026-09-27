//! Every duel timer and distance, in one place.
//!
//! All of these are **project policy, not recovered data** (ledger D-SS18,
//! D-SS19, D-SS21). SS-E1 found no client constant for the countdown: the
//! client shows whatever duration the server sends (D-Q1). No challenge or
//! arena range was recovered (D-Q2).

use std::time::Duration;

/// A challenge with no answer expires after this (D-SS18).
pub const CHALLENGE_TIMEOUT: Duration = Duration::from_secs(30);

/// From the accept to the engaged duel (D-SS18).
pub const COUNTDOWN: Duration = Duration::from_secs(5);

/// After a decline or an expiry, the same challenger may not challenge the
/// same target again for this long (D-SS21).
pub const PAIR_COOLDOWN: Duration = Duration::from_secs(60);

/// The target must be within this many world units of the challenger, in
/// the same space (D-SS19, text 877).
pub const CHALLENGE_RANGE: f32 = 20.0;
