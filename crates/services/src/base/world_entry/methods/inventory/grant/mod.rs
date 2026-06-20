//! Item-grant subsystem.
//!
//! Split along natural seams (issue #529):
//! - [`validation`] — `normalize_item_ids` (pure) and `item_allows_container`
//!   (the container-placement gate).
//! - [`grant_item`] — `handle_grant_item`, the full persist + client-sync
//!   grant flow.
//!
//! Re-exported here so every existing `grant::{normalize_item_ids,
//! item_allows_container, handle_grant_item}` import path stays valid.

mod grant_item;
mod validation;

pub use grant_item::handle_grant_item;
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
mod tests;
