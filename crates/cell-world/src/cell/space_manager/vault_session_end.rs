//! Ending a player's vault session, and its `bank` `vault_session_closed`
//! event (bank-vault BV-02, D-BV05, D-BV19).
//!
//! The session lives on the `CellEntity`, so it cannot outlive the entity;
//! this module makes each end visible. `disconnect_entity` ends it with
//! `logout` before the teardown, `destroy_entity` with `space_change` (every
//! other player destroy is a move to another space), the interaction
//! pin (`cimmeria-cell-interactions`) with `re_pin`, and the end of the
//! player's membership of the organization whose vault is open (the cell's
//! `OrgMembershipEnded` arm, BV-07) with `org_left`. Opening lives beside
//! the Banker arm in `cell::interactions::bank`.

use cimmeria_entity::cell_entity::{PlayerIdentity, VaultCloseReason, VaultSession};

use super::SpaceManager;

impl SpaceManager {
    /// Take the entity's vault session, if any, and log its end.
    pub fn end_vault_session(
        &mut self,
        entity_id: u32,
        reason: VaultCloseReason,
    ) -> Option<VaultSession> {
        let entity = self.get_entity_mut(entity_id)?;
        let identity = entity.identity();
        let session = entity.vault_session.take()?;
        log_vault_session_closed(entity_id, identity, &session, reason);
        Some(session)
    }
}

/// The `vault_session_closed` event (DEBUG, target `bank`).
pub fn log_vault_session_closed(
    entity_id: u32,
    identity: PlayerIdentity,
    session: &VaultSession,
    reason: VaultCloseReason,
) {
    tracing::debug!(
        target: "bank",
        event = "vault_session_closed",
        account_id = identity.account_id,
        player_id = identity.player_id,
        entity_id,
        reason = reason.as_str(),
        scope = session.scope.as_str(),
        org_id = session.org_id,
        banker_id = session.banker_id,
        gm_override = session.banker_id.is_none(),
        space_id = session.space_id,
        open_ms = session.opened_at.elapsed().as_millis() as u64,
        "vault_session_closed: the vault session ended"
    );
}

#[cfg(test)]
mod tests {
    use cimmeria_entity::cell_entity::{VaultCloseReason, VaultScope, VaultSession};
    use tracing::Level;

    use crate::test_support::{make_space_manager, LogCapture};

    const PLAYER: u32 = 1;

    fn staged() -> crate::cell::space_manager::SpaceManager {
        let mut mgr = make_space_manager();
        mgr.create_entity(PLAYER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(PLAYER);
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.account_id = Some(6);
        p.player_id = Some(12);
        p.vault_session = Some(VaultSession {
            org_id: None,
            scope: VaultScope::Personal,
            banker_id: Some(100_001),
            space_id: 1,
            opened_at: std::time::Instant::now(),
            expansion_offer: None,
        });
        mgr
    }

    /// The one `vault_session_closed` row, with every contract field.
    fn assert_closed(capture: &crate::test_support::LogCaptureGuard, reason: &str) {
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.target == "bank" && c.has_field("event", "vault_session_closed"))
            .collect();
        assert_eq!(rows.len(), 1, "exactly one close row: {rows:#?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::DEBUG);
        for (k, v) in [
            ("reason", reason),
            ("scope", "personal"),
            ("account_id", "6"),
            ("player_id", "12"),
            ("entity_id", "1"),
            ("banker_id", "100001"),
            ("gm_override", "false"),
        ] {
            assert!(row.has_field(k, v), "{k}={v} missing: {row:#?}");
        }
        assert!(row.fields.contains_key("open_ms"), "open_ms: {row:#?}");
    }

    /// Logout: `disconnect_entity` logs `reason=logout` once (not again as
    /// `space_change` from the destroy it runs). Fails if the disconnect
    /// hook is removed (the destroy then labels it `space_change`).
    #[tokio::test]
    async fn logout_logs_vault_session_closed_with_reason_logout() {
        let mut mgr = staged();
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        let capture = LogCapture::install();
        mgr.disconnect_entity(PLAYER, &tx).await;
        assert_closed(&capture, "logout");
    }

    /// A destroy for a move to another space logs `reason=space_change`.
    /// Fails if `destroy_entity` stops ending the session.
    #[test]
    fn destroy_logs_vault_session_closed_with_reason_space_change() {
        let mut mgr = staged();
        let capture = LogCapture::install();
        mgr.destroy_entity(PLAYER);
        assert_closed(&capture, "space_change");
    }

    /// `end_vault_session` with `re_pin` (the interaction pin's reason).
    #[test]
    fn end_vault_session_logs_the_given_reason_and_clears_it() {
        let mut mgr = staged();
        let capture = LogCapture::install();
        assert!(mgr
            .end_vault_session(PLAYER, VaultCloseReason::RePin)
            .is_some());
        assert_closed(&capture, "re_pin");
        assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
        assert!(mgr
            .end_vault_session(PLAYER, VaultCloseReason::RePin)
            .is_none());
    }
}
