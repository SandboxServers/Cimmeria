//! Message types for Base↔Cell inter-service communication.
//!
//! In the C++ architecture, BaseApp and CellApp communicated over Mercury UDP.
//! In our single-process Rust server, they use `tokio::mpsc` channels with these
//! typed message enums.
//!
//! Module layout:
//! - `data` — shared structs (`MailOp`, `NpcAoIData`, `SavedMission`).
//! - `base_to_cell` — `BaseToCellMsg` (BaseApp → CellApp messages).
//! - `cell_to_base` — `CellToBaseMsg` (CellApp → BaseApp messages).

mod base_to_cell;
mod cell_to_base;
mod data;

pub use base_to_cell::BaseToCellMsg;
pub use cell_to_base::CellToBaseMsg;
pub use data::{MailOp, NpcAoIData, SavedMission};

#[cfg(test)]
mod tests;
