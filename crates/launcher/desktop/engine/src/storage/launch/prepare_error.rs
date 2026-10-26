//! Why Play preparation stopped before the host spawned anything (PP-06 ruling 1).
//! It carries the failure code `execute` records on the `NotStarted` launch.
//! Any error that has no code of its own is `launch_prepare_failed`, as before.
use super::*;
use crate::failure::FailureCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PrepareError(pub(super) FailureCode);

impl From<IntentError> for PrepareError {
    fn from(_: IntentError) -> Self {
        Self(FailureCode::LaunchPrepareFailed)
    }
}
impl From<StorageError> for PrepareError {
    fn from(_: StorageError) -> Self {
        Self(FailureCode::LaunchPrepareFailed)
    }
}
impl From<ContractError> for PrepareError {
    fn from(_: ContractError) -> Self {
        Self(FailureCode::LaunchPrepareFailed)
    }
}
