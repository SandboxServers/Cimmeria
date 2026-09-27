//! OrganizationMember interface exposed CellMethods (indices 8–19).
//!
//! Every method is decoded in full by
//! [`decode_org_cell_method`](cimmeria_wire::cell::cell_methods::organization::decode_org_cell_method)
//! (the `WSTRING`s of CM 13, 14, 15 and 17 included) and logged; none has
//! gameplay behaviour yet. The organizations campaign fills the arms in
//! (ORG-03 squads, ORG-07 forwarding to the base, ORG-08 texts and ranks;
//! `docs/analysis/organizations/work-packets.md`).

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use cimmeria_wire::cell::cell_methods::organization::{decode_org_cell_method, OrgCellCall};
pub use cimmeria_wire::cell::cell_methods::organization::{
    BROADCAST_MINIMAP_PING, INVITE_RESPONSE, LEAVE, MOTD, NOTE, OFFICER_NOTE, PVP_LEAVE_RESPONSE,
    SET_RANK_NAME, SET_RANK_PERMISSIONS, SQUAD_SET_LOOT_MODE, STRIKE_TEAM_RESPONSE, TRANSFER_CASH,
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

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    if !(INVITE_RESPONSE..=TRANSFER_CASH).contains(&method_index) {
        return false;
    }
    match decode_org_cell_method(method_index, args) {
        Ok(call) => {
            tracing::info!(
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
        }
        Err(e) => {
            // A real client always sends the `.def` shape; a malformed
            // payload is a forged or corrupted call.
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

    /// CM 13 used to read only the org id and log it; the MOTD `WSTRING`
    /// was dropped (audit A-02). The dispatcher now decodes it: two UTF-16
    /// units of text reach the log.
    #[tokio::test]
    async fn motd_is_decoded_with_its_text() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let args = [7, 0, 0, 0, 2, 0, 0, 0, 0x48, 0, 0x69, 0];
        assert!(dispatch(1, MOTD, &args, &tx, &mut mgr).await);
        let ev = capture
            .find_message(Level::INFO, "UNIMPLEMENTED: organizationMOTD")
            .expect("decoded MOTD log");
        assert_eq!(ev.target, "org");
        assert!(ev.has_field("text_units", "2"), "{:?}", ev.fields);
    }

    /// A WSTRING whose declared length runs past the payload is rejected
    /// with a reason, not read past or allocated.
    #[tokio::test]
    async fn forged_wstring_length_is_rejected() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        let args = [7, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0];
        assert!(dispatch(1, NOTE, &args, &tx, &mut mgr).await);
        assert!(capture
            .find_event(Level::WARN, "did not decode", "truncated")
            .is_some());
    }

    #[tokio::test]
    async fn indices_outside_8_to_19_are_not_handled() {
        let mut mgr = make_space_manager_with_player(1);
        let (tx, _rx) = mpsc::channel(8);
        assert!(!dispatch(1, 7, &[], &tx, &mut mgr).await);
        assert!(!dispatch(1, 20, &[], &tx, &mut mgr).await);
    }
}
