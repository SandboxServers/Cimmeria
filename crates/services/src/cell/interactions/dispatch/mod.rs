//! Top-level interaction routing — `handle_interact` and `handle_initial_response`.
//!
//! Split along the two entry points:
//! - [`interact`]          — `handle_interact` (the `interact(targetEntityId)`
//!   cell method): validate, distance-check, pin target, dispatch by type.
//! - [`initial_response`]  — `handle_initial_response` (the `initialResponse`
//!   cell method): recover the pinned NPC and fire the matching dialog.

mod initial_response;
mod interact;

#[cfg(test)]
mod tests;

pub use initial_response::handle_initial_response;
pub use interact::handle_interact;

/// Maximum distance for NPC interaction (world units).
/// From `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
///
/// `pub(super)` so the sibling `loot` module can re-validate looting
/// distance against the same bound the initial `interact` enforces
/// (#446 — the loot handler must re-check range on every take, not just
/// trust the interact-time `looting_entity` pin).
pub(super) const MAX_INTERACT_DISTANCE: f32 = 5.0;
