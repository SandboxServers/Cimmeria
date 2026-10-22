//! The player controller's input state, read in process.
//!
//! What is read: the local `ASGWController_Player` mode bits, button bits,
//! drag counters and hover entity id, and the `PlayerInput` axes. Every
//! input row is a poll of these fields (D-LN5); nothing is hooked. Reads go
//! through [`Mem`](crate::hooks::entity_trace::map::Mem), so a stale pointer is
//! `None`, never a panic.

pub mod controller;
#[cfg(test)]
mod controller_tests;
