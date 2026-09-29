//! User chat channels (issue #1039): `chatJoin` creates or joins a named
//! channel, `chatLeave` leaves it, and a `sendPlayerCommunication` on a
//! user channel id (12 and up, [`CHAN_CHAT`](cimmeria_wire::cell::chat::CHAN_CHAT))
//! reaches every member.
//!
//! [`registry`] holds the channel table itself
//! ([`registry::UserChannelRegistry`], one process-wide instance via
//! [`registry::user_channel_registry`]); the base's `dispatch/chat.rs`
//! calls it directly rather than through a second layer here, the same way
//! `chat_gates.rs` calls [`super::mutes::mute_table`] directly.
//!
//! # Scope: global, not per-world or per-space
//!
//! A user channel is server-wide, like the legacy `ChatChannelManager`
//! singleton (`Chat.py`) -- gate travel changes a player's cell and space,
//! never their base `SGWPlayer` entity, and channel membership is keyed on
//! that entity id, so it survives gate travel untouched. Nothing here
//! reaches the cell: unlike squad chat (ORG-04), a user channel has no
//! spatial component to distribute, so every send goes straight from the
//! base to each member's own client.
//!
//! # Lifecycle
//!
//! - **Join**: `chatJoin(name, password)` creates the channel if no
//!   existing one's case-folded name matches, or joins the existing one.
//!   The password argument is decoded (so a malformed payload is still
//!   rejected) and otherwise ignored -- `chatPassword` (0xCC) remains "not
//!   available yet" and no channel is created with one, so there's nothing
//!   for a join password to check yet.
//! - **Leave**: `chatLeave(displayId)` removes the caller's entity; a
//!   channel emptied by the last member's leave is deleted immediately, its
//!   wire id freed for reuse -- there is no `CHANNEL_FLAG_KeepIfEmpty`
//!   here, because nothing pre-creates a channel before anyone has joined
//!   it.
//! - **Disconnect / logoff / return to character select**: the character's
//!   entity is torn down, and [`registry::UserChannelRegistry::leave_all`]
//!   is called at exactly the sites that already tear down that entity's
//!   other per-session state (crafting inductions;
//!   `crates/base/src/base/dispatch/session.rs::handle_log_off` and
//!   `crates/base-session/src/base/helpers/mod.rs::destroy_client_entities`),
//!   never on gate travel (the entity survives that).
//! - **Post**: `sendPlayerCommunication` on a channel id 12 and up looks the
//!   caller's entity up in that channel's member set before sending
//!   anything -- a client cannot post to a channel id it was never joined
//!   to, no matter what byte it sends (server authority).
//!
//! # No client-visible feedback beyond `onChatJoined` / `onChatLeft`
//!
//! The client's own `ChatMod.onChannelJoined` / `onChannelLeft`
//! (`ChatWindow.lua`) print "You have joined/left channel" themselves from
//! those two calls; a caller that also sent a `CHAN_FEEDBACK` line for a
//! successful join or leave would double the message. This holds for the
//! three login auto-joins exactly as it does for a manual `/chatjoin`: the
//! wire cannot tell the two apart (both are the same `chatJoin` call), so
//! there is nothing to spam and nothing extra to add. A **refused** join or
//! leave (bad name, already a member, not a member, at either limit) gets
//! exactly one feedback line, because there `onChatJoined`/`onChatLeft`
//! never fires at all.

pub mod registry;

pub use registry::{user_channel_registry, JoinOutcome, LeaveOutcome, UserChannelRegistry};
