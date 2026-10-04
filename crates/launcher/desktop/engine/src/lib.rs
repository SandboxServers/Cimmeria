//! Platform-independent launcher operation ownership. No game mutations yet.
mod operations;
pub use operations::*;

mod storage;
pub use storage::{DesktopState, Preferences, StorageError};
mod commands;
pub use commands::{NativeCommand, NativeSnapshot};

// Share the existing Windows manifest schema and signature policy verbatim.
// The desktop catalog adds bounded transport; it never calls the legacy fetcher.
pub mod catalog;
#[path = "../../../src/manifest.rs"]
pub mod manifest;
