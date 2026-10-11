//! The launcher logic both shells share: the egui `sgw-launcher` and the
//! desktop launcher's `cimmeria-launcher-engine` (LX-01 of the launcher
//! consolidation campaign, `docs/analysis/launcher-consolidation/`).
//!
//! Before this crate the engine compiled these files out of
//! `crates/launcher/src/` through `#[path]` includes. Each shell now
//! re-exports the modules under their old names, so `crate::manifest` in
//! the egui launcher and `cimmeria_launcher_engine::manifest` keep
//! resolving. Nothing here reaches back into either shell: where a module
//! needs something only a shell knows (the launcher's own directory, say),
//! it takes it as a parameter.

pub mod client_changes;
pub mod client_paths;
pub mod client_setup;
pub mod install;
pub mod install_layout;
pub mod install_progress;
pub mod install_report;
pub mod logs;
pub mod manifest;
pub mod overlay_meta;
pub mod patch_dest;
pub mod state;
pub mod telemetry;
pub mod unpack;

// The desktop engine has always named these two at its root.
pub use telemetry::endpoint as telemetry_endpoint;
pub use telemetry::session as telemetry_session;
