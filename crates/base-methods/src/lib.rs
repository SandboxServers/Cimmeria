//! # cimmeria-base-methods
//!
//! The BaseApp's DB-backed feature handlers, `base::world_entry::methods`:
//!
//! - player load (`player_load`, `world_entry_db`): the character, inventory
//!   and stargate queries world entry and gate travel build their payloads
//!   from;
//! - inventory (`inventory`): grant, move, remove, use, ammo and the
//!   appearance refresh an equip change triggers;
//! - vendors (`vendor`): the store window, purchase, sell, buyback, repair
//!   and recharge;
//! - player trade (`trade`): the atomic swap of two players' offers;
//! - mail, missions and progression (`mail`, `missions`, `progression`):
//!   mail forwarding, `sgw_mission` persistence, cash and XP grants, level-ups
//!   and ability training.
//!
//! Split out of `cimmeria-services` (wave B2 of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::base::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports `methods` at its old path
//! (`base::world_entry::methods`), where the cell dispatch, world entry and
//! gate travel call it.

#![warn(unreachable_pub)]

pub mod base;

// The crate-level paths the moved code names, as it did in
// `cimmeria-services`: the services-side Mercury builders, the ability-tree
// payloads and, under `cell::`, the Base<->Cell messages, the mail serializers
// and the client-method indices. Private imports, so the code keeps writing
// `crate::mercury::…`, `crate::ability_tree::…` and `crate::cell::…`.
use cimmeria_wire::{ability_tree, mercury};

mod cell {
    pub(crate) use cimmeria_wire::cell::{client_methods, mail, messages};
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), next to
// the session crate's fixtures, so the moved tests keep importing them from
// `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_base_session::test_fixtures::*;
    pub(crate) use cimmeria_test_support::*;
}
