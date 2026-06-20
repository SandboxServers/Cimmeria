//! Typed event taxonomy. Every server-side notification is one of these
//! variants — the type system makes it impossible to construct an event
//! with the wrong payload shape, and `EventKind` provides a discriminant
//! that lines up with the per-event toggles in [`crate::config`].
//!
//! Events route to one of 8 channels (`ChannelKind`). The mapping lives in
//! [`crate::router`] and is exhaustively tested.
//!
//! The module is split along three seams:
//!
//! - [`channel_kind`] — the logical destination [`ChannelKind`].
//! - [`event_kind`] — the [`EventKind`] discriminant (toggle/router key).
//! - [`payload`] — the rich [`Event`] enum plus its supporting enums
//!   ([`DisconnectReason`], [`ChatKind`], [`TracingEventKind`],
//!   [`Severity`]) and the discriminant/severity derivations.

mod channel_kind;
mod event_kind;
mod payload;

pub use channel_kind::ChannelKind;
pub use event_kind::EventKind;
pub use payload::{ChatKind, DisconnectReason, Event, Severity, TracingEventKind};
