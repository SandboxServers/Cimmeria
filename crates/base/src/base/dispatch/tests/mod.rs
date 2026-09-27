//! Integration tests for `dispatch_sgw_player_base_method`, split by theme
//! (issue #529). Originally a single `dispatch/tests.rs`; every test body and
//! assertion is byte-identical to the original.
//!
//! - [`routing_logging`]: logoff send-failure surfacing, unhandled-method
//!   WARN, and the explicit DEBUG handlers (`perfStats`, `elementDataRequest`).
//! - [`chat_speaker_flags`]: `speaker_flags` GM/DND assembly, `CHAT_SET_DND`
//!   set/clear/malformed handling, per-character DND reset, and `CHAT_SET_AFK`.
//! - [`organization`]: the 0xCF-0xD2 arm answers each call with
//!   `onErrorCode` and a feedback line, and drops a malformed payload.

mod chat_dnd_limit;
mod chat_speaker_flags;
mod organization;
mod routing_logging;
