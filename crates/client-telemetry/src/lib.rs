//! Cimmeria client-side telemetry DLL.
//!
//! Side-loaded into `SGW.exe` by `sgw-launcher`'s injector
//! ([`crates/launcher/src/inject.rs`]) at game start. Once attached,
//! the DLL installs CME EventSignal subscribers, function hooks, and
//! log tees that observe gameplay without modifying it; events flow
//! through a lock-free MPMC ring to a separate uploader thread that
//! POSTs them to cimmeria-server's `/api/telemetry/upload-chunk`
//! endpoint, where they're replayed into `tracing` and shipped to
//! SigNoz under `service.name = cimmeria-client`.
//!
//! # Phase 1 scope (this file)
//!
//! The bootstrap-thread DllMain pattern only. No hooks, no upload,
//! no event queue. Phase 1's contract is: "the DLL loads cleanly,
//! its bootstrap thread runs, and SGW.exe does not crash." Follow-up
//! phases land tier-by-tier per
//! [`docs/architecture/client-telemetry.md`].
//!
//! # Why a bootstrap thread
//!
//! Windows holds the loader lock during `DllMain`. Doing anything
//! more than the bare minimum there — starting an HTTP client,
//! spawning real threads, even initialising `tracing` — invites
//! re-entrant `LoadLibrary` calls and deadlocks (see
//! [Microsoft's DLL best-practices doc]). The accepted pattern is:
//! `DllMain` spawns one bootstrap thread via `CreateThread` and
//! returns immediately. The bootstrap thread runs once loader lock
//! has cleared and does all the real work.
//!
//! [Microsoft's DLL best-practices doc]: https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices

#![cfg_attr(not(windows), allow(dead_code))]

// Phase 2 modules — all cross-platform. The Windows-specific bits
// (DllMain, GetModuleFileNameW for the host exe path, thread creation
// via CreateThread) stay in `boot`. The event queue, wire schema,
// session loader, and uploader thread compile and run on Linux for
// unit tests; only the Windows cdylib actually executes them inside
// SGW.exe.
pub mod events;
pub mod queue;
pub mod session;
pub mod uploader;

#[cfg(windows)]
mod boot;

#[cfg(windows)]
pub use boot::{attach_diagnostics, bootstrap_main, module_handle, producer, AttachDiagnostics};

// Non-Windows stub: the crate exists in the workspace so `cargo
// check --workspace` works on Linux, but its real surface is Windows
// only. The stub gives us a place to hang unit tests for any
// platform-agnostic helpers we factor out later.
#[cfg(not(windows))]
pub fn bootstrap_main() {
    unreachable!("cimmeria-client-telemetry has no non-Windows runtime surface");
}
