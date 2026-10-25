//! Linux launch preparation through Proton. PP-06 only stubs it: admission
//! already refuses a Linux Play without a matching Proton, and PP-07 starts the
//! game through Proton here.
use super::*;
use crate::game_telemetry::Session;
use crate::helper_supervisor::HelperCommand;
use crate::linux_proton::prefix::Prefix;
use crate::storage::display_options::{DisplayOptions, Placement};
use cimmeria_runtime_probe::game_launch::Request;

/// Prepares a Linux Play. Until PP-07 this always refuses, so a Play that was
/// admitted ends `NotStarted` before the host spawns anything.
pub(super) fn prepare(
    plan: &Plan,
    root: &Path,
    session: Option<&Session>,
    servers: &[LoginServer],
    display: DisplayOptions,
    placements: &[Placement],
) -> Result<(HelperCommand, Request, Prefix), IntentError> {
    let _ = (plan, root, session, servers, display, placements);
    Err(StorageError::Corrupt.into())
}
