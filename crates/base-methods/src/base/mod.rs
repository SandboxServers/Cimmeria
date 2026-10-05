//! The `base::` skeleton around the feature handlers.
//!
//! `world_entry::methods` is the only code this crate holds. The other names
//! here are the session-layer modules the handlers reach through
//! `crate::base::…` and `super::super::…` paths, re-exported privately from
//! `cimmeria-base-session` and `cimmeria-resources`, so those paths compile
//! unchanged. World entry proper stays in `cimmeria-services`, which
//! re-exports `methods` at its old path.

pub(crate) use cimmeria_base_session::base::{
    contact_list, feedback, gm_feedback, helpers, inventory_locks, outbox, player_index,
    rate_limit, session_identity, ConnectedClientState,
};
// The bag tables.
pub(crate) use cimmeria_resources::base::resources;

pub mod world_entry {
    //! World entry's feature handlers.

    pub mod methods;

    // `world_entry_db` resolves world names through the space registry.
    pub(crate) use cimmeria_base_session::base::world_entry::space_registry;
    // ...and refuses a non-GM a GM-only world (D-DA4).
    pub(crate) use cimmeria_base_session::base::world_entry::gm_only_worlds;
}

/// The appearance builder the inventory's equip refresh resends.
mod world_entry_appearance {
    pub(crate) use cimmeria_base_session::base::world_entry_appearance::builders::build_appearance_args;
}
