//! Reliable UDP channel state machine.
//!
//! Each remote peer gets a dedicated `Channel` that tracks the sliding windows
//! for transmit and receive, handles ACK processing, and manages retransmission
//! timers. The channel lifecycle mirrors the C++ `Mercury::Channel` class.
//!
//! # Module layout
//!
//! - [`state`] — lifecycle state ([`ChannelState`]) and per-packet TX/RX
//!   bookkeeping records ([`TxEntry`], [`RxEntry`]).
//! - [`channel_core`] — the [`Channel`] struct and its state-machine impl.
//! - [`tick`] — the multi-channel tick punch list ([`TickActions`]).
//! - [`rto`] — adaptive retransmission-timeout state.

pub mod rto;

mod channel_core;
mod state;
mod tick;

pub use channel_core::Channel;
pub use state::{ChannelState, RxEntry, TxEntry};
pub use tick::TickActions;

#[cfg(test)]
mod tests;
