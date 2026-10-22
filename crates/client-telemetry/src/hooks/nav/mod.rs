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
pub mod frame;
pub mod notices;
pub mod pose;
// The LN-08 button edges, the LN-09 transitions and the LN-10 UI ownership
// fill these in; LN-07 only sets up the frame they read.
mod buttons;
#[cfg(test)]
mod pose_tests;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) mod tick;
mod transitions;
mod ui;
