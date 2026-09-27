//! OrganizationMember interface exposed CellMethods (indices 8–19).
//!
//! Every method is decoded in full by
//! [`decode_org_cell_method`](cimmeria_wire::cell::cell_methods::organization::decode_org_cell_method)
//! (the `WSTRING`s of CM 13, 14, 15 and 17 included), logged, and answered
//! with [`send_unavailable_feedback`]; none has gameplay behaviour yet. The
//! organizations campaign fills the arms in (ORG-03 squads, ORG-07
//! forwarding to the base, ORG-08 texts and ranks;
//! `docs/analysis/organizations/work-packets.md`).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use cimmeria_wire::cell::cell_methods::organization::{decode_org_cell_method, OrgCellCall};
pub use cimmeria_wire::cell::cell_methods::organization::{
    BROADCAST_MINIMAP_PING, INVITE_RESPONSE, LEAVE, MOTD, NOTE, OFFICER_NOTE, PVP_LEAVE_RESPONSE,
    SET_RANK_NAME, SET_RANK_PERMISSIONS, SQUAD_SET_LOOT_MODE, STRIKE_TEAM_RESPONSE, TRANSFER_CASH,
};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;
use cimmeria_wire::cell::client_methods::player::{
    build_on_error_code, CONDITION_FEEDBACK_INVALID_ENTITY, ERRORCODE_SYSTEM_ABILITY, ON_ERROR_CODE,
};

/// UTF-16 units of client text the call carries, for the log line. The text
/// itself is player-authored and is not logged.
fn text_units(call: &OrgCellCall) -> usize {
    let units = |s: &str| s.encode_utf16().count();
    match call {
        OrgCellCall::Motd { motd, .. } => units(motd),
        OrgCellCall::Note { note, .. } => units(note),
        OrgCellCall::OfficerNote { name, note, .. } => units(name) + units(note),
        OrgCellCall::SetRankName { name, .. } => units(name),
        _ => 0,
    }
}

/// Answer an organization request the server cannot serve yet, so the press
/// is not silent: `onErrorCode(ERRORCODE_SYSTEM_Ability, instance_id,
/// CONDITION_FEEDBACK_InvalidEntity)` and then a feedback chat line, the
/// same pair the base arm for 0xCF-0xD2 sends. The chat line is the part
/// the player sees: the client has no text for an organization error code
/// (ORG-E1 Q4).
pub(crate) async fn send_unavailable_feedback(
    entity_id: u32,
    method_index: u16,
    instance_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let msgs = [
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_ERROR_CODE,
            args: build_on_error_code(
                ERRORCODE_SYSTEM_ABILITY,
                instance_id,
                CONDITION_FEEDBACK_INVALID_ENTITY,
            ),
        },
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication(
                "SYSTEM",
                0,
                CHAN_FEEDBACK,
                ORG_NOT_AVAILABLE_TEXT,
            ),
        },
    ];
    for msg in msgs {
        if tx.send(msg).await.is_err() {
            tracing::warn!(
                target: "org",
                event = "org.feedback_send_failed",
                entity_id,
                method_index,
                reason = "cell_to_base_closed",
                "organization feedback could not be queued"
            );
            return;
        }
    }
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    if !(INVITE_RESPONSE..=TRANSFER_CASH).contains(&method_index) {
        return false;
    }
    match decode_org_cell_method(method_index, args) {
        Ok(call) => {
            tracing::debug!(
                target: "org",
                event = "org.cell_method_unimplemented",
                entity_id,
                method_index,
                method = call.method_name(),
                org_id = call.org_id(),
                text_units = text_units(&call),
                "UNIMPLEMENTED: {}",
                call.method_name()
            );
            let instance_id = call.org_id().unwrap_or(0);
            send_unavailable_feedback(entity_id, method_index, instance_id, tx).await;
        }
        Err(e) => {
            // A real client always sends the `.def` shape; a malformed
            // payload is a forged or corrupted call and gets no answer.
            tracing::warn!(
                target: "org",
                event = "org.cell_method_malformed",
                entity_id,
                method_index,
                reason = e.reason(),
                error = %e,
                "organization cell method payload did not decode"
            );
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::*;
    use crate::test_support::{make_space_manager_with_player, LogCapture};

    /// Drain everything the handler queued for the base.
    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u16, Vec<u8>)> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } => out.push((entity_id, method_index, args)),
                other => panic!("unexpected {other:?}"),
            }
        }
        out
    }

    /// CM 13 used to read only the org id; the MOTD `WSTRING` was dropped
    /// (audit A-02). The dispatcher decodes it (two UTF-16 units reach the
    /// DEBUG log) and answers: `onErrorCode` with the org id, then the
    /// feedback line.
    #[tokio::test]
    async fn motd_is_decoded_and_answered() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, mut rx) = mpsc::channel(8);
        let args = [7, 0, 0, 0, 2, 0, 0, 0, 0x48, 0, 0x69, 0];
        assert!(dispatch(1, MOTD, &args, &tx, &mut mgr).await);
        let ev = capture
            .find_message(Level::DEBUG, "UNIMPLEMENTED: organizationMOTD")
            .expect("decoded MOTD log");
        assert_eq!(ev.target, "org");
        assert!(ev.has_field("text_units", "2"), "{:?}", ev.fields);

        let sent = drain(&mut rx);
        assert_eq!(sent.len(), 2, "error code + feedback line");
        // onErrorCode(0, InstanceID 7, 0).
        assert_eq!(sent[0], (1, 121, vec![0, 7, 0, 0, 0, 0, 0]));
        assert_eq!(sent[1].1, 28, "onPlayerCommunication");
        assert_eq!(
            sent[1].2,
            serialize_on_player_communication(
                "SYSTEM",
                0,
                9,
                "Organizations are not available yet."
            )
        );
    }

    /// Every method 8-19 answers, including the two with no org id
    /// (instance 0).
    #[tokio::test]
    async fn every_org_method_is_answered() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, mut rx) = mpsc::channel(64);
        let cases: [(u16, Vec<u8>, i32); 12] = [
            (8, vec![1, 0, 0, 0, 1], 0),
            (9, vec![5, 0, 0, 0], 5),
            (10, [&[5u8, 0, 0, 0][..], &[0; 12]].concat(), 5),
            (11, vec![5, 0, 0, 0, 1], 5),
            (12, vec![5, 0, 0, 0, 1], 5),
            (13, vec![5, 0, 0, 0, 0, 0, 0, 0], 5),
            (14, vec![5, 0, 0, 0, 0, 0, 0, 0], 5),
            (15, vec![5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 5),
            (16, vec![5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0], 5),
            (17, vec![5, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0x41, 0], 5),
            (18, vec![1, 0, 0, 0], 0),
            (19, vec![5, 0, 0, 0, 1, 0, 0, 0], 5),
        ];
        for (idx, args, instance) in cases {
            assert!(dispatch(1, idx, &args, &tx, &mut mgr).await);
            let sent = drain(&mut rx);
            assert_eq!(sent.len(), 2, "CM {idx}");
            assert_eq!(sent[0].1, 121, "CM {idx}");
            assert_eq!(&sent[0].2[1..5], &instance.to_le_bytes(), "CM {idx}");
        }
    }

    /// A WSTRING whose declared length runs past the payload is rejected
    /// with a reason, not read past or allocated, and not answered.
    #[tokio::test]
    async fn forged_wstring_length_is_rejected() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, mut rx) = mpsc::channel(8);
        let args = [7, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0];
        assert!(dispatch(1, NOTE, &args, &tx, &mut mgr).await);
        assert!(capture
            .find_event(Level::WARN, "did not decode", "truncated")
            .is_some());
        assert!(drain(&mut rx).is_empty());
    }

    #[tokio::test]
    async fn indices_outside_8_to_19_are_not_handled() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        assert!(!dispatch(1, 7, &[], &tx, &mut mgr).await);
        assert!(!dispatch(1, 20, &[], &tx, &mut mgr).await);
    }
}
