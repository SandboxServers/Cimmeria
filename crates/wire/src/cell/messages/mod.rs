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
//! - `bank_cell_to_base` / `bank_base_to_cell` — the nested bank enums
//!   carried by `CellToBaseMsg::Bank` and `BaseToCellMsg::Bank`.
//! - `chat_cell_to_base` — the nested chat enum carried by
//!   `CellToBaseMsg::Chat`.
//! - `duel_base_to_cell` — the nested duel enum carried by
//!   `BaseToCellMsg::Duel`.
//! - `mail_gm_cell_to_base` — the GM mail tools (SS-U1), carried by
//!   `CellToBaseMsg::MailGm`.
//! - `content_mail_cell_to_base` — the content engine's `send_system_mail`
//!   action (SS-U3), carried by `CellToBaseMsg::ContentSystemMail`.
//! - `loot_grant` — `LootGrantSource` and `GrantRefusal`, the loot-grant
//!   round trip that returns a refused item to its corpse.
//! - `item_use` — `ConsumeItemForUse` and `ItemUseConsumed`, the native
//!   consumable round trip (the base consumes, then the cell applies).

mod bank_base_to_cell;
mod bank_cell_to_base;
mod base_to_cell;
mod black_market_cell_to_base;
mod cell_to_base;
mod chat_cell_to_base;
mod content_mail_cell_to_base;
mod data;
mod duel_base_to_cell;
mod item_use;
mod lab;
mod loot_grant;
mod mail_gm_cell_to_base;
mod org_base_to_cell;
mod org_cell_to_base;

pub use crate::crafting::{
    CraftRequest, CraftVerb, CraftingStations, GmAllCraft, GmCraftGrant, GmCraftGrantKind,
    RespecCraftOpen, StationChangeCause, StationSet,
};
pub use bank_base_to_cell::BankBaseToCell;
pub use bank_cell_to_base::{BankCellToBase, BankSubject, ExpandTrigger};
pub use base_to_cell::{BaseToCellMsg, LabConsoleResult};
pub use black_market_cell_to_base::{BlackMarketCellToBase, BmGmActor};
pub use cell_to_base::CellToBaseMsg;
pub use chat_cell_to_base::{ChatCellToBase, MAX_MUTE_MINUTES};
pub use content_mail_cell_to_base::{ContentMailCooldown, ContentSystemMail};
pub use data::{
    MailOp, MailSend, MailSendReject, NpcAoIData, NpcVitals, PlayerAoIData, SavedMission,
};
pub use duel_base_to_cell::DuelBaseToCell;
pub use item_use::{ConsumeItemForUse, ItemUseConsumed};
pub use lab::{
    LabEntityFilter, LabEntitySnapshot, LabQuery, LabQueryReply, LabQueryResult, LabRadius,
    LabRadiusCenter, LabWitnessReport, LAB_ENTITY_QUERY_CAP,
};
pub use loot_grant::{GrantRefusal, LootGrantSource};
pub use mail_gm_cell_to_base::{MailGmActor, MailGmCellToBase};
pub use org_base_to_cell::OrgBaseToCell;
pub use org_cell_to_base::OrgCellToBase;

#[cfg(test)]
mod tests;
