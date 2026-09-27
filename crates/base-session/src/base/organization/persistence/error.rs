//! The typed outcome of an organization write that did not happen.

use cimmeria_entity::organization::{OrgRank, TextReject};

/// Why an organization write was refused. Every `rows_affected == 0` path
/// maps to one of these instead of returning `Ok`, so a caller cannot
/// mistake a miss for a success and fan out a change that never happened.
///
/// Every variant except [`OrgStoreError::Db`] leaves the transaction usable
/// and unchanged: the checks run before the write, the inserts that can
/// collide use `ON CONFLICT DO NOTHING`, and `create_org` rolls its writes
/// back to a savepoint. After `Db` the caller must drop the transaction.
#[derive(Debug, thiserror::Error)]
pub enum OrgStoreError {
    /// No organization with that id (never created, or disbanded).
    #[error("no such organization")]
    NoSuchOrg,
    /// The player is not a member of that organization.
    #[error("not a member of the organization")]
    NotAMember,
    /// The player is already a member of that organization.
    #[error("already a member of the organization")]
    AlreadyMember,
    /// The player is already in another organization of the same type
    /// (D-ORG18: one Team and one Command per player).
    #[error("already in an organization of this type")]
    AlreadyInType,
    /// Another organization of the same type has this name (compared on the
    /// case-folded `name_key`, D-ORG10).
    #[error("an organization of this type already has that name")]
    NameTaken,
    /// The organization's type does not use this rank (D-ORG07), so it has
    /// no rank row for it.
    #[error("rank {} is not used by this organization type", .0.as_u8())]
    RankNotInType(OrgRank),
    /// The `Leader` rank is pinned: a rank change may neither assign it nor
    /// move the current leader off it, and its permission row always holds
    /// every bit (D-ORG08, D-ORG09 (4)). Only a memberless organization may
    /// take a new member directly as `Leader`.
    #[error("the Leader rank is pinned")]
    LeaderPinned,
    /// A memberless organization (D-ORG20) may only take a new member as
    /// `Leader`; anything else would leave it without one.
    #[error("a memberless organization needs a Leader first")]
    NeedsLeader,
    /// The text failed the D-ORG10 rules.
    #[error("invalid text: {0}")]
    InvalidText(TextReject),
    /// The vault or treasury is not empty, so the organization may not be
    /// disbanded (D-ORG20).
    #[error("the organization's vault is not empty")]
    VaultNotEmpty,
    /// Squads are cell state and are never persisted (D-ORG03).
    #[error("squads are not persisted")]
    NotPersistent,
    /// The player id names no character.
    #[error("no such character")]
    NoSuchPlayer,
    /// Any other database failure. The transaction is aborted.
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

impl OrgStoreError {
    /// Stable value for the `reason` log field.
    pub fn reason(&self) -> &'static str {
        match self {
            OrgStoreError::NoSuchOrg => "no_such_org",
            OrgStoreError::NotAMember => "not_a_member",
            OrgStoreError::AlreadyMember => "already_member",
            OrgStoreError::AlreadyInType => "already_in_type",
            OrgStoreError::NameTaken => "name_taken",
            OrgStoreError::RankNotInType(_) => "rank_not_in_type",
            OrgStoreError::LeaderPinned => "leader_pinned",
            OrgStoreError::NeedsLeader => "needs_leader",
            OrgStoreError::InvalidText(r) => r.reason(),
            OrgStoreError::VaultNotEmpty => "vault_not_empty",
            OrgStoreError::NotPersistent => "not_persistent",
            OrgStoreError::NoSuchPlayer => "no_such_player",
            OrgStoreError::Db(_) => "db_error",
        }
    }
}

impl From<TextReject> for OrgStoreError {
    fn from(r: TextReject) -> Self {
        OrgStoreError::InvalidText(r)
    }
}
