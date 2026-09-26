//! The `base::` skeleton around world entry.
//!
//! `world_entry`, `world_entry_appearance` and `character` are the code this
//! crate holds. The other names here are the session-layer modules and types
//! that code reaches through `crate::base::…` and `super::super::…` paths,
//! re-exported privately from `cimmeria-base-session`, so those paths compile
//! unchanged. The service, the connect loop and the base-method dispatch stay
//! in `cimmeria-services` (wave B4 moves them to `cimmeria-base`).

pub mod character;
pub mod world_entry;
// Crate-private, as it was in `cimmeria-services`: `world_entry` re-exports
// the two handlers the connect loop calls.
pub(crate) mod world_entry_appearance;

pub(crate) use cimmeria_base_session::base::{
    cinematic_aoi_hold, console_authoring, contact_list, crafting, deferred_aoi,
    deferred_aoi_lifecycle, gm_spawn, helpers, session_identity, world_entry_chat,
    ConnectedClientState, PendingClientReadyInfo,
};
