//! The `onCharacterCreateFailed` (0x83) error codes.
//!
//! The client looks the code up in `error_texts` and shows that row's text,
//! so every code sent must be a real `ERROR_CharacterCreation*` /
//! `ERROR_InvalidCharacterName` row. Codes 1, 2 and 3, which this handler used
//! to send, are `CONDITION_FEEDBACK_*` rows: a rejected name showed
//! "CONDITION_FEEDBACK_PositionCheckNotBelow" (Class Start v6 CS-08 F1).
//! The values and which failure gets which are python's
//! (`deprecated/python/base/Account.py:createCharacter`,
//! `common/defs/CharacterCreation.py:getAllChoices`).

/// `ERROR_CharacterCreationNotEnoughInformation`: the payload is short or
/// malformed, or a required visual group has no choice.
pub(super) const NOT_ENOUGH_INFORMATION: i32 = 10000;
/// `ERROR_CharacterCreationInvalidCharacterType`: an unknown char_def, or a
/// start profile that cannot be used (lock L3).
pub(super) const INVALID_CHARACTER_TYPE: i32 = 10001;
/// `ERROR_CharacterCreationInvalidSkinColor`: a skin tint outside 0..=15.
pub(super) const INVALID_SKIN_COLOR: i32 = 10002;
/// `ERROR_CharacterCreationUnspecifiedError`: an invalid visual choice, no
/// database, or a database error.
pub(super) const UNSPECIFIED: i32 = 10003;
/// `ERROR_InvalidCharacterName`: the name or extra name breaks the format
/// rules, or the name is taken (python checked both in
/// `isCharacterNameAllowed` and answered both with this code).
pub(super) const INVALID_CHARACTER_NAME: i32 = 20001;
