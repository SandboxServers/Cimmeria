//! # cimmeria-client-patches
//!
//! An injected DLL that restores client features the 2009 `SGW.exe` shipped
//! unfinished. It is separate from `cimmeria-client-telemetry` because
//! gameplay must not depend on the telemetry opt-in; see
//! `docs/architecture/client-patches.md`.
//!
//! The first feature is the Black Market's **receive** side. The client
//! resolves the six `onBM*` client methods (90–95) to a
//! `MethodDescription` and then drops them, because nothing was bound to
//! them. This DLL:
//!
//! 1. **Checks the build** ([`fingerprint`]): the prologue bytes at every
//!    address it uses must match this `SGW.exe`, or it installs nothing.
//! 2. **Receives** on the network thread ([`receive`]): a detour on the
//!    entity-method dispatcher remembers `(entity, stream)`, and a detour on
//!    the drop callee claims the call when the method name is one of the six
//!    and the entity is the local player. It decodes the arguments with
//!    `cimmeria-patch-wire` straight from the client's `BinaryIStream` and
//!    queues the result. Nothing on that thread touches Lua or game state.
//! 3. **Delivers** on the main thread ([`deliver`]): a detour on
//!    `FEngineLoop::Tick` drains the queue into the UI Lua through the
//!    `lua51.dll` C API, calling `CimmeriaBM.onOpen(...)` and its siblings
//!    under `lua_pcall`.
//!
//! Sending (the cell methods 61–66) is not here yet.
//!
//! Every address is in [`addresses`], with its evidence. Everything that is
//! not raw FFI (name matching, the local-player check, the queue, the Lua
//! call plan, the prologue check) is portable and unit-tested on the host;
//! the hooks themselves only compile for `i686-pc-windows-msvc`.

// Off the real target the FFI callers are compiled out, which leaves the
// portable layers without their production callers.
#![cfg_attr(not(all(windows, target_arch = "x86")), allow(dead_code))]

pub mod addresses;
pub mod counters;
pub mod deliver;
pub mod fingerprint;
pub mod log;
pub mod memory;
pub mod queue;
pub mod receive;

#[cfg(all(windows, target_arch = "x86"))]
mod boot;
#[cfg(all(windows, target_arch = "x86"))]
mod hooks;

use cimmeria_patch_wire::black_market::ClientCall;

/// Most decoded calls waiting for the main thread. A burst beyond this is
/// dropped and counted: the UI can re-request, and an unbounded queue
/// behind a UI that never comes up would grow for the whole session.
pub const QUEUE_CAPACITY: usize = 256;

/// Calls decoded on the network thread, waiting for the main thread.
pub(crate) static EVENTS: queue::EventQueue<ClientCall> = queue::EventQueue::new(QUEUE_CAPACITY);

/// What the patch has done this session.
pub(crate) static COUNTERS: counters::Counters = counters::Counters::new();
