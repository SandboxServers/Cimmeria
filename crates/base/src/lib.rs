//! # cimmeria-base
//!
//! The BaseApp service, under `base::`:
//!
//! - `BaseService`: the Mercury UDP listener's lifecycle, the admin API's
//!   online-player snapshot and the `chaos-testing` transport override.
//! - `connect_loop`: the receive loop, the encrypted-bundle scanner and the
//!   Account and cell-method arms that route each message to its handler.
//! - `login`: the Phase 3 handshake (`BASEMSG_REPLY`, time sync, the tick
//!   loop) and log-off.
//! - `dispatch`: the SGWPlayer base methods (chat, AFK and DND, log-off,
//!   perf stats).
//! - `character_create`: `createCharacter`, from the CharDef lookup to the
//!   starter inventory.
//!
//! Split out of `cimmeria-services` (wave B4 of
//! `docs/architecture/services-crate-split.md`), the top of the base track.
//! The module tree keeps its old nesting, so `crate::base::…` and `super::…`
//! paths inside it are unchanged, and `cimmeria-services` re-exports
//! `base::BaseService` at its old path, where the orchestrator builds it. The
//! cell is reached only through the Base<->Cell messages in `cimmeria-wire`.
//! The crate owns the `chaos-testing` feature; `cimmeria-services` forwards
//! its own to it.

#![warn(unreachable_pub)]

pub mod base;

// The crate-level paths the moved code names, as it did in
// `cimmeria-services`: the services-side Mercury builders and firehoses, the
// decoded wire stream, the minigame host, the auth handoff, credential
// redaction and, under `cell::`, the Base<->Cell messages. Private imports, so
// the code keeps writing `crate::mercury::…`, `crate::cell::…` and so on.
use cimmeria_auth::{auth, credential_redaction};
use cimmeria_minigame::minigame;
use cimmeria_wire::{firehose, mercury};
use cimmeria_wire_log::wire_log;

mod cell {
    pub(crate) use cimmeria_wire::cell::messages;
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), next to
// the session crate's fixtures, so the moved tests keep importing them from
// `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_base_session::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
