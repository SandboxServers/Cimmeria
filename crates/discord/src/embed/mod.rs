//! Build Discord embed JSON from an [`Event`](crate::event::Event).
//!
//! Pure function — no I/O, no globals. The sender pipes the JSON into a
//! `POST` against the webhook URL.
//!
//! # Discord embed limits
//!
//! Discord enforces the following per the API docs:
//!
//! - `title`: 256 chars
//! - `description`: 4096 chars
//! - `fields[].name`: 256 chars
//! - `fields[].value`: 1024 chars
//! - 25 fields max
//! - `footer.text`: 2048 chars
//! - sum of all character counts: 6000
//!
//! Inputs longer than these are truncated with the `…` ellipsis. A
//! truncation is intentionally visible — the alternative (silently
//! dropping the tail) hides bugs.
//!
//! The module is split along these seams:
//!
//! - [`builder`] — the public entry points ([`build_embed_body`],
//!   [`build_embed`]) that assemble the embed JSON.
//! - [`format`] — the per-variant `format_event` formatter, its
//!   string-building helpers, and `named`, the one `Name (#id)` renderer
//!   for typed events.
//! - [`format_gameplay`] — the gameplay and GM variants of the formatter.
//! - [`tracing_fields`] — Rule 6 ID/name folding and the trace footer
//!   for harvested `warn!`/`error!` events, driven by the pairing table
//!   in [`naming`].
//! - [`links`] — the no-internal-links guard every embed passes last.
//! - [`budget`] — truncation + the 6000-char total-budget enforcement.

mod budget;
mod builder;
mod format;
mod format_gameplay;
mod links;
mod naming;
#[cfg(test)]
mod pairing_tests;
mod tracing_fields;

pub use builder::{build_embed, build_embed_body};

// ── Discord embed limits (shared across submodules) ──────────────────────

pub(super) const MAX_TITLE: usize = 256;
pub(super) const MAX_DESC: usize = 4096;
pub(super) const MAX_FIELD_VALUE: usize = 1024;
pub(super) const MAX_FOOTER: usize = 2048;
pub(super) const MAX_FIELDS: usize = 25;
pub(super) const MAX_TOTAL: usize = 6000;
pub(super) const ELLIPSIS: &str = "…";
