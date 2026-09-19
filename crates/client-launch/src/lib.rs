//! Shared client launch primitives.
//!
//! These three modules were extracted verbatim from `sgw-launcher`
//! (issue #685, ADR §3.4) so both the launcher and the Live Research
//! Lab supervisor (`cimmeria-lab`) drive the *same* suspended-launch +
//! inject code path rather than maintaining two copies:
//!
//! - [`launch`] — spawn SGW.exe (optionally with the telemetry DLL
//!   injected before any user thread runs), plus Atera-debug launch
//!   helpers and install-dir probes.
//! - [`inject`] — the `CreateProcessW(SUSPENDED)` → `VirtualAllocEx`
//!   → `WriteProcessMemory` → `CreateRemoteThread(LoadLibraryW)` →
//!   `ResumeThread` DLL-injection pipeline and its RAII
//!   [`inject::SuspendedProcess`] handle.
//! - [`patch_rdata`] — patch the hardcoded server hostname in
//!   SGW.exe's `.rdata` so the client connects to a chosen shard.
//!
//! This is a pure refactor: the launcher re-exports these at its own
//! crate root (`use cimmeria_client_launch::{inject, launch,
//! patch_rdata};`) so every existing `crate::launch::…` /
//! `crate::inject::…` reference inside the launcher keeps resolving
//! unchanged.

pub mod inject;
pub mod launch;
pub mod patch_rdata;
