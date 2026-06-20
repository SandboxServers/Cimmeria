//! BeingAppearance assembly, onEntityTint, and visual resend helpers.
//!
//! Extracted from `world_entry.rs` — these functions build the appearance
//! wire data and handle the post-transaction / post-cinematic resend logic.
//!
//! Split along three natural seams (issue #529):
//! - [`builders`] — pure wire-arg builders (`build_appearance_args`,
//!   `build_tint_args`) and the bundle composers, plus their byte-exact
//!   regression guards.
//! - [`client_ready`] — the `SGWPlayer.onClientReady` world-entry
//!   finalization handler.
//! - [`cinematic`] — `onPlayMovie` dispatch + the post-cinematic
//!   appearance-recovery spam guard and the `cancelMovie` handler.
//!
//! Re-exported here so every existing `crate::base::world_entry_appearance::*`
//! import path stays valid.
//!
//! Chat-channel registration helpers (`DEFAULT_CHAT_CHANNELS`, `CHAN_TELL`,
//! `build_chat_joined_args`, `build_welcome_message_args`) and their
//! byte-exact tests live in `super::world_entry_chat`. They're used inside
//! `client_ready::handle_on_client_ready` for the post-`onClientReady`
//! `ChannelManager.playerLoggedIn` flow.

mod builders;
mod cinematic;
mod client_ready;

pub(crate) use builders::{build_appearance_args, build_tint_args};
pub(crate) use cinematic::{handle_cancel_movie, send_cinematic};
pub(crate) use client_ready::handle_on_client_ready;
