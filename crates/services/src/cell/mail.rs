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
pub async fn handle_request_mail_headers(
    entity_id: u32,
    b_archive: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "requestMailHeaders") else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::RequestHeaders { b_archive },
        })
        .await
    {
        tracing::warn!(entity_id, player_id, "MailRequest send failed: {e}");
    }
}

/// Forward a `requestMailBody` call to BaseApp for DB execution.
pub async fn handle_request_mail_body(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "requestMailBody") else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::RequestBody { mail_id },
        })
        .await
    {
        tracing::warn!(entity_id, player_id, "MailRequest send failed: {e}");
    }
}

/// Forward a `deleteMailMessage` call to BaseApp for DB execution.
pub async fn handle_delete_mail(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "deleteMailMessage") else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::Delete { mail_id },
        })
        .await
    {
        tracing::warn!(entity_id, player_id, "MailRequest send failed: {e}");
    }
}

/// Forward an `archiveMailMessage` call to BaseApp for DB execution.
pub async fn handle_archive_mail(
    entity_id: u32,
    mail_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = resolve_mail_player_id(entity_id, space_mgr, "archiveMailMessage") else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op: MailOp::Archive { mail_id },
        })
        .await
    {
        tracing::warn!(entity_id, player_id, "MailRequest send failed: {e}");
    }
}

// ── Wire format helpers for BaseApp to build mail response packets ───────────

/// Serialize `onMailHeaderInfo` args.
///
/// Wire format:
/// - ResetCategory: UINT8
/// - bArchive: UINT8
/// - MessageHeaders: ARRAY of MessageHeader FIXED_DICT
///   - count: u32 LE
///   - per header: id(i32), fromText(WSTRING), fromId(i32), subjectText(WSTRING),
///     subjectId(i32), cash(i32), sentTime(f32), readTime(f32), flags(i32)
/// - MessageAttachments: ARRAY of MessageAttachment FIXED_DICT (always empty)
///   - count: u32 LE (= 0)
pub fn serialize_on_mail_header_info(b_archive: u8, headers: &[MailHeader]) -> Vec<u8> {
    let mut args = Vec::with_capacity(2 + 4 + headers.len() * 64 + 4);

    // ResetCategory: always 0
    args.push(0u8);
    // bArchive
    args.push(b_archive);

    // MessageHeaders array
    args.extend_from_slice(&(headers.len() as u32).to_le_bytes());
    for h in headers {
        // id: INT32
        args.extend_from_slice(&h.id.to_le_bytes());
        // fromText: WSTRING
        write_wstring(&mut args, &h.from_text);
        // fromId: INT32
        args.extend_from_slice(&h.from_id.to_le_bytes());
        // subjectText: WSTRING
        write_wstring(&mut args, &h.subject_text);
        // subjectId: INT32 (always 0)
        args.extend_from_slice(&0i32.to_le_bytes());
        // cash: INT32
        args.extend_from_slice(&h.cash.to_le_bytes());
        // sentTime: FLOAT
        args.extend_from_slice(&h.sent_time.to_le_bytes());
        // readTime: FLOAT
        args.extend_from_slice(&h.read_time.to_le_bytes());
        // flags: INT32
        args.extend_from_slice(&h.flags.to_le_bytes());
    }

    // MessageAttachments: empty array
    args.extend_from_slice(&0u32.to_le_bytes());

    args
}

/// Serialize `onMailRead` args.
///
/// Wire format:
/// - MailId: INT32
/// - BodyText: WSTRING
/// - BodyId: INT32 (always 0)
/// - ToText: WSTRING (recipient name)
pub fn serialize_on_mail_read(mail_id: i32, body_text: &str, recipient_name: &str) -> Vec<u8> {
    let mut args =
        Vec::with_capacity(4 + 4 + body_text.len() * 2 + 8 + recipient_name.len() * 2 + 8);

    // MailId: INT32
    args.extend_from_slice(&mail_id.to_le_bytes());
    // BodyText: WSTRING
    write_wstring(&mut args, body_text);
    // BodyId: INT32 (always 0)
    args.extend_from_slice(&0i32.to_le_bytes());
    // ToText: WSTRING
    write_wstring(&mut args, recipient_name);

    args
}

/// Serialize `onMailHeaderRemove` args.
///
/// Wire format:
/// - MailId: INT32
pub fn serialize_on_mail_header_remove(mail_id: i32) -> Vec<u8> {
    mail_id.to_le_bytes().to_vec()
}

/// Mail header data from the database.
#[derive(Debug, Clone)]
pub struct MailHeader {
    pub id: i32,
    pub from_text: String,
    pub from_id: i32,
    pub subject_text: String,
    pub cash: i32,
    pub sent_time: f32,
    pub read_time: f32,
    pub flags: i32,
}

/// Write a WSTRING (u32 char_count + UTF-16LE data).
fn write_wstring(buf: &mut Vec<u8>, s: &str) {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
    for &ch in &utf16 {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_empty_mail_headers() {
        let args = serialize_on_mail_header_info(0, &[]);
        // ResetCategory(1) + bArchive(1) + headers count(4) + attachments count(4)
        assert_eq!(args.len(), 1 + 1 + 4 + 4);
        assert_eq!(args[0], 0); // ResetCategory
        assert_eq!(args[1], 0); // bArchive
                                // Headers count = 0
        assert_eq!(u32::from_le_bytes([args[2], args[3], args[4], args[5]]), 0);
        // Attachments count = 0
        assert_eq!(u32::from_le_bytes([args[6], args[7], args[8], args[9]]), 0);
    }

    #[test]
    fn serialize_one_mail_header() {
        let headers = vec![MailHeader {
            id: 42,
            from_text: "Bob".to_string(),
            from_id: 7,
            subject_text: "Hi".to_string(),
            cash: 100,
            sent_time: 1000.0,
            read_time: 0.0,
            flags: 0,
        }];
        let args = serialize_on_mail_header_info(1, &headers);

        // Verify basic structure
        assert_eq!(args[0], 0); // ResetCategory
        assert_eq!(args[1], 1); // bArchive

        // Headers count = 1
        let count = u32::from_le_bytes([args[2], args[3], args[4], args[5]]);
        assert_eq!(count, 1);

        // First header starts at offset 6
        let offset = 6;
        let id = i32::from_le_bytes([
            args[offset],
            args[offset + 1],
            args[offset + 2],
            args[offset + 3],
        ]);
        assert_eq!(id, 42);
    }

    #[test]
    fn serialize_mail_read() {
        let args = serialize_on_mail_read(42, "Hello world", "Alice");

        // MailId
        let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        assert_eq!(mail_id, 42);

        // BodyText WSTRING: char_count
        let text_len = u32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        assert_eq!(text_len, 11); // "Hello world" = 11 chars
    }

    #[test]
    fn serialize_mail_header_remove() {
        let args = serialize_on_mail_header_remove(99);
        assert_eq!(args.len(), 4);
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), 99);
    }

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
