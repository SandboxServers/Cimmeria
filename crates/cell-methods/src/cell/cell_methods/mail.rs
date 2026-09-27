//! SGWMailManager interface exposed CellMethods (indices 43–51).

use crate::cell::messages::{CellToBaseMsg, MailOp};
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::mail::{
    ARCHIVE_MAIL_MESSAGE, DELETE_MAIL_MESSAGE, PAY_COD_FOR_MAIL, REQUEST_MAIL_BODY,
    REQUEST_MAIL_HEADERS, RETURN_MAIL_MESSAGE, SEND_MAIL_MESSAGE, TAKE_CASH_FROM_MAIL,
    TAKE_ITEM_FROM_MAIL,
};

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
            crate::cell::mail::handle_request_mail_headers(entity_id, b_archive, tx, space_mgr)
                .await;
            true
        }
        SEND_MAIL_MESSAGE => {
            tracing::debug!(entity_id, payload_len = args.len(), "sendMailMessage");
            crate::cell::mail::handle_send_mail(entity_id, args, tx, space_mgr).await;
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
            if let Some(mail_id) = mail_id_arg(entity_id, "returnMailMessage", args) {
                forward(
                    entity_id,
                    "returnMailMessage",
                    MailOp::Return { mail_id },
                    tx,
                    space_mgr,
                )
                .await;
            }
            true
        }
        REQUEST_MAIL_BODY => {
            if args.len() >= 4 {
                let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, mail_id, "requestMailBody");
                crate::cell::mail::handle_request_mail_body(entity_id, mail_id, tx, space_mgr)
                    .await;
            }
            true
        }
        TAKE_CASH_FROM_MAIL => {
            let method = "takeCashFromMailMessage";
            if let Some(mail_id) = mail_id_arg(entity_id, method, args) {
                forward(
                    entity_id,
                    method,
                    MailOp::TakeCash { mail_id },
                    tx,
                    space_mgr,
                )
                .await;
            }
            true
        }
        TAKE_ITEM_FROM_MAIL => {
            // `ContainerId` and `SlotId` ride along for the log only: the
            // shipped client fills them with uninitialised stack (SS-E1
            // M-Q5), so the base chooses the slot itself.
            let method = "takeItemFromMailMessage";
            if let Some(mail_id) = mail_id_arg(entity_id, method, args) {
                let word = |at: usize| {
                    args.get(at..at + 4)
                        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .unwrap_or(0)
                };
                let op = MailOp::TakeItem {
                    mail_id,
                    container_id: word(4),
                    slot_id: word(8),
                };
                forward(entity_id, method, op, tx, space_mgr).await;
            }
            true
        }
        PAY_COD_FOR_MAIL => {
            let method = "payCODForMailMessage";
            if let Some(mail_id) = mail_id_arg(entity_id, method, args) {
                forward(entity_id, method, MailOp::PayCod { mail_id }, tx, space_mgr).await;
            }
            true
        }
        _ => false,
    }
}

/// The leading `INT32 MailId`, or `None` (logged) for a payload too short
/// to hold one.
fn mail_id_arg(entity_id: u32, method: &'static str, args: &[u8]) -> Option<i32> {
    match args.get(..4) {
        Some(b) => Some(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        None => {
            tracing::warn!(
                target: "mail",
                entity_id,
                method,
                payload_len = args.len(),
                reason = "truncated",
                "mail method dropped: payload too short for MailId",
            );
            None
        }
    }
}

async fn forward(
    entity_id: u32,
    method: &'static str,
    op: MailOp,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    tracing::debug!(entity_id, method, "mail attachment op");
    crate::cell::mail::handle_attachment_op(entity_id, method, op, tx, space_mgr).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space_mgr_with_player(entity_id: u32, player_id: i32) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(entity_id, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(entity_id).unwrap().player_id = Some(player_id);
        mgr
    }

    async fn forwarded(method_index: u16, args: &[u8]) -> Option<(i32, MailOp)> {
        let (tx, mut rx) = mpsc::channel(4);
        let mut mgr = space_mgr_with_player(7, 42);
        assert!(dispatch(7, method_index, args, &tx, &mut mgr).await);
        match rx.try_recv().ok()? {
            CellToBaseMsg::MailRequest {
                entity_id: 7,
                player_id,
                op,
            } => Some((player_id, op)),
            other => panic!("unexpected {other:?}"),
        }
    }

    /// SS-M3: CM 49, 50, 51 and 47 reach the base as their `MailOp`, with
    /// the caller's `player_id` from the cell entity. Take-item's
    /// `ContainerId` and `SlotId` are carried for the log exactly as sent
    /// (the base never uses them). Before SS-M3 these were `UNIMPLEMENTED`
    /// log lines that forwarded nothing.
    #[tokio::test]
    async fn attachment_ops_forward_to_base() {
        let id = 99i32.to_le_bytes();
        let mut take_item = id.to_vec();
        take_item.extend_from_slice(&0x5A5A_5A5Ai32.to_le_bytes());
        take_item.extend_from_slice(&(-3i32).to_le_bytes());
        let cases = [
            (TAKE_CASH_FROM_MAIL, id.to_vec(), "TakeCash { mail_id: 99 }"),
            (
                TAKE_ITEM_FROM_MAIL,
                take_item,
                "TakeItem { mail_id: 99, container_id: 1515870810, slot_id: -3 }",
            ),
            (PAY_COD_FOR_MAIL, id.to_vec(), "PayCod { mail_id: 99 }"),
            (RETURN_MAIL_MESSAGE, id.to_vec(), "Return { mail_id: 99 }"),
        ];
        for (method, args, expected) in cases {
            let (player_id, op) = forwarded(method, &args).await.expect("forwarded");
            assert_eq!(player_id, 42);
            assert_eq!(format!("{op:?}"), expected);
        }
    }

    /// A payload too short for the `MailId` is consumed and dropped (logged
    /// `reason=truncated`), never forwarded with a made-up id.
    #[tokio::test]
    async fn truncated_attachment_op_is_not_forwarded() {
        for method in [
            TAKE_CASH_FROM_MAIL,
            TAKE_ITEM_FROM_MAIL,
            PAY_COD_FOR_MAIL,
            RETURN_MAIL_MESSAGE,
        ] {
            assert!(forwarded(method, &[1, 2, 3]).await.is_none());
        }
    }
}
