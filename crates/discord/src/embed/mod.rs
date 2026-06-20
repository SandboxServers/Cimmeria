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
//! The module is split along three seams:
//!
//! - [`builder`] — the public entry points ([`build_embed_body`],
//!   [`build_embed`]) that assemble the embed JSON.
//! - [`format`] — the per-variant `format_event` formatter and its
//!   string-building helpers.
//! - [`budget`] — truncation + the 6000-char total-budget enforcement.

mod budget;
mod builder;
mod format;

pub use builder::{build_embed, build_embed_body};

// ── Discord embed limits (shared across submodules) ──────────────────────

pub(super) const MAX_TITLE: usize = 256;
pub(super) const MAX_DESC: usize = 4096;
pub(super) const MAX_FIELD_VALUE: usize = 1024;
// Footer is reserved for future use; cap kept here in lockstep with
// Discord's documented limit so the budget enforcement is uniform if
// we ever start using it.
#[allow(dead_code)]
pub(super) const MAX_FOOTER: usize = 2048;
pub(super) const MAX_FIELDS: usize = 25;
pub(super) const MAX_TOTAL: usize = 6000;
pub(super) const ELLIPSIS: &str = "…";
