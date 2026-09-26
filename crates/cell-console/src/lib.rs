//! # cimmeria-cell-console
//!
//! The cell's GM surfaces.
//!
//! - [`cell::console`]: the GM `.`-console command channel: the command
//!   registry, the parser and dispatcher, and the command families, among
//!   them the #523 authoring commands that emit seed SQL for a human to
//!   commit.
//! - [`cell::console::chat`]: the chat distribution, whose `CHAN_SAY` arm
//!   routes a GM's `.`-lines to the console.
//! - [`cell::console::gm`]: the native `gm*` cell methods (SGWGmPlayer, index
//!   109+).
//!
//! Split out of `cimmeria-services` (wave C5b of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting, so `crate::cell::…` and `super::…` paths inside it are unchanged,
//! and `cimmeria-services` re-exports `cell::console` at its old path and
//! `cell::chat` beside it.

#![warn(unreachable_pub)]

pub mod cell;

// Lower crates, at the crate-root path the moved code names them by.
pub(crate) use cimmeria_wire::mercury;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), so the
// moved tests keep importing `LogCapture` from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
