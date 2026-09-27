//! # cimmeria-minigame
//!
//! The SmartFoxServer 1.x host the original Stargate Worlds Flash minigame
//! SWFs connect to (Livewire, Hack, Bypass, ...), run in-process beside the
//! Base and Cell services:
//!
//! - [`minigame::protocol`]: the SFS XML packet codec, and the `sfs_vars!`
//!   macro the games build their variable maps with.
//! - [`minigame::session`]: [`minigame::SessionRegistry`], the one-time ticket
//!   registry the base fills when a chain starts a minigame, with its TTL
//!   sweep for sessions nobody connects to.
//! - [`minigame::server`]: the TCP listener, the version/login handshake and
//!   the per-connection game loop. Every outcome leaves through one seam, as a
//!   `CellToBaseMsg::MinigameResult` on the wire contract's channel.
//! - [`minigame::games`]: the game implementations (Livewire, and a
//!   placeholder for the games not yet ported).
//!
//! Split out of `cimmeria-services` (wave W3c of
//! `docs/architecture/services-crate-split.md`). The module keeps its old
//! path, so `crate::minigame::…` and `super::…` paths inside it are
//! unchanged, and `cimmeria-services` re-exports it at `minigame`.

#![warn(unreachable_pub)]

pub mod minigame;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), so the
// moved tests keep importing them from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
