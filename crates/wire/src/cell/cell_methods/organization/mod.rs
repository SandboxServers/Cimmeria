//! OrganizationMember interface exposed CellMethods (indices 8–19).
//!
//! The handlers are in `cimmeria_cell_methods::cell::cell_methods::organization`,
//! which re-exports these constants. [`decode_org_cell_method`] and
//! [`decode_on_organization_creation`] (SGWPlayer CM 94) are the typed
//! decoders both the cell and the base use.

mod decode;
mod reader;

pub use decode::{decode_on_organization_creation, decode_org_cell_method, OrgCellCall};
pub(crate) use reader::ArgReader;
pub use reader::OrgDecodeError;

pub const INVITE_RESPONSE: u16 = 8;
pub const LEAVE: u16 = 9;
pub const BROADCAST_MINIMAP_PING: u16 = 10;
pub const STRIKE_TEAM_RESPONSE: u16 = 11;
pub const PVP_LEAVE_RESPONSE: u16 = 12;
pub const MOTD: u16 = 13;
pub const NOTE: u16 = 14;
pub const OFFICER_NOTE: u16 = 15;
pub const SET_RANK_PERMISSIONS: u16 = 16;
pub const SET_RANK_NAME: u16 = 17;
pub const SQUAD_SET_LOOT_MODE: u16 = 18;
pub const TRANSFER_CASH: u16 = 19;

#[cfg(test)]
mod tests;
