//! The vault-open client-method payload: `onVaultOpen` (106),
//! `onTeamVaultOpen` (107) and `onCommandVaultOpen` (108) all carry
//! `(INT32 EntityId, VECTOR3 Position)`
//! (`docs/protocol/client-method-dispatch-table.md:253-255`,
//! `entities/defs/SGWPlayer.def`).
//!
//! The client only shows the window: `Event_UI_VaultVisibility` runs
//! `VaultMod.onVaultVisibility` (audit A-02). Nothing on the client reads
//! `Position` (BV-E1 Q4), so it is sent for wire fidelity and no server
//! behaviour depends on the client using it.

use cimmeria_entity::cell_entity::VaultScope;

/// The Banker's "Expand vault" dialog (bank-vault BV-05, D-BV02): one
/// screen with one Generic 1 button, authored as a Cimmeria cooked-data
/// override (`cimmeria-resources` `DIALOG_OVERRIDES`). The Bank campaign
/// owns dialog ids 60110-60119 and screen ids 200010-200019.
///
/// The client's reply (`dialogButtonChoice`) is routed by this id to the
/// purchase path, never to a content chain. The reply is not an authority
/// check: the purchase re-checks the vault session, the Banker's
/// proximity, the cash and the ceiling.
pub const VAULT_EXPAND_DIALOG_ID: i32 = 60110;

/// Slots one expansion adds (D-BV02: 40 to 100 in steps of 10). The
/// `bank_slots_sanity` CHECK on `sgw_player` enforces the same grid.
pub const VAULT_EXPAND_STEP: i16 = 10;

/// Serialize the `(INT32 EntityId, VECTOR3 Position)` args of a vault-open
/// method: 4 bytes of LE `i32`, then three LE `f32` (x, y, z). 16 bytes.
pub fn build_vault_open_args(entity_id: i32, position: [f32; 3]) -> Vec<u8> {
    let mut args = Vec::with_capacity(16);
    args.extend_from_slice(&entity_id.to_le_bytes());
    for axis in position {
        args.extend_from_slice(&axis.to_le_bytes());
    }
    args
}

/// The vault-session verdict the cell attaches to every inventory request it
/// forwards to the base (`moveItem`, `useItem`, `removeItem`, content
/// `RemoveItem`, `gmRemoveItem`). Bank campaign BV-03, D-BV05.
///
/// The session and the positions live in the cell and the inventory
/// transaction runs on the base, so the cell takes the verdict when it
/// forwards the request: every request gets its own check, including a
/// fresh proximity check against the pinned Banker. The base consults it only
/// when the request touches the personal vault (17), and ignores it
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultAccess {
    /// A session is open in the player's space and passed its check.
    Open {
        /// Which vault the session opened. Only a `Personal` session opens
        /// container 17; the org vaults will need their own scope.
        scope: VaultScope,
        /// The pinned Banker; `None` for a GM `.bank` session, which skips
        /// the proximity check.
        banker_id: Option<u32>,
        /// Player to Banker distance at the check; `None` for a GM session.
        distance: Option<f32>,
    },
    /// No session, or the session failed its check.
    Closed {
        /// `VaultReject::reason()`: `no_vault_session`,
        /// `vault_session_other_space`, `banker_gone`, `banker_other_space`,
        /// `banker_out_of_range` or `player_missing`.
        reason: &'static str,
        /// The pinned Banker, when a session named one.
        banker_id: Option<u32>,
        /// The distance, when the refusal is `banker_out_of_range`.
        distance: Option<f32>,
    },
}

impl VaultAccess {
    /// No vault session: the verdict for in-process callers that never
    /// reach the vault (right-click auto-equip) and the default in tests.
    pub const NO_SESSION: VaultAccess = VaultAccess::Closed {
        reason: "no_vault_session",
        banker_id: None,
        distance: None,
    };

    /// Whether a session of any scope is open and passed its check.
    pub fn is_open(&self) -> bool {
        matches!(self, VaultAccess::Open { .. })
    }

    /// Whether the personal vault (17) may be touched by this request: an
    /// open session of the `Personal` scope. A Team or Command session does
    /// not open it.
    pub fn opens_personal_vault(&self) -> bool {
        matches!(
            self,
            VaultAccess::Open {
                scope: VaultScope::Personal,
                ..
            }
        )
    }

    /// Why the personal vault is shut to this request: the check's refusal
    /// label, or `vault_scope_mismatch` for an open org-vault session.
    /// `None` when [`Self::opens_personal_vault`].
    pub fn personal_vault_refusal(&self) -> Option<&'static str> {
        match self {
            VaultAccess::Open {
                scope: VaultScope::Personal,
                ..
            } => None,
            VaultAccess::Open { .. } => Some("vault_scope_mismatch"),
            VaultAccess::Closed { reason, .. } => Some(reason),
        }
    }

    /// The check's refusal label, `None` when open (any scope).
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            VaultAccess::Open { .. } => None,
            VaultAccess::Closed { reason, .. } => Some(reason),
        }
    }

    /// The pinned Banker, when there is one.
    pub fn banker_id(&self) -> Option<u32> {
        match self {
            VaultAccess::Open { banker_id, .. } | VaultAccess::Closed { banker_id, .. } => {
                *banker_id
            }
        }
    }

    /// The Banker distance measured by the check, when there was one.
    pub fn distance(&self) -> Option<f32> {
        match self {
            VaultAccess::Open { distance, .. } | VaultAccess::Closed { distance, .. } => *distance,
        }
    }

    /// An open GM `.bank` session (no Banker, no proximity check).
    pub fn gm_override(&self) -> bool {
        matches!(
            self,
            VaultAccess::Open {
                banker_id: None,
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only a personal session opens the personal vault; an org session
    /// is refused as `vault_scope_mismatch`, a closed one with its reason.
    #[test]
    fn only_a_personal_session_opens_the_personal_vault() {
        let open = |scope| VaultAccess::Open {
            scope,
            banker_id: Some(9),
            distance: Some(1.0),
        };
        assert!(open(VaultScope::Personal).opens_personal_vault());
        assert_eq!(open(VaultScope::Personal).personal_vault_refusal(), None);
        for scope in [VaultScope::Team, VaultScope::Command] {
            assert!(open(scope).is_open());
            assert!(!open(scope).opens_personal_vault());
            assert_eq!(
                open(scope).personal_vault_refusal(),
                Some("vault_scope_mismatch")
            );
        }
        assert_eq!(
            VaultAccess::NO_SESSION.personal_vault_refusal(),
            Some("no_vault_session")
        );
        assert!(!VaultAccess::NO_SESSION.opens_personal_vault());
    }

    /// Byte-exact: INT32 id, then x, y, z as LE f32, no marker or padding.
    /// A reordered or widened field shifts every byte after it.
    #[test]
    fn vault_open_args_are_int32_then_vector3() {
        let args = build_vault_open_args(0x0001_86A5, [1.5, -2.0, 300.25]);
        assert_eq!(
            args,
            vec![
                0xA5, 0x86, 0x01, 0x00, // EntityId 100005
                0x00, 0x00, 0xC0, 0x3F, // x = 1.5
                0x00, 0x00, 0x00, 0xC0, // y = -2.0
                0x00, 0x20, 0x96, 0x43, // z = 300.25
            ]
        );
    }
}
