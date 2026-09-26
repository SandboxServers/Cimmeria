//! BaseApp service -- Mercury UDP listener for persistent entity state
//! and client connections.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.
//!
//! Every module of the base track is in its own crate
//! (docs/architecture/services-crate-split.md): the per-connection session
//! state and the session-layer modules in `cimmeria-base-session` (wave B1),
//! the feature handlers in `cimmeria-base-methods` (B2), world entry and the
//! character list in `cimmeria-base-world-entry` (B3), and `BaseService`, the
//! connect loop, login, the base-method dispatch and the character creator in
//! `cimmeria-base` (B4). This module re-exports what the rest of this crate
//! and the downstream crates name at their old paths.

// Split out to `cimmeria-resources` (docs/architecture/services-crate-split.md,
// wave W1b) and re-exported at their old paths.
pub use cimmeria_resources::base::{
    chardef, dialog_overrides, item_overrides, mission_overrides, resources, sequence_overrides,
};

// Split out to `cimmeria-base-session` (wave B1) and re-exported at their old
// paths, with their old visibility. The session modules are not: their users
// here moved to `cimmeria-base-methods` (B2), `cimmeria-base-world-entry` (B3)
// and `cimmeria-base` (B4), and `contact_list`'s last one, the cell's
// contact-list methods, to `cimmeria-cell-methods` (C5a), which names the
// `wire` module in `cimmeria-wire`. `ConnectedClientState` and
// `PendingClientReadyInfo` are left only for the gate round trips; `gm_spawn`'s
// last user here, the template parity guard, moved to `cimmeria-base-session`
// in wave F.
pub use cimmeria_base_session::base::{BaseError, OnlinePlayer};
#[cfg(test)]
pub(crate) use cimmeria_base_session::base::{ConnectedClientState, PendingClientReadyInfo};

// Split out to `cimmeria-base-world-entry` (wave B3) and re-exported at its old
// path for the tests here that drive the cell and then the base's world entry
// (`gate_round_trip_tests`, `mission_round_trip_tests`). `character` is not:
// the connect loop and the character creator, its users here, moved to
// `cimmeria-base` (B4).
#[cfg(test)]
pub(crate) use cimmeria_base_world_entry::base::world_entry;

// Split out to `cimmeria-base` (wave B4) with the connect loop, login, the
// base-method dispatch and the character creator, and re-exported at its old
// path for the orchestrator and the downstream crates (§2H: the facade owns
// this re-export).
pub use cimmeria_base::base::BaseService;
