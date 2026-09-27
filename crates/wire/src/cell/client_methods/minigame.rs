//! MinigamePlayer interface ClientMethods (indices 52–64).

/// Launch a minigame SWF (URL contains IP/port/gameName/entityId/ticket).
pub const ON_START_MINIGAME: u16 = 52;
/// Show pre-game team/helper dialog.
pub const ON_START_MINIGAME_DIALOG: u16 = 53;
/// Close pre-game dialog.
pub const ON_START_MINIGAME_DIALOG_CLOSE: u16 = 54;
/// End the active minigame.
pub const ON_END_MINIGAME: u16 = 55;
/// List of players available to spectate.
pub const ON_SPECTATE_LIST: u16 = 56;
/// Prompt for minigame help registration cost.
pub const ON_MINIGAME_REGISTRATION_PROMPT: u16 = 57;
/// Current help registration state.
pub const MINIGAME_REGISTRATION_INFO: u16 = 58;
/// Add or update a helper in the helper list.
pub const ADD_OR_UPDATE_MINIGAME_HELPER: u16 = 59;
/// Remove a helper from the helper list.
pub const REMOVE_MINIGAME_HELPER: u16 = 60;
/// Display incoming call request from another player.
pub const MINIGAME_CALL_DISPLAY: u16 = 61;
/// Call request outcome (accepted/declined/timeout).
pub const MINIGAME_CALL_RESULT: u16 = 62;
/// Call was aborted.
pub const MINIGAME_CALL_ABORT: u16 = 63;
/// Show NPC minigame contact card.
pub const SHOW_MINIGAME_CONTACT: u16 = 64;
