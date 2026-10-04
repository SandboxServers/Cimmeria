//! Platform-independent launcher operation ownership. No game mutations yet.
mod operations;
pub use operations::*;

mod storage;
pub use storage::{DesktopState, Preferences, StorageError};
mod commands;
pub use commands::{NativeCommand, NativeSnapshot};
