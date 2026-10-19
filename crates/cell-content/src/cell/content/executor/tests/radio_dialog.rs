//! `Action::DisplayRadioDialog` (DK-06b, D-DK10): a dialog with no speaker
//! entity. The wire `EntityId` of `onDialogDisplay` is 0, with or without an
//! interaction pin, and no NPC is needed.

use super::*;
use crate::test_support::LogCapture;

const PLAYER: u32 = 1;
const RADIO_DIALOG: i32 = 5816;

fn radio() -> ResolvedActions {
    ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            8160,
            Action::DisplayRadioDialog {
                dialog_id: RADIO_DIALOG,
            },
        )],
    }
}

/// The one message sent: `onDialogDisplay` to the player. Returns its
/// `(EntityId, DialogID)`.
fn only_dialog_display(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (i32, i32) {
    let msg = rx.try_recv().expect("must emit onDialogDisplay");
    assert!(rx.try_recv().is_err(), "exactly one message");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, PLAYER, "sent to the player");
            assert_eq!(method_index, crate::mercury::method_idx::ON_DIALOG_DISPLAY);
            (
                i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                i32::from_le_bytes([args[4], args[5], args[6], args[7]]),
            )
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// No NPC and no pin: `display_dialog` would WARN and send nothing. The
/// radio verb sends `EntityId` 0 and the dialog id.
#[tokio::test]
async fn radio_dialog_sends_entity_id_zero_with_no_npc() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_eq!(only_dialog_display(&mut rx), (0, RADIO_DIALOG));
    assert!(
        capture
            .find_message(tracing::Level::WARN, "DisplayDialog: no NPC entity id")
            .is_none(),
        "the radio verb needs no NPC and must not warn"
    );
}

/// A player who just clicked an SG-18 remains still holds that pin; the
/// radio call must not borrow it as the speaker.
#[tokio::test]
async fn radio_dialog_ignores_the_interaction_pin() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(0xBEEF);
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert_eq!(only_dialog_display(&mut rx), (0, RADIO_DIALOG));
}

/// The dialog is recorded as offered, so its close (`dialog_choice`, button
/// -1) passes the #479 gate.
#[tokio::test]
async fn radio_dialog_is_recorded_as_offered() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(8);

    execute_actions(radio(), PLAYER, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .offered_dialogs()
        .contains(&RADIO_DIALOG));
}
