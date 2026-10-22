//! The launcher logic of the desktop launcher's `cimmeria-launcher-engine`,
//! split out of the original egui launcher in LX-01 of the launcher
//! consolidation campaign (`docs/analysis/launcher-consolidation/`), which
//! shared it with the desktop launcher until LX-18 retired the egui one.
//!
//! The engine re-exports the modules under their old names, so
//! `cimmeria_launcher_engine::manifest` keeps resolving. Nothing here
//! reaches back into the shell: where a module needs something only the
//! shell knows (the launcher's own directory, say), it takes it as a
//! parameter.

pub mod client_changes;
pub mod client_paths;
pub mod client_setup;
pub mod file_index;
pub mod install;
mod install_codes;
pub mod install_layout;
pub mod install_progress;
pub mod install_report;
pub mod logs;
pub mod manifest;
pub mod manifest_chain;
pub mod overlay_meta;
pub mod overlay_pack;
pub mod patch_dest;
pub mod patch_summary;
pub mod seed_source;
pub mod stage_seed;
pub mod state;
pub mod telemetry;
pub mod unpack;

// The desktop engine has always named these two at its root.
pub use telemetry::endpoint as telemetry_endpoint;
pub use telemetry::session as telemetry_session;
