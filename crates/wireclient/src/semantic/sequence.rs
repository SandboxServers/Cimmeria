//! The sequence family: `onSequence` (1, shared), `onKnownAbilitiesUpdate`
//! (101) and `onPlayMovie` (155), the last two player-only.

use super::{S2CEvent, SemanticError};

pub(super) fn sequence(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-07 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onSequence",
    })
}

pub(super) fn known_abilities_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-07 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onKnownAbilitiesUpdate",
    })
}

pub(super) fn play_movie(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    // WC-07 fills this in
    let _ = args;
    Err(SemanticError::NotImplemented {
        method: "onPlayMovie",
    })
}
