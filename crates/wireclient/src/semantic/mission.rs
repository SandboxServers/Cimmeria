//! The mission family: `onMissionUpdate` (80), `onStepUpdate` (81) and
//! `onObjectiveUpdate` (82), all player-only.

use super::{S2CEvent, SemanticError};

pub(super) fn mission_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-06 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onMissionUpdate",
    })
}

pub(super) fn step_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-06 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onStepUpdate",
    })
}

pub(super) fn objective_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-06 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onObjectiveUpdate",
    })
}
