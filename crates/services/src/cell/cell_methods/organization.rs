//! OrganizationMember interface exposed CellMethods (indices 8–19).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

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

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        INVITE_RESPONSE => {
            if args.len() >= 5 {
                let request_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(
                    entity_id,
                    request_id,
                    response,
                    "UNIMPLEMENTED: organizationInviteResponse"
                );
            }
            true
        }
        LEAVE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationLeave");
            }
            true
        }
        BROADCAST_MINIMAP_PING => {
            if args.len() >= 16 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    x,
                    y,
                    z,
                    "UNIMPLEMENTED: BroadcastMinimapPing"
                );
            }
            true
        }
        STRIKE_TEAM_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(
                    entity_id,
                    org_id,
                    response,
                    "UNIMPLEMENTED: strikeTeamResponse"
                );
            }
            true
        }
        PVP_LEAVE_RESPONSE => {
            if args.len() >= 5 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let response = args[4];
                tracing::info!(
                    entity_id,
                    org_id,
                    response,
                    "UNIMPLEMENTED: pvpOrganizationLeaveResponse"
                );
            }
            true
        }
        MOTD => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationMOTD");
            }
            true
        }
        NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationNote");
            }
            true
        }
        OFFICER_NOTE => {
            if args.len() >= 4 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, org_id, "UNIMPLEMENTED: organizationOfficerNote");
            }
            true
        }
        SET_RANK_PERMISSIONS => {
            if args.len() >= 12 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let permissions = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    rank,
                    permissions,
                    "UNIMPLEMENTED: organizationSetRankPermissions"
                );
            }
            true
        }
        SET_RANK_NAME => {
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let rank = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    rank,
                    "UNIMPLEMENTED: organizationSetRankName"
                );
            }
            true
        }
        SQUAD_SET_LOOT_MODE => {
            if args.len() >= 4 {
                let loot_mode = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, loot_mode, "UNIMPLEMENTED: squadSetLootMode");
            }
            true
        }
        TRANSFER_CASH => {
            if args.len() >= 8 {
                let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let cash = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    org_id,
                    cash,
                    "UNIMPLEMENTED: organizationTransferCash"
                );
            }
            true
        }
        _ => false,
    }
}
