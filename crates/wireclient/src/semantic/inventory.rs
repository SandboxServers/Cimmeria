//! The inventory family: `onActiveSlotUpdate` (70), `onRemoveItem` (71) and
//! `onUpdateItem` (72), all player-only.

use super::{S2CEvent, SemanticError};

pub(super) fn active_slot_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-08 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onActiveSlotUpdate",
    })
}

pub(super) fn remove_item(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-08 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onRemoveItem",
    })
}

pub(super) fn update_item(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-08 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onUpdateItem",
    })
}
