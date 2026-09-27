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
/// offer-mission), so it's where the server records the dialog in
/// [`CellEntity::offered_dialog_ids`](cimmeria_entity::cell_entity::CellEntity::offered_dialog_ids) —
/// the "was this dialog offered?" precondition the `DialogButtonChoice`
/// handler validates against (CAT-J-01 / #479). The id is recorded
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
    // Record the dialog as offered so DialogButtonChoice can verify the
    // player was actually shown it before firing its content chain.
    // Additive, not a replace: the client holds two dialogs at once and
    // answers an evicted one AFTER the replacement has been displayed,
    // so a single pin would reject that answer and lose its chain
    // (mirrors python `SGWPlayer.displayDialog` inserting into the
    // `displayedDialogs` dict).
    if let Some(player) = space_mgr.get_entity_mut(player_id) {
        if let Some(evicted_dialog_id) = player.offer_dialog(dialog_id) {
            // Losing an offer means a dialog this player was shown can no
            // longer be answered — its `dialog_choice` chain will never
            // fire. Only reachable if a player accumulates more than
            // MAX_OFFERED_DIALOGS unanswered dialogs, which no shipped
            // content does; a sustained stream of these points at a chain
            // displaying dialogs in a loop.
            tracing::warn!(
                player_id,
                dialog_id,
                evicted_dialog_id,
                max_offered = cimmeria_entity::cell_entity::MAX_OFFERED_DIALOGS,
                offered_dialog_ids = ?player.offered_dialogs(),
                "DisplayDialog: offered-dialog set full -- evicted the oldest \
                 unanswered dialog; its dialog_choice chain can no longer fire"
            );
        }
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

    /// **#479 set-side guard.** `send_dialog_display` must record the
    /// dialog as offered so the `DialogButtonChoice` handler has a
    /// precondition to validate against. Without this, the gate in the
    /// choice handler can never pass and every dialog choice would be
    /// rejected (the inverse failure mode).
    #[tokio::test]
    async fn send_dialog_display_records_the_offered_dialog() {
        let mut mgr = crate::test_support::make_space_manager();
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, _rx) = mpsc::channel(4);

        send_dialog_display(
            7, /* npc */ 100, /* dialog_id */ 5354, &tx, &mut mgr,
        )
        .await;

        assert_eq!(
            mgr.get_entity(7).map(|e| e.offered_dialogs()),
            Some(vec![5354]),
            "send_dialog_display must offer the dialog so DialogButtonChoice can verify it"
        );
    }

    /// **DU-08 / F13.** Displaying B must NOT drop A. The client keeps A
    /// in its slot until B evicts it, and then answers A late; a
    /// replacing pin loses that answer. Both ids stay offered, A first.
    #[tokio::test]
    async fn a_second_display_keeps_the_first_dialog_offered() {
        let mut mgr = crate::test_support::make_space_manager();
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, _rx) = mpsc::channel(8);

        send_dialog_display(7, 100, /* A */ 2574, &tx, &mut mgr).await;
        send_dialog_display(7, 100, /* B */ 2576, &tx, &mut mgr).await;

        assert_eq!(
            mgr.get_entity(7).map(|e| e.offered_dialogs()),
            Some(vec![2574, 2576]),
            "displaying B must keep A offered -- the client answers the evicted \
             A after B was displayed (F13), and a replacing pin would reject it"
        );
    }

    /// **DU-08 overflow guard.** The set is bounded; the eviction is a
    /// player-visible loss (that dialog's chain can never fire), so it
    /// warns per the negative-logging convention.
    #[tokio::test]
    async fn send_dialog_display_warns_when_the_offered_set_overflows() {
        use cimmeria_entity::cell_entity::MAX_OFFERED_DIALOGS;
        let capture = LogCapture::install();
        let mut mgr = crate::test_support::make_space_manager();
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, _rx) = mpsc::channel(64);

        for i in 0..MAX_OFFERED_DIALOGS as i32 {
            send_dialog_display(7, 100, 9000 + i, &tx, &mut mgr).await;
        }
        assert!(
            capture
                .find_message(Level::WARN, "offered-dialog set full")
                .is_none(),
            "filling the set exactly to the bound must not warn"
        );

        send_dialog_display(7, 100, 9999, &tx, &mut mgr).await;

        assert!(
            capture
                .find_message(Level::WARN, "offered-dialog set full")
                .is_some(),
            "negative-logging convention: overflow drops a dialog the player was \
             shown, so it must WARN -- silence here hides a chain that can never fire"
        );
        let offered = mgr.get_entity(7).map(|e| e.offered_dialogs()).unwrap();
        assert_eq!(
            offered.len(),
            MAX_OFFERED_DIALOGS,
            "the set must stay bounded"
        );
        assert_eq!(offered[0], 9001, "the OLDEST offer is the one evicted");
    }

    /// **DU-08 teardown.** Per-session dialog state dies with the cell
    /// entity: logout, cross-world travel and GM despawn all route
    /// through `destroy_entity`, which removes the `CellEntity` (and with
    /// it the offered set) from its space. A re-created entity on the
    /// same id must start empty, or a relog would inherit answerable
    /// dialog ids from the previous session.
    #[tokio::test]
    async fn the_offered_set_does_not_survive_entity_teardown() {
        let mut mgr = crate::test_support::make_space_manager();
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, _rx) = mpsc::channel(4);
        send_dialog_display(7, 100, 2576, &tx, &mut mgr).await;
        assert!(mgr.get_entity(7).unwrap().dialog_is_offered(2576));

        mgr.destroy_entity(7);
        mgr.create_entity(7, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        assert_eq!(
            mgr.get_entity(7).map(|e| e.offered_dialogs()),
            Some(Vec::new()),
            "a new session on a reused entity id must not inherit offered dialogs"
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
