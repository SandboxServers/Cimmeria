//! Dialog display — `onDialogDisplay` (flat method index 105).

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Send `onDialogDisplay` (flat index 105) to the player.
///
/// Wire: `entityId:i32, dialogId:i32, missionFlags:i32, isImmediate:u8, missionId:i32`.
///
/// `npc_entity_id` becomes `args[0..4]` — the wire `EntityId` the
/// client uses as the lookup key for the dialog portrait actor and
/// the per-screen speaker entity (see PR #401, ghidra finding
/// `dialog-portrait-lookup.md`). Recording it as a span field on
/// every send lets operators verify "did the right NPC id reach
/// the wire?" without a packet capture.
///
/// This is the single choke point all dialog-display paths route through
/// (interact-open, monologue, chain `display_dialog`/`start_dialog`,
/// offer-mission), so it's where the server pins
/// [`CellEntity::open_dialog_id`](cimmeria_entity::cell_entity::CellEntity::open_dialog_id) —
/// the "is this dialog open?" precondition the `DialogButtonChoice`
/// handler validates against (CAT-J-01 / #479). The pin is set
/// unconditionally (before the best-effort channel send) because the
/// client treats the dialog as open the moment this method is dispatched;
/// the existing send-failure `warn!` below covers the rare case where the
/// packet never reaches the client.
#[tracing::instrument(
    name = "dialog.send_display",
    level = "info",
    skip_all,
    fields(player_id, npc_entity_id, dialog_id)
)]
pub async fn send_dialog_display(
    player_id: u32,
    npc_entity_id: i32,
    dialog_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Pin the open dialog on the player so DialogButtonChoice can verify
    // the dialog was actually shown before firing its content chain.
    // Overwrites any prior pin — SGW dialogs are strictly sequential, so
    // a new display always supersedes the last (mirrors python
    // `SGWPlayer.displayDialog` setting `self.displayedDialogs[dialogId]`).
    if let Some(player) = space_mgr.get_entity_mut(player_id) {
        player.open_dialog_id = Some(dialog_id);
    }

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
        // failure to deliver onDialogDisplay leaves the
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

    /// dropped onDialogDisplay leaves the player stuck. The
    /// guard drops the receiver before calling the helper so the send
    /// fails synchronously; assertion pins both the WARN level and the
    /// message body, so a revert to `let _ = tx.send(…)` trips it.
    #[tokio::test]
    async fn send_dialog_display_warns_when_cell_to_base_channel_closed() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let mut mgr = crate::test_support::make_space_manager();

        send_dialog_display(
            /* player_id */ 1, /* npc_entity_id */ 100, /* dialog_id */ 42, &tx,
            &mut mgr,
        )
        .await;

        assert!(
            capture
                .find_message(Level::WARN, "DisplayDialog: cell→base send failed")
                .is_some(),
            "negative-logging convention: send_dialog_display must WARN when cell→base channel is closed; \
             reverting to `let _` breaks player-stuck-on-NPC diagnosability"
        );
    }

    /// **#479 set-side guard.** `send_dialog_display` must pin
    /// `open_dialog_id` on the player so the `DialogButtonChoice` handler
    /// has a precondition to validate against. Without this, the gate in
    /// the choice handler can never pass and every dialog choice would be
    /// rejected (the inverse failure mode).
    #[tokio::test]
    async fn send_dialog_display_pins_open_dialog_id() {
        let mut mgr = crate::test_support::make_space_manager();
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, _rx) = mpsc::channel(4);

        send_dialog_display(
            7, /* npc */ 100, /* dialog_id */ 5354, &tx, &mut mgr,
        )
        .await;

        assert_eq!(
            mgr.get_entity(7).and_then(|e| e.open_dialog_id),
            Some(5354),
            "send_dialog_display must pin the dialog so DialogButtonChoice can verify it"
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
