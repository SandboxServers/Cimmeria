//! The player camera and the pawn pose, read in process.
//!
//! What is read: the local `ASGWCamera_Player` block (invert flags, gain,
//! zoom, pitch and yaw), the camera actor's pose, and, under S-CAM A, the
//! local pawn's pose. The wire yaw and pitch are the pawn rotation, so the
//! pawn pose is read as well. The readers run when the emitter asks (PC-16,
//! on interact and useItem). They ride the existing diagnostics opt-in
//! (D-PC3 and F13), and add no consent of their own.

pub mod emit;
#[cfg(test)]
mod emit_tests;
pub mod layout;
pub mod read;
#[cfg(test)]
mod tests;
