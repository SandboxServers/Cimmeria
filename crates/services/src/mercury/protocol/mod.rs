//! Session/phase builders: connect, time sync, char list, tick sync, reset,
//! logged off, char create failed, character visuals, resource fragment, version info.

// ── Submodules ───────────────────────────────────────────────────────────────

mod session;
mod character;
mod resources;

#[cfg(test)]
mod tests;

// ── Re-exports ───────────────────────────────────────────────────────────────
// Matches the public API that mercury/mod.rs imports from `protocol::*`.

pub use session::{
    build_connect_reply, build_time_sync, build_ongoing_tick_sync,
    build_reset_entities, build_logged_off,
};

pub use character::{
    build_char_list, build_on_character_list, build_char_create_failed,
    build_character_visuals,
};

pub use resources::{build_resource_fragment, build_version_info};

// ── Shared imports from parent (mercury) ─────────────────────────────────────
// Used by submodules via `super::`.

pub(crate) use super::{
    encrypt_packet, write_wstring, REPLY_FLAGS,
    BASEMSG_REPLY_MESSAGE, BASEMSG_UPDATE_FREQUENCY_NOTIFICATION,
    BASEMSG_TICK_SYNC, BASEMSG_SET_GAME_TIME, BASEMSG_CREATE_BASE_PLAYER,
    BASEMSG_ON_CHARACTER_LIST, BASEMSG_ON_CHARACTER_CREATE_FAILED,
    BASEMSG_ON_CHARACTER_VISUALS, BASEMSG_ON_VERSION_INFO,
    BASEMSG_RESET_ENTITIES, BASEMSG_RESOURCE_FRAGMENT, BASEMSG_LOGGED_OFF,
    ACCOUNT_CLASS_ID,
};
pub(crate) use super::types::CharacterInfo;
