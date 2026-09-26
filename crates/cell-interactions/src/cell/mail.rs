//! Mail system handlers for the CellService.
//!
//! The mail system requires database access, which lives in the BaseApp.
//! CellService forwards mail requests to BaseApp via [`CellToBaseMsg::MailRequest`],
//! and BaseApp queries the DB and sends the result directly to the client via
//! [`CellToBaseMsg::EntityMethodCall`].
//!
//! Reference: `python/cell/SGWPlayer.py:requestMailHeaders()`, `requestMailBody()`

use tokio::sync::mpsc;

use super::messages::{CellToBaseMsg, MailOp};
use super::space_manager::SpaceManager;

/// Resolve the player_id for a mail-routing entity, refusing to fall back to 0.
///
/// Sending mail ops with player_id=0 risks targeting a sentinel/test row in
/// `sgw_gate_mail` instead of the actual player's mailbox. Returning `None`
/// here makes the caller bail and log rather than misrouting silently.
fn resolve_mail_player_id(entity_id: u32, space_mgr: &SpaceManager, op: &str) -> Option<i32> {
    match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => Some(id),
        None => {
            tracing::warn!(entity_id, op, "mail op dropped: entity has no player_id");
            None
        }
    }
}

/// Forward a `requestMailHeaders` call to BaseApp for DB execution.
#[tracing::instrument(
    name = "mail.request_headers",
    level = "info",
    skip_all,
    fields(entity_id, b_archive)
)]
pub async fn handle_request_mail_headers(
    entity_id: u32,
    b_archive: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "requestMailHeaders") else {
        return;
    };
    let _ = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::RequestHeaders { b_archive },
        })
        .await;
}

/// Forward a `requestMailBody` call to BaseApp for DB execution.
#[tracing::instrument(
    name = "mail.request_body",
    level = "info",
    skip_all,
    fields(entity_id, mail_id)
)]
pub async fn handle_request_mail_body(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "requestMailBody") else {
        return;
    };
    let _ = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::RequestBody { mail_id },
        })
        .await;
}

/// Forward a `deleteMailMessage` call to BaseApp for DB execution.
#[tracing::instrument(
    name = "mail.delete",
    level = "info",
    skip_all,
    fields(entity_id, mail_id)
)]
pub async fn handle_delete_mail(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "deleteMailMessage") else {
        return;
    };
    let _ = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::Delete { mail_id },
        })
        .await;
}

/// Forward an `archiveMailMessage` call to BaseApp for DB execution.
#[tracing::instrument(
    name = "mail.archive",
    level = "info",
    skip_all,
    fields(entity_id, mail_id)
)]
pub async fn handle_archive_mail(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "archiveMailMessage") else {
        return;
    };
    let _ = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::Archive { mail_id },
        })
        .await;
}

// ── Wire format helpers for BaseApp to build mail response packets ───────────
//
// `MailHeader` and the serializers are wire contract and live in
// `cimmeria-wire`; re-exported at their old paths.

pub use cimmeria_wire::cell::mail::{
    serialize_on_mail_header_info, serialize_on_mail_header_remove, serialize_on_mail_read,
    MailHeader,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn space_mgr_with_player(entity_id: u32, player_id: i32) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(entity_id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(entity_id) {
            e.player_id = Some(player_id);
        }
        mgr
    }

    #[tokio::test]
    async fn request_headers_sends_message() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let space_mgr = space_mgr_with_player(1, 42);
        handle_request_mail_headers(1, 0, &tx, &space_mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::MailRequest {
                entity_id,
                player_id,
                op,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 42);
                match op {
                    MailOp::RequestHeaders { b_archive } => assert_eq!(b_archive, 0),
                    _ => panic!("Expected RequestHeaders"),
                }
            }
            _ => panic!("Expected MailRequest"),
        }
    }

    #[tokio::test]
    async fn request_body_sends_message() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let space_mgr = space_mgr_with_player(1, 42);
        handle_request_mail_body(1, 42, &tx, &space_mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::MailRequest {
                entity_id,
                player_id,
                op,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 42);
                match op {
                    MailOp::RequestBody { mail_id } => assert_eq!(mail_id, 42),
                    _ => panic!("Expected RequestBody"),
                }
            }
            _ => panic!("Expected MailRequest"),
        }
    }

    #[tokio::test]
    async fn delete_mail_sends_message() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let space_mgr = space_mgr_with_player(1, 42);
        handle_delete_mail(1, 99, &tx, &space_mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::MailRequest {
                entity_id,
                player_id,
                op,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 42);
                match op {
                    MailOp::Delete { mail_id } => assert_eq!(mail_id, 99),
                    _ => panic!("Expected Delete"),
                }
            }
            _ => panic!("Expected MailRequest"),
        }
    }

    #[tokio::test]
    async fn missing_player_id_drops_request() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let mut space_mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        space_mgr.parse_spaces_xml(spaces_xml).unwrap();
        space_mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        space_mgr
            .create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // entity intentionally has no player_id

        handle_request_mail_headers(1, 0, &tx, &space_mgr).await;
        assert!(
            rx.try_recv().is_err(),
            "Expected no message when player_id is unset"
        );
    }
}
