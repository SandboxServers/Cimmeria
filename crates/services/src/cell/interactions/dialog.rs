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
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // EntityId
    args.extend_from_slice(&dialog_id.to_le_bytes());       // DialogID
    args.extend_from_slice(&0i32.to_le_bytes());            // MissionFlags
    args.push(1);                                           // IsImmediate
    args.extend_from_slice(&0i32.to_le_bytes());            // aMissionId

    tracing::debug!(player_id, npc_entity_id, dialog_id, "Sending onDialogDisplay");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: 105, // onDialogDisplay
        args,
    }).await;
}

#[cfg(test)]
mod tests {
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
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), npc_id);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), dialog_id);
        assert_eq!(args[12], 1); // isImmediate
    }
}
