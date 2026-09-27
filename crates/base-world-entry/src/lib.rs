//! # cimmeria-base-world-entry
//!
//! The BaseApp's world entry, under `base::`:
//!
//! - `world_entry`: `playCharacter` teardown, `ENABLE_ENTITIES`, `mapLoaded`,
//!   gate travel, reanchor and teleport, and `cell_dispatch`, which turns
//!   every `CellToBaseMsg` from the cell into client packets (the AoI emitters
//!   and the deferred-AoI flush among them). The feature handlers it calls are
//!   `cimmeria-base-methods`, re-exported at `world_entry::methods`.
//! - `world_entry_appearance`: `onClientReady`, the post-cinematic appearance
//!   recovery and `cancelMovie`, and the release half of the first-login
//!   cinematic AoI hold.
//! - `character`: the character list, character visuals and delete.
//!
//! Split out of `cimmeria-services` (wave B3 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::base::…` and `super::…` paths inside it are unchanged.
//! The connect loop and `BaseService` in `cimmeria-base` call it through
//! `base::{world_entry, character}`. The cell is reached only through the
//! Base<->Cell messages in `cimmeria-wire`.

#![warn(unreachable_pub)]

pub mod base;

// The crate-level paths the moved code names, as it did in
// `cimmeria-services`: the services-side Mercury builders and firehoses, the
// decoded wire stream, the minigame host, credential redaction and, under
// `cell::`, the Base<->Cell messages, the client-method and `CLIENT_MG_*`
// indices and the player journal. Private imports, so the code keeps writing
// `crate::mercury::…`, `crate::cell::…` and so on.
use cimmeria_auth::credential_redaction;
use cimmeria_minigame::minigame;
use cimmeria_wire::{firehose, mercury};
use cimmeria_wire_log::wire_log;

mod cell {
    pub(crate) use cimmeria_wire::cell::{client_methods, dispatch, messages, player_journal};
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), next to
// the session crate's fixtures, so the moved tests keep importing them from
// `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_base_session::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
