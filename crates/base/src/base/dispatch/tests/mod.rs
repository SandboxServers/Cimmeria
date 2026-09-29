//! Integration tests for `dispatch_sgw_player_base_method`, split by theme
//! (issue #529). Originally a single `dispatch/tests.rs`; every test body and
//! assertion is byte-identical to the original.
//!
//! - [`routing_logging`]: logoff send-failure surfacing, unhandled-method
//!   WARN, and the explicit DEBUG handlers (`perfStats`, `elementDataRequest`).
//! - [`chat_speaker_flags`]: `speaker_flags` GM/DND assembly, `CHAT_SET_DND`
//!   set/clear/malformed handling, per-character DND reset, and `CHAT_SET_AFK`.
//! - [`crafting_teardown`]: `logOff` drops the queued crafting inductions.
//! - [`organization`]: the 0xCF-0xD2 arm answers each call with
//!   `onErrorCode` and a feedback line, and drops a malformed payload.
//! - [`chat_flood_limit`]: the chat bucket and length cap before the cell
//!   forward (SS-00).
//! - [`player_index_logoff`]: `logOff` unlists the character (SS-00).
//! - [`duel_challenge`]: `sendDuelChallenge` (0xD9): the duel bucket, squad
//!   refusal, the online lookup and the forward (SS-D1).
//! - [`tell`]: tells on the base, fan-out and refusals (SS-C1).
//! - [`chat_ignore`]: `chatIgnore` refusals and list edits (SS-C1).
//! - [`chat_ignore_race`]: the cap and duplicate checks under the list lock.
//! - [`chat_channel_mute`]: the channel allowlist and the GM mute (SS-C3).
//! - [`communicator_unsupported`]: the 0xC6-0xCE feedback arms (SS-C3).
//! - [`chat_user_channels`]: `chatJoin`/`chatLeave`/user-channel posts and
//!   their disconnect cleanup (issue #1039).

mod chat_channel_mute;
mod chat_dnd_limit;
mod chat_flood_limit;
mod chat_ignore;
mod chat_ignore_race;
mod chat_org_refusals;
mod chat_speaker_flags;
mod chat_squad_refusals;
mod chat_user_channels;
mod communicator_unsupported;
mod crafting_teardown;
mod duel_challenge;
mod org_invite_logoff;
mod org_logoff_presence;
mod organization;
mod player_index_logoff;
mod routing_logging;
mod tell;
