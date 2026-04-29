//! SGWMailManager interface exposed CellMethods (indices 43–51).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const REQUEST_MAIL_HEADERS: u16 = 43;
pub const SEND_MAIL_MESSAGE: u16 = 44;
pub const ARCHIVE_MAIL_MESSAGE: u16 = 45;
pub const DELETE_MAIL_MESSAGE: u16 = 46;
pub const RETURN_MAIL_MESSAGE: u16 = 47;
pub const REQUEST_MAIL_BODY: u16 = 48;
pub const TAKE_CASH_FROM_MAIL: u16 = 49;
pub const TAKE_ITEM_FROM_MAIL: u16 = 50;
pub const PAY_COD_FOR_MAIL: u16 = 51;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        REQUEST_MAIL_HEADERS => {
            let b_archive = if !args.is_empty() { args[0] } else { 0 };
            tracing::debug!(entity_id, b_archive, "requestMailHeaders");
            crate::cell::mail::handle_request_mail_headers(entity_id, b_archive, tx, space_mgr).await;
            true
        }
        SEND_MAIL_MESSAGE => {
            tracing::info!(entity_id, "UNIMPLEMENTED: sendMailMessage");
            true
        }
        ARCHIVE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "archiveMailMessage");
                crate::cell::mail::handle_archive_mail(entity_id, mail_id, tx, space_mgr).await;
            }
            true
        }
        DELETE_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "deleteMailMessage");
                crate::cell::mail::handle_delete_mail(entity_id, mail_id, tx, space_mgr).await;
            }
            true
        }
        RETURN_MAIL_MESSAGE => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: returnMailMessage");
            }
            true
        }
        REQUEST_MAIL_BODY => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "requestMailBody");
                crate::cell::mail::handle_request_mail_body(entity_id, mail_id, tx, space_mgr).await;
            }
            true
        }
        TAKE_CASH_FROM_MAIL => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: takeCashFromMailMessage");
            }
            true
        }
        TAKE_ITEM_FROM_MAIL => {
            if args.len() >= 12 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let container_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let slot_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                tracing::info!(entity_id, mail_id, container_id, slot_id, "UNIMPLEMENTED: takeItemFromMailMessage");
            }
            true
        }
        PAY_COD_FOR_MAIL => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, mail_id, "UNIMPLEMENTED: payCODForMailMessage");
            }
            true
        }
        _ => false,
    }
}
