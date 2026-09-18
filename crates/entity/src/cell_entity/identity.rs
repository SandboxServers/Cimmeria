//! Stable player identity for log correlation.
//!
//! # Why this exists
//!
//! `entity_id` is **not** a stable identity. It is a recycled per-space slot
//! integer: when a player disconnects their id returns to the pool and a
//! later connection can be handed the same number within the same hour. A
//! SigNoz query filtered on `entity_id` therefore answers "what happened in
//! this slot", not "what did this player do".
//!
//! `account_id` (the login) and `player_id` (the `sgw_player` character row)
//! are both stable for the life of a session, and `account_id` is stable
//! across reconnects and character switches. Stamping the pair onto every
//! log a connection produces is what makes "show me everything account 6 did"
//! a single log filter instead of a forensic reconstruction from
//! `access_level` values and wall-clock proximity.
//!
//! # The `Option` contract
//!
//! Both fields are `Option` and are handed to `tracing` **as `Option`s**, not
//! unwrapped. `tracing`'s `impl<T: Value> Value for Option<T>` records
//! nothing when the value is `None`, so:
//!
//! - a player's log line carries `account_id=6 player_id=12`, and
//! - an NPC's log line carries neither field at all — rather than the
//!   useless-to-filter-on `account_id="None"`.
//!
//! That is deliberate: NPCs have no account, and a literal `"None"` string
//! would pollute the field's value cardinality in the log store. Never
//! `unwrap_or(0)` these — a sentinel 0 is indistinguishable from a real id
//! in a query.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 5.

/// The stable identity correlator pair for one player connection.
///
/// Built from [`CellEntity`](super::CellEntity) on the cell side and from
/// `ConnectedClientState` on the base side, so both halves of the server
/// emit the same two field names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerIdentity {
    /// Owning `account.account_id`. `None` for NPCs and for a player entity
    /// whose session identity has not been threaded in yet.
    pub account_id: Option<u32>,
    /// The `sgw_player.player_id` of the character being played. `None` for
    /// NPCs and before character select.
    pub player_id: Option<i32>,
}

impl PlayerIdentity {
    /// Identity with neither field known — the correct value for an NPC or
    /// an entity id that doesn't resolve. Both fields being `None` means
    /// both tracing fields are omitted.
    pub const UNKNOWN: Self = Self {
        account_id: None,
        player_id: None,
    };

    /// Build from the two raw halves.
    #[must_use]
    pub fn new(account_id: Option<u32>, player_id: Option<i32>) -> Self {
        Self {
            account_id,
            player_id,
        }
    }

    /// `true` when at least one half is known, i.e. at least one field will
    /// actually be emitted. Mostly useful in tests and assertions.
    #[must_use]
    pub fn is_known(&self) -> bool {
        self.account_id.is_some() || self.player_id.is_some()
    }
}

impl super::CellEntity {
    /// This entity's stable log-correlation identity.
    ///
    /// Returns [`PlayerIdentity::UNKNOWN`] for NPCs, whose `account_id` and
    /// `player_id` are both `None`.
    #[must_use]
    pub fn identity(&self) -> PlayerIdentity {
        PlayerIdentity {
            account_id: self.account_id,
            player_id: self.player_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerIdentity;
    use crate::cell_entity::CellEntity;
    use cimmeria_common::{EntityId, SpaceId, Vector3};

    fn entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(1), Vector3::zero())
    }

    #[test]
    fn fresh_entity_identity_is_unknown() {
        // A just-created entity (and every NPC, which never gets these set)
        // must report UNKNOWN so both tracing fields are omitted.
        assert_eq!(entity().identity(), PlayerIdentity::UNKNOWN);
        assert!(!entity().identity().is_known());
    }

    #[test]
    fn identity_mirrors_the_entity_fields() {
        let mut e = entity();
        e.account_id = Some(6);
        e.player_id = Some(12);
        assert_eq!(e.identity(), PlayerIdentity::new(Some(6), Some(12)));
        assert!(e.identity().is_known());
    }

    #[test]
    fn half_known_identity_is_still_known() {
        // Between CreateEntity and InitPlayerState an entity can legitimately
        // have account_id without player_id; the account half alone is still
        // enough to answer "which account is this".
        let mut e = entity();
        e.account_id = Some(6);
        assert_eq!(e.identity().player_id, None);
        assert!(e.identity().is_known());
    }
}
