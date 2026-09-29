//! Cimmeria client-side telemetry DLL.
//!
//! Side-loaded into `SGW.exe` by `sgw-launcher`'s injector
//! ([`crates/launcher/src/inject.rs`]) at game start. Once attached,
//! the DLL installs function hooks and log tees that observe gameplay
//! without modifying it; events flow
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
pub mod capture;
pub mod events;
pub mod fingerprint;
pub mod governor;
pub mod hooks;
pub mod log;
pub mod msvc_string;
pub mod queue;
pub mod session;
pub mod uploader;

// Live Research Lab client bridge — an inbound command channel behind
// the `lab-bridge` feature (off by default). The double activation
// gate (feature present AND a `lab` block in current-session.json) is
// documented in [`bridge`] and `docs/architecture/live-research-lab.md`.
#[cfg(feature = "lab-bridge")]
pub mod bridge;

/// Text a `lab-bridge` build carries in its image, and a player build
/// never does. The DLL writes [`BUILD_MARKER`] to its log at boot, so
/// the literal is in `.rdata` of whichever build it names.
///
/// Players get the telemetry DLL only when they opt in, and it must be
/// the build without the inbound TCP listener. Three checks read this
/// exact text from a DLL image and refuse a lab build: the launcher's
/// `build.rs` (it will not embed one), the launcher at launch time
/// (`crates/launcher/src/client_telemetry_dll.rs`, which also pins that
/// every copy of the text agrees), and the `verify` stage of
/// `tools/launcher-release/build.sh`. Change all of them together.
pub const LAB_BRIDGE_MARKER: &str = "cimmeria-client-telemetry build flavour: lab-bridge";

/// This build's marker: [`LAB_BRIDGE_MARKER`] under `lab-bridge`, else a
/// player marker that does not contain it.
#[cfg(feature = "lab-bridge")]
pub const BUILD_MARKER: &str = LAB_BRIDGE_MARKER;
#[cfg(not(feature = "lab-bridge"))]
pub const BUILD_MARKER: &str = "cimmeria-client-telemetry build flavour: player";

/// The short flavour name the DLL puts on `client.dll.attached`
/// (`dll_flavor`), so SigNoz shows which build a session ran.
#[cfg(feature = "lab-bridge")]
pub const BUILD_FLAVOR: &str = "lab-bridge";
#[cfg(not(feature = "lab-bridge"))]
pub const BUILD_FLAVOR: &str = "player";

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

#[cfg(test)]
mod build_marker_tests {
    use super::*;

    /// The packaging checks refuse any image containing
    /// `LAB_BRIDGE_MARKER`. A player build's marker must not contain it,
    /// or the launcher would refuse the DLL it is meant to ship.
    #[cfg(not(feature = "lab-bridge"))]
    #[test]
    fn a_player_build_does_not_carry_the_lab_bridge_marker() {
        assert!(!BUILD_MARKER.contains(LAB_BRIDGE_MARKER), "{BUILD_MARKER}");
        assert_eq!(BUILD_FLAVOR, "player");
    }

    /// The lab build must carry the marker, or the checks could never
    /// catch one.
    #[cfg(feature = "lab-bridge")]
    #[test]
    fn a_lab_bridge_build_carries_the_marker() {
        assert_eq!(BUILD_MARKER, LAB_BRIDGE_MARKER);
        assert_eq!(BUILD_FLAVOR, "lab-bridge");
    }

    /// Pinned: the launcher and `tools/launcher-release/build.sh` search
    /// DLL images for this exact text.
    #[test]
    fn the_lab_bridge_marker_text_is_pinned() {
        assert_eq!(
            LAB_BRIDGE_MARKER,
            "cimmeria-client-telemetry build flavour: lab-bridge"
        );
    }
}
