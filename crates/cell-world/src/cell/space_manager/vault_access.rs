//! The vault-session rule every bank move, use and removal must pass
//! (bank-vault BV-02 built it, BV-03 wires it; D-BV05), and the verdict the
//! cell attaches to each inventory request it forwards to the base.
//!
//! It lives here, below `cimmeria-cell-interactions` (which re-exports it
//! beside the Banker), so the content executor can take the verdict too.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_wire::cell::vault::VaultAccess;

use super::interact_range::{interact_range, InteractRangeFail};
use super::SpaceManager;

/// Why a bank move is refused. The label ([`VaultReject::reason`]) is the
/// `vault_reason` field BV-03 logs with `move_rejected`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultReject {
    /// No vault window is open.
    NoSession,
    /// The session was opened in another space than the player is in now.
    /// A space change destroys the entity that held it, so this is a
    /// belt-and-braces check.
    SessionInOtherSpace,
    /// The pinned Banker no longer exists (despawned).
    BankerGone,
    /// The pinned Banker is in another space than the player.
    BankerInOtherSpace,
    /// The player walked out of the interact distance of the Banker.
    BankerOutOfRange {
        /// The distance, in world units.
        dist: f32,
    },
    /// The player entity is not in any space.
    PlayerMissing,
}

impl VaultReject {
    /// Stable `reason` label for the `bank` log target.
    pub fn reason(self) -> &'static str {
        match self {
            VaultReject::NoSession => "no_vault_session",
            VaultReject::SessionInOtherSpace => "vault_session_other_space",
            VaultReject::BankerGone => "banker_gone",
            VaultReject::BankerInOtherSpace => "banker_other_space",
            VaultReject::BankerOutOfRange { .. } => "banker_out_of_range",
            VaultReject::PlayerMissing => "player_missing",
        }
    }
}

/// May `player` move an item into or out of its vault right now?
///
/// Pure: no logging, no sends. The rule (D-BV05):
/// - a vault session must be open, opened in the space the player is in;
/// - with a Banker (`banker_id` is `Some`), the Banker must still exist, be
///   in the player's space, and be within `MAX_INTERACT_DISTANCE`: the same
///   [`interact_range`] rule the opening `interact` passed;
/// - a GM `.bank` session (`banker_id` is `None`) skips the proximity check.
///
/// It does not look at `session.scope`; the caller knows which container it
/// is moving and checks the scope that container needs.
pub fn vault_move_allowed(
    player: &CellEntity,
    space_mgr: &SpaceManager,
) -> Result<(), VaultReject> {
    let session = player
        .vault_session
        .as_ref()
        .ok_or(VaultReject::NoSession)?;
    if i64::from(session.space_id) != i64::from(player.space_id.0) {
        return Err(VaultReject::SessionInOtherSpace);
    }
    let Some(banker_id) = session.banker_id else {
        return Ok(());
    };
    let player_id = player.entity_id.0 as u32;
    interact_range(player_id, banker_id, space_mgr).map_err(|fail| match fail {
        InteractRangeFail::PlayerMissing => VaultReject::PlayerMissing,
        InteractRangeFail::TargetMissing => VaultReject::BankerGone,
        InteractRangeFail::OtherSpace => VaultReject::BankerInOtherSpace,
        InteractRangeFail::TooFar { dist } => VaultReject::BankerOutOfRange { dist },
    })
}

/// The verdict for one forwarded inventory request by `entity_id`: a fresh
/// [`vault_move_allowed`] check, plus the Banker and the distance for the
/// base's log. Taken on every request, whether or not it touches the vault:
/// only the base knows the source row's container, and the check costs two
/// map lookups.
pub fn vault_access(entity_id: u32, space_mgr: &SpaceManager) -> VaultAccess {
    let Some(player) = space_mgr.get_entity(entity_id) else {
        return VaultAccess::Closed {
            reason: VaultReject::PlayerMissing.reason(),
            banker_id: None,
            distance: None,
        };
    };
    let banker_id = player.vault_session.as_ref().and_then(|s| s.banker_id);
    match vault_move_allowed(player, space_mgr) {
        Ok(()) => VaultAccess::Open {
            banker_id,
            distance: banker_id.and_then(|b| banker_distance(entity_id, b, space_mgr)),
        },
        Err(reject) => VaultAccess::Closed {
            reason: reject.reason(),
            banker_id,
            distance: match reject {
                VaultReject::BankerOutOfRange { dist } => Some(dist),
                _ => None,
            },
        },
    }
}

/// Player to Banker distance, for the log of an accepted check.
fn banker_distance(entity_id: u32, banker_id: u32, space_mgr: &SpaceManager) -> Option<f32> {
    let player = space_mgr.get_entity(entity_id)?.position;
    let banker = space_mgr.get_entity(banker_id)?.position;
    Some(player.distance_squared_to(&banker).sqrt())
}
