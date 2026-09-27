//! Founding a Team or Command (ORG-05): the base half of the registrar
//! flow and of the GM `.org_create`.
//!
//! The flow, cell and base together (`docs/gameplay/organization-system.md`
//! § "Creation"):
//!
//! 1. A right-click on a registrar reaches the cell's
//!    `try_open_org_registrar`, which forwards `OrgCellToBase::RegistrarOpen`
//!    with the type the registrar's seed data names.
//! 2. [`handler::handle_registrar_open`] checks D-ORG18 eligibility (a
//!    display read: the create re-checks under its own transaction) and
//!    answers `OrgBaseToCell::RegistrarEligible`, or refuses with a line.
//! 3. The cell records the pending creation and sends
//!    `launchOrganizationCreation` [135]. The client names the organization
//!    with cell method 94; the cell checks the pending creation and the
//!    D-ORG10 name, then forwards `OrgCellToBase::Create`.
//! 4. [`handler::handle_create`] runs [`found_organization`] and answers the
//!    client itself: 134 and ORG-06's `push_org_state` on success, or
//!    `onOrganizationCreationResult` [134] with a refusal code and a line.
//!    It then tells the cell (`OrgBaseToCell::CreateResult`) whether to close
//!    the pending creation or charge it an attempt.
//!
//! The creation cost is D-ORG15: a per-type constant, 0 for now, debited in
//! the same transaction as `create_org`, so a refused name costs nothing.

pub mod handler;
mod telemetry;

pub use telemetry::count_org_action;

use cimmeria_entity::organization::OrgType;
use sqlx::PgPool;

use super::persistence::{create_org, load_memberships, CreatedOrg, OrgStoreError};

/// Naquadah it costs to found a Team (D-ORG15: free for now; a price is a
/// one-line change here).
pub const ORG_CREATE_COST_TEAM: i32 = 0;
/// Naquadah it costs to found a Command (D-ORG15).
pub const ORG_CREATE_COST_COMMAND: i32 = 0;

/// The D-ORG15 cost of founding an organization of `org_type`. A Squad is
/// never founded here.
pub fn creation_cost(org_type: OrgType) -> i32 {
    match org_type {
        OrgType::Team => ORG_CREATE_COST_TEAM,
        OrgType::Command => ORG_CREATE_COST_COMMAND,
        OrgType::Squad => 0,
    }
}

/// A founded organization, with what the founder paid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Founded {
    pub org: CreatedOrg,
    /// The cost charged: 0 while D-ORG15 keeps creation free.
    pub cost: i32,
    /// The founder's naquadah before and after the debit, only when a cost
    /// was charged.
    pub cash: Option<(i32, i32)>,
}

/// Why [`found_organization`] founded nothing.
#[derive(Debug, thiserror::Error)]
pub enum FoundReject {
    /// `create_org` refused (name taken, invalid text, already in an
    /// organization of the type, ...).
    #[error(transparent)]
    Store(#[from] OrgStoreError),
    /// The founder has less naquadah than the cost. `have` is `None` when
    /// the character row could not be read back.
    #[error("insufficient funds: cost {cost}, have {have:?}")]
    InsufficientFunds { cost: i32, have: Option<i32> },
}

impl From<sqlx::Error> for FoundReject {
    fn from(e: sqlx::Error) -> Self {
        FoundReject::Store(OrgStoreError::Db(e))
    }
}

/// Found an organization of `org_type` named `name`, led by `player_id`,
/// charging the D-ORG15 cost. One transaction: the organization, its ranks
/// and its leader, then the debit; a refusal anywhere writes nothing.
pub async fn found_organization(
    pool: &PgPool,
    org_type: OrgType,
    name: &str,
    player_id: i32,
) -> Result<Founded, FoundReject> {
    found_organization_at_cost(pool, org_type, name, player_id, creation_cost(org_type)).await
}

/// [`found_organization`] at an explicit cost, so the debit path is tested
/// while the constants are 0.
pub(crate) async fn found_organization_at_cost(
    pool: &PgPool,
    org_type: OrgType,
    name: &str,
    player_id: i32,
    cost: i32,
) -> Result<Founded, FoundReject> {
    let mut tx = pool.begin().await?;
    if cost > 0 {
        // Refuse a short purse before `create_org` draws an organization id
        // from its NO CYCLE sequence, as it does for a taken name. Only a
        // pre-check: the debit below is what enforces the cost.
        let have: Option<i32> =
            sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .fetch_optional(&mut *tx)
                .await?;
        if have.is_some_and(|h| h < cost) {
            return Err(FoundReject::InsufficientFunds { cost, have });
        }
    }
    // ORG-LOCK order: `create_org` inserts (and so locks) the organization
    // row first; the debit then takes the founder's `sgw_player` row.
    let org = create_org(&mut tx, org_type, name, player_id).await?;
    let cash = if cost > 0 {
        // A plain UPDATE (FOR NO KEY UPDATE), never `SELECT ... FOR UPDATE`:
        // `insert_member` holds this row FOR KEY SHARE, which NO KEY UPDATE
        // is compatible with and FOR UPDATE is not.
        let after: Option<i32> = sqlx::query_scalar(
            "UPDATE sgw_player SET naquadah = naquadah - $1 \
             WHERE player_id = $2 AND naquadah >= $1 RETURNING naquadah",
        )
        .bind(cost)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(after) = after else {
            let have: Option<i32> =
                sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
                    .bind(player_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            // Dropping `tx` rolls the organization back with the refusal.
            return Err(FoundReject::InsufficientFunds { cost, have });
        };
        Some((after + cost, after))
    } else {
        None
    };
    tx.commit().await?;
    Ok(Founded { org, cost, cash })
}

/// Whether `player_id` may found an organization of `org_type` (D-ORG18:
/// not already in one of that type). A display read for the registrar's
/// dialog; [`found_organization`] enforces the rule in its transaction.
pub async fn is_eligible(
    pool: &PgPool,
    player_id: i32,
    org_type: OrgType,
) -> Result<bool, OrgStoreError> {
    let memberships = load_memberships(pool, player_id).await?;
    Ok(!memberships.iter().any(|m| m.header.org_type == org_type))
}

#[cfg(test)]
mod tests;
