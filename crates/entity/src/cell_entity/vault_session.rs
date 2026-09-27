//! The vault session: the server's record that a player's vault window is
//! open, and at whose counter (bank-vault campaign, D-BV05).
//!
//! The client sends nothing when the vault window closes (BV-E1 Q4, audit
//! A-03), and it ignores `onVaultOpen`'s `Position`, so it never closes the
//! window on walk-away either. The session therefore ends only on the
//! server's own signals: a space change or logout (both destroy the
//! `CellEntity` that holds it), or a later `interact` that pins a different
//! target ([`super::CellEntity::pin_interaction_target`]). Moves into or out
//! of the bank re-check the Banker's proximity on every move (BV-03), so a
//! stale session that outlives a walk-away still moves nothing.

/// Which vault a Banker opens: `entity_templates.vault_scope`, read only when
/// the template carries `INT_BANKER` (D-BV09). The legacy `EInteractionType`
/// numbering is not used; there is no Team or Command banker bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultScope {
    /// The player's own vault, container 17 (`onVaultOpen`, 106).
    Personal,
    /// The Team organization vault, container 19 (`onTeamVaultOpen`, 107).
    Team,
    /// The Command organization vault, container 20 (`onCommandVaultOpen`, 108).
    Command,
}

impl VaultScope {
    /// The `entity_templates.vault_scope` text for this scope.
    pub fn as_str(self) -> &'static str {
        match self {
            VaultScope::Personal => "personal",
            VaultScope::Team => "team",
            VaultScope::Command => "command",
        }
    }
}

/// Parse the `entity_templates.vault_scope` column. The column's `CHECK`
/// allows exactly these three values; anything else is an error, never a
/// silent default.
impl TryFrom<&str> for VaultScope {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "personal" => Ok(VaultScope::Personal),
            "team" => Ok(VaultScope::Team),
            "command" => Ok(VaultScope::Command),
            other => Err(format!("unknown vault_scope {other:?}")),
        }
    }
}

/// Why a vault session ended: the `reason` of the `bank`
/// `vault_session_closed` event (D-BV19 telemetry contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultCloseReason {
    /// The player's cell entity was destroyed for a move to another space
    /// (cross-world travel, a GM transfer, a gate trip).
    SpaceChange,
    /// The client disconnected or logged off.
    Logout,
    /// A later `interact` pinned a different target.
    RePin,
}

impl VaultCloseReason {
    /// The stable `reason` string.
    pub fn as_str(self) -> &'static str {
        match self {
            VaultCloseReason::SpaceChange => "space_change",
            VaultCloseReason::Logout => "logout",
            VaultCloseReason::RePin => "re_pin",
        }
    }
}

/// An open vault window, on the player's `CellEntity`.
///
/// Set only by the Banker interaction arm and by the GM `.bank` command.
#[derive(Debug, Clone, PartialEq)]
pub struct VaultSession {
    /// Which vault is open. Only `Personal` is ever set until the org
    /// vaults land (Wave 4).
    pub scope: VaultScope,
    /// The Banker the session is pinned to. `None` is a GM `.bank` session,
    /// which has no counter and skips the proximity check.
    pub banker_id: Option<u32>,
    /// The space the vault was opened in.
    pub space_id: u32,
    /// When the vault was opened (server-local clock).
    pub opened_at: std::time::Instant,
}

impl super::CellEntity {
    /// Pin `target` as this player's interaction target
    /// (`last_interaction_target`), ending any vault session that is not
    /// pinned to that same target. Returns the session it ended, so the
    /// caller can log it.
    ///
    /// Every `interact` pin goes through here, which is how "a new
    /// interaction target ends the session" (D-BV05) holds. Re-clicking the
    /// same Banker keeps the session; a GM `.bank` session (no Banker) ends
    /// on any pin.
    pub fn pin_interaction_target(&mut self, target: u32) -> Option<VaultSession> {
        self.last_interaction_target = Some(target);
        if self
            .vault_session
            .as_ref()
            .is_some_and(|s| s.banker_id != Some(target))
        {
            return self.vault_session.take();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_entity::CellEntity;
    use cimmeria_common::{EntityId, SpaceId, Vector3};

    fn session(banker_id: Option<u32>) -> VaultSession {
        VaultSession {
            scope: VaultScope::Personal,
            banker_id,
            space_id: 1,
            opened_at: std::time::Instant::now(),
        }
    }

    /// A pin of a different target ends the session; re-pinning the same
    /// Banker keeps it; a GM session (no Banker) ends on any pin. Fails if
    /// the re-pin clear is removed.
    #[test]
    fn pin_of_a_different_target_ends_the_vault_session() {
        let mut p = CellEntity::new(EntityId(1), SpaceId(1), Vector3::new(0.0, 0.0, 0.0));

        p.vault_session = Some(session(Some(500)));
        assert_eq!(p.pin_interaction_target(500), None);
        assert!(p.vault_session.is_some(), "same Banker keeps the session");

        let ended = p.pin_interaction_target(501);
        assert_eq!(ended.and_then(|s| s.banker_id), Some(500));
        assert!(p.vault_session.is_none(), "another target ends it");
        assert_eq!(p.last_interaction_target, Some(501));

        p.vault_session = Some(session(None));
        assert!(p.pin_interaction_target(500).is_some());
        assert!(p.vault_session.is_none(), "a GM session ends on any pin");
    }

    /// Every value the column's CHECK allows round-trips; anything else is
    /// refused rather than defaulting to `Personal`.
    #[test]
    fn vault_scope_parses_exactly_the_checked_values() {
        for scope in [VaultScope::Personal, VaultScope::Team, VaultScope::Command] {
            assert_eq!(VaultScope::try_from(scope.as_str()), Ok(scope));
        }
        for bad in ["", "Personal", "guild", "org"] {
            assert!(VaultScope::try_from(bad).is_err(), "{bad:?} must not parse");
        }
    }
}
