//! Channel tests — split from a single tests.rs to keep each file
//! under the 700-line cap. Submodules are grouped by subsystem.

use super::*;

// The test submodules reference `Channel`/`ChannelState` (re-exported by the
// parent via `use super::*`) plus `consts` and `Packet` bare. The latter two
// were module-level imports on the pre-split `channel/mod.rs`; bring them in
// here so `use super::*` in the child test files keeps resolving them.
pub(super) use crate::consts;
pub(super) use crate::packet::Packet;

mod channel_lifecycle;
mod reassembly;
