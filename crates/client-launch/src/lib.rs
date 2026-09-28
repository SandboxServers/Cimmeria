//! Shared client launch primitives.
//!
//! These modules were extracted verbatim from `sgw-launcher`
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
//!
//! The `.rdata` "hostname patch" that used to live here is gone: the only
//! ASCII `www.stargateworlds.com` in SGW.exe is the SOAP namespace
//! `http://www.stargateworlds.com/xml/sgwlogin`, so it redirected nothing.
//! The client's login servers come from `LoginInternal.lua`, which the
//! launcher writes (`sgw-launcher`'s `client_setup`).
//!
//! The launcher re-exports these at its own crate root (`use
//! cimmeria_client_launch::launch;`) so every existing `crate::launch::…`
//! reference inside the launcher keeps resolving unchanged.

pub mod inject;
pub mod launch;
pub mod process;
pub mod start32;
