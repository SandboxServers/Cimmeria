//! What an `Action::MovementLock` does (cellblock-boot-lock D-BL3).
//!
//! A `hold` pins the acting player to a fixed seeded anchor; a `release`
//! removes that hold. The loader is `loader/action_movement_lock.rs`.
use serde::{Deserialize, Serialize};

/// `max_distance` when the row omits it, in world units (horizontal).
pub const DEFAULT_MAX_DISTANCE: f32 = 3.0;
/// The largest `max_distance` the loader accepts.
pub const MAX_MAX_DISTANCE: f32 = 10.0;
/// The longest `message` the loader accepts, in chars.
pub const MAX_MESSAGE_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MovementLockOp {
    /// Pin the acting player to `at` while the prop tagged `anchor_tag` is live
    /// in their space and they stand within `max_distance` of `at`.
    Hold {
        anchor_tag: String,
        at: [f32; 3],
        max_distance: f32,
        /// Also set the ref-counted `BSF_MOVEMENT_LOCK` state flag (Layer A).
        client_flag: bool,
        message: Option<String>,
    },
    /// Remove the hold (if any). `message` is said either way.
    Release { message: Option<String> },
}

impl MovementLockOp {
    /// The `op` param value, for logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hold { .. } => "hold",
            Self::Release { .. } => "release",
        }
    }
}
