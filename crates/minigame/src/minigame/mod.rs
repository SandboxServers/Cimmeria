//! Minigame server module.
//!
//! Implements a SmartFoxServer 1.x compatible TCP server that Flash SWFs
//! connect to for minigame gameplay (Livewire, Alignment, GoauldCrystals, etc.).

pub mod game;
pub mod games;
pub mod protocol;
mod protocol_list;
pub mod repl;
pub mod server;
pub mod session;
mod session_claim;
mod session_offline;
mod session_snapshot;

pub use session::SessionRegistry;
