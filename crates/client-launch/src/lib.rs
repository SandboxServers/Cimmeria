//! Shared client launch primitives.
//!
//! These modules were extracted verbatim from the original egui launcher
//! (issue #685, ADR §3.4) so the launcher and the Live Research Lab
//! supervisor (`cimmeria-lab`) drive the *same* suspended-launch + inject
//! code path rather than maintaining two copies (today the desktop
//! launcher's 32-bit launch worker and the `sgw-start32` helper are its
//! launcher-side users):
//!
//! - [`launch`] — spawn SGW.exe (optionally with the telemetry DLL
//!   injected before any user thread runs), plus Atera-debug launch
//!   helpers and install-dir probes.
//! - [`inject`] — the `CreateProcessW(SUSPENDED)` → `VirtualAllocEx`
//!   → `WriteProcessMemory` → `CreateRemoteThread(LoadLibraryW)` →
//!   `ResumeThread` DLL-injection pipeline and its RAII
//!   [`inject::SuspendedProcess`] handle.
//!
//! The `.rdata` "hostname patch" that used to live here is gone: the only
//! ASCII `www.stargateworlds.com` in SGW.exe is the SOAP namespace
//! `http://www.stargateworlds.com/xml/sgwlogin`, so it redirected nothing.
//! The client's login servers come from `LoginInternal.lua`, which the
//! launcher writes (`cimmeria-launcher-core`'s `client_setup`).

pub mod inject;
pub mod launch;
pub mod process;
pub mod start32;
