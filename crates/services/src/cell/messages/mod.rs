//! Message types for Base↔Cell inter-service communication.
//!
//! In the C++ architecture, BaseApp and CellApp communicated over Mercury UDP.
//! In our single-process Rust server, they use `tokio::mpsc` channels with these
//! typed message enums.
//!
//! Module layout:
//! - `data` — shared structs (`MailOp`, `NpcAoIData`, `PlayerAoIData`, `SavedMission`).
//! - `base_to_cell` — `BaseToCellMsg` (BaseApp → CellApp messages).
//! - `cell_to_base` — `CellToBaseMsg` (CellApp → BaseApp messages).
//! - `lab` — the live-research-lab read-only query contract (`LabQuery`).

mod base_to_cell;
mod cell_to_base;
mod data;
mod lab;

pub use base_to_cell::{BaseToCellMsg, LabConsoleResult};
pub use cell_to_base::CellToBaseMsg;
pub use data::{MailOp, NpcAoIData, PlayerAoIData, SavedMission};
pub use lab::{
    LabEntityFilter, LabEntitySnapshot, LabQuery, LabQueryReply, LabQueryResult, LabRadius,
    LabRadiusCenter, LabWitnessReport, LAB_ENTITY_QUERY_CAP,
};

#[cfg(test)]
mod tests;
