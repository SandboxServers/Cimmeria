//! The always-injected `cimmeria-client-patches` DLL (Black Market plan
//! D2, packet BM-06).
//!
//! The DLL restores client features the 2009 `SGW.exe` shipped
//! unfinished; see `docs/architecture/client-patches.md`. The launcher
//! injects it on every `SGW.exe` launch unless the player opts out, and
//! independently of the telemetry opt-in.
//!
//! - [`dll_source`] finds the DLL: a configured override, the copy
//!   bundled into release launchers, or a file beside the launcher.
//! - [`plan`] decides whether to inject, and in what order when the
//!   telemetry DLL goes in too.

pub mod dll_source;
pub mod plan;

pub use plan::{decide, injection_order, InjectDecision, PatchInjection};
