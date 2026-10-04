//! Platform-independent launcher operation ownership. No game mutations yet.
mod operations;
pub use operations::*;

mod storage;
pub use storage::{
    DesktopState, InstallAdmission, InstallIntent, IntentError, Preferences, StorageError,
};
mod commands;
pub use commands::{NativeCommand, NativeSnapshot};

// Share the existing Windows manifest schema and signature policy verbatim.
// The desktop catalog adds bounded transport; it never calls the legacy fetcher.
pub mod catalog;
#[path = "../../../src/manifest.rs"]
pub mod manifest;

// One implementation of the legacy installation algorithms on both shells.
#[path = "../../../src/client_setup/mod.rs"]
pub mod client_setup;
#[path = "../../../src/install.rs"]
pub mod install;
#[path = "../../../src/install_layout.rs"]
pub mod install_layout;
#[path = "../../../src/install_report.rs"]
pub mod install_report;
#[path = "../../../src/patch_dest.rs"]
pub mod patch_dest;
#[path = "../../../src/state.rs"]
pub mod state;
#[path = "../../../src/unpack/mod.rs"]
pub mod unpack;

#[path = "../../../src/install_progress.rs"]
pub mod install_progress;

pub mod archive_worker;

pub mod helper_supervisor;
