//! Item-grant subsystem.
//!
//! Split along natural seams (issue #529):
//! - [`validation`] — `normalize_item_ids` (pure) and `item_allows_container`
//!   (the container-placement gate).
//! - [`grant_item`] — `handle_grant_item` and `handle_loot_grant`, the grant
//!   entry points: placement, the vault guard, the transaction, the client
//!   sync, and the loot hand-back on a refusal.
//! - [`placement`] — which container the grant writes into (a storage
//!   request falls through to the first carried bag the item lists).
//! - [`persist`] — the grant transaction and its outcome.
//! - [`equip_epilogue`] — the bandolier and appearance work after an
//!   equipment grant commits.
//!
//! Re-exported here so every existing `grant::{normalize_item_ids,
//! item_allows_container, handle_grant_item}` import path stays valid.

mod equip_epilogue;
mod grant_item;
mod persist;
mod placement;
mod validation;

pub use grant_item::{handle_grant_item, handle_loot_grant};
pub use validation::{item_allows_container, normalize_item_ids};

// Types used by the test module below via `use super::*`. Gated to `cfg(test)`
// so they don't trip `unused_imports` in non-test builds (the slim re-export
// `mod.rs` itself references none of them).
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::net::SocketAddr;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
use cimmeria_mercury::transport::Transport;
#[cfg(test)]
use sqlx::PgPool;

#[cfg(test)]
use crate::base::ConnectedClientState;

#[cfg(test)]
mod bind_on_acquire_tests;
#[cfg(test)]
mod fall_through_tests;
#[cfg(test)]
mod full_bag_tests;
#[cfg(test)]
mod loot_refusal_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vault_guard_tests;
