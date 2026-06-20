//! Integration tests for `dispatch_sgw_player_base_method`, split by theme
//! (issue #529). Originally a single `dispatch/tests.rs`; every test body and
//! assertion is byte-identical to the original.
//!
//! - [`routing_logging`]: logoff send-failure surfacing, unhandled-method
//!   WARN, and the explicit DEBUG handlers (`perfStats`, `elementDataRequest`).
//! - [`chat_speaker_flags`]: `speaker_flags` GM/DND assembly, `CHAT_SET_DND`
//!   set/clear/malformed handling, per-character DND reset, and `CHAT_SET_AFK`.

mod chat_speaker_flags;
mod routing_logging;
