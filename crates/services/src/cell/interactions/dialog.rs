//! Dialog display — `onDialogDisplay` (flat method index 105).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

/// Send `onDialogDisplay` (flat index 105) to the player.
///
/// Wire: `entityId:i32, dialogId:i32, missionFlags:i32, isImmediate:u8, missionId:i32`.
pub async fn send_dialog_display(
    player_id: u32,
    npc_entity_id: i32,
    dialog_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&npc_entity_id.to_le_bytes()); // EntityId
    args.extend_from_slice(&dialog_id.to_le_bytes()); // DialogID
    args.extend_from_slice(&0i32.to_le_bytes()); // MissionFlags
    args.push(1); // IsImmediate
    args.extend_from_slice(&0i32.to_le_bytes()); // aMissionId

    tracing::debug!(
        player_id,
        npc_entity_id,
        dialog_id,
        "Sending onDialogDisplay"
    );
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: player_id,
            method_index: crate::mercury::method_idx::ON_DIALOG_DISPLAY,
            args,
        })
        .await
    {
        // Issue #304: failure to deliver onDialogDisplay leaves the
        // player stuck — they interacted with an NPC and nothing
        // happens. warn! because it's player-visible.
        tracing::warn!(
            player_id,
            npc_entity_id,
            dialog_id,
            "DisplayDialog: cell→base send failed -- dialog not opened on client: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tokio::sync::mpsc;
    use tracing::Level;

    /// Issue #304: dropped onDialogDisplay leaves the player stuck. The
    /// guard drops the receiver before calling the helper so the send
    /// fails synchronously; assertion pins both the WARN level and the
    /// message body, so a revert to `let _ = tx.send(…)` trips it.
    #[tokio::test]
    async fn send_dialog_display_warns_when_cell_to_base_channel_closed() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel(1);
        drop(rx);

        send_dialog_display(
            /* player_id */ 1, /* npc_entity_id */ 100, /* dialog_id */ 42, &tx,
        )
        .await;

        assert!(
            capture
                .find_message(Level::WARN, "DisplayDialog: cell→base send failed")
                .is_some(),
            "issue #304: send_dialog_display must WARN when cell→base channel is closed; \
             reverting to `let _` breaks player-stuck-on-NPC diagnosability"
        );
    }

    #[test]
    fn dialog_display_args_format() {
        let mut args = Vec::new();
        let npc_id: i32 = 100_000;
        let dialog_id: i32 = 42;
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&dialog_id.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes()); // missionFlags
        args.push(1); // isImmediate
        args.extend_from_slice(&0i32.to_le_bytes()); // missionId

        assert_eq!(args.len(), 17);
        assert_eq!(
            i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
            npc_id
        );
        assert_eq!(
            i32::from_le_bytes([args[4], args[5], args[6], args[7]]),
            dialog_id
        );
        assert_eq!(args[12], 1); // isImmediate
    }
}
