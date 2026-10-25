//! Linux launch preparation through Proton. PP-06 only stubs it: admission
//! already refuses a Linux Play without a matching Proton, and PP-07 starts the
//! game through Proton here.
use super::*;
use crate::failure::FailureCode;
use crate::game_telemetry::Session;
use crate::helper_supervisor::HelperCommand;
use crate::linux_proton::prefix::Prefix;
use crate::storage::display_options::{DisplayOptions, Placement};
use cimmeria_runtime_probe::game_launch::Request;

/// Prepares a Linux Play. Until PP-07 this always refuses with `proton_missing`,
/// so an admitted Linux Play ends `NotStarted` with that code.
pub(super) fn prepare(
    plan: &Plan,
    root: &Path,
    session: Option<&Session>,
    servers: &[LoginServer],
    display: DisplayOptions,
    placements: &[Placement],
) -> Result<(HelperCommand, Request, Prefix), PrepareError> {
    let _ = (plan, root, session, servers, display, placements);
    Err(PrepareError(FailureCode::ProtonMissing))
}
