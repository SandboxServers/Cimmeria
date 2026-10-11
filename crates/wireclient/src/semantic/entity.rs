//! The entity family: `onStaticMeshNameUpdate` (0), `onEntityProperty` (7),
//! `onVisible` (8) and `onBeingNameIDUpdate` (11), shared by both kinds, and
//! `onPlayerCommunication` (28), player-only.

use super::{S2CEvent, SemanticError};

pub(super) fn static_mesh_name_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-09 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onStaticMeshNameUpdate",
    })
}

pub(super) fn entity_property(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-09 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onEntityProperty",
    })
}

pub(super) fn visible(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-09 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onVisible",
    })
}

pub(super) fn being_name_id_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-09 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onBeingNameIDUpdate",
    })
}

pub(super) fn player_communication(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-09 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onPlayerCommunication",
    })
}
