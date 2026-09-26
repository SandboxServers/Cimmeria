//! Minigame server module.
//!
//! Implements a SmartFoxServer 1.x compatible TCP server that Flash SWFs
//! connect to for minigame gameplay (Livewire, Alignment, GoauldCrystals, etc.).

pub mod game;
pub mod games;
pub mod protocol;
pub mod server;
pub mod session;

pub use session::SessionRegistry;
