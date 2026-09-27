//! Message types for Base↔Cell inter-service communication.
//!
//! In the C++ architecture, BaseApp and CellApp communicated over Mercury UDP.
//! In our single-process Rust server, they use `tokio::mpsc` channels with these
//! typed message enums.
//!
//! Module layout:
//! - `data` — shared structs (`MailOp`, `NpcAoIData`, `NpcVitals`, `PlayerAoIData`, `SavedMission`).
//! - `base_to_cell` — `BaseToCellMsg` (BaseApp → CellApp messages).
//! - `cell_to_base` — `CellToBaseMsg` (CellApp → BaseApp messages).
//! - `lab` — the live-research-lab read-only query contract (`LabQuery`).
//! - `org_cell_to_base` / `org_base_to_cell` — the nested organization
//!   enums carried by `CellToBaseMsg::Org` and `BaseToCellMsg::Org`.
//! - `chat_cell_to_base` — the nested chat enum carried by
//!   `CellToBaseMsg::Chat`.

mod base_to_cell;
mod cell_to_base;
mod chat_cell_to_base;
mod data;
mod lab;
mod org_base_to_cell;
mod org_cell_to_base;

pub use crate::crafting::{
    CraftRequest, CraftVerb, CraftingStations, GmAllCraft, StationChangeCause, StationSet,
};
pub use base_to_cell::{BaseToCellMsg, LabConsoleResult};
pub use cell_to_base::CellToBaseMsg;
pub use chat_cell_to_base::ChatCellToBase;
pub use data::{MailOp, NpcAoIData, NpcVitals, PlayerAoIData, SavedMission};
pub use lab::{
    LabEntityFilter, LabEntitySnapshot, LabQuery, LabQueryReply, LabQueryResult, LabRadius,
    LabRadiusCenter, LabWitnessReport, LAB_ENTITY_QUERY_CAP,
};
pub use org_base_to_cell::OrgBaseToCell;
pub use org_cell_to_base::OrgCellToBase;

#[cfg(test)]
mod tests;
