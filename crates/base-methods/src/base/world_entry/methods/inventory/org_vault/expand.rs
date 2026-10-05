//! Buying Team vault space from the Team treasury (bank-vault BV-09,
//! D-BV14, D-BV28): the base half of GM `.orgvaultexpand`.
//!
//! The Team vault (container 19) starts at 40 slots and grows in +10 steps
//! to 100. A step costs the D-BV02 price (`resources.bank_expansion_price`,
//! 100 naquadah) and is paid from the Team's treasury
//! (`sgw_organizations.cash`), not from anyone's wallet. Only the Team's
//! leader may buy it; there is no permission bit for it. The Command vault
//! (20) is fixed at 100 and is refused.
//!
//! There is no client UI: the Expand dialog is quarantined (#943), so the
//! trigger is the GM console. Without a size the command only quotes; with
//! the current size it buys, keyed on that size, so a repeated command is a
//! `replay` and charges nothing.
//!
//! One transaction, in the order every vault action takes
//! ([`super::access::lock_actor`]): the buyer's `sgw_player` row `FOR KEY
//! SHARE`, then the organization lock and the buyer's rank under it. Then
//! one grow-only `UPDATE` raises `vault_slots` and debits `cash` together,
//! only while the vault is still at the size the GM named, below 100, with
//! a price row for the step and the treasury to pay it; then the
//! `sgw_organization_cash_log` row (`direction = vault_expansion`); then
//! the commit.
//!
//! After the commit the buyer gets `onBagInfo` with the Team vault at its
//! new size and a line, and every online member the new treasury
//! (`onOrganizationCashUpdate`). Telemetry (target `bank`): INFO `expand`
//! and INFO `org_cash_transfer` (`direction = vault_expansion`) for a
//! purchase, DEBUG `expand_quote` for a quote, WARN `expand_rejected` with a
//! stable `reason` for every refusal, each with a line.

use cimmeria_base_session::base::org_cash::persist::{insert_cash_log, CashDirection, CashLogRow};
use cimmeria_base_session::base::org_cash::sends::Actor;
use cimmeria_base_session::base::organization::api::broadcast_to_org;
use cimmeria_base_session::base::organization::handlers::{OrgCtx, OrgPlayer};
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::known_names;
use cimmeria_entity::organization::{OrgRank, OrgType};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_cash_update, ON_ORGANIZATION_CASH_UPDATE,
};
use sqlx::PgPool;

use super::access::{lock_actor, OrgLockMiss};
use super::open::{org_label, org_vault_bag_info};
use crate::mercury::method_idx;

/// The largest Team vault (D-BV14).
const TEAM_VAULT_CEILING: i16 = 100;
/// One step.
const STEP: i16 = 10;

/// One `.orgvaultexpand`, as the cell forwarded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrgVaultExpandRequest {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: i32,
    pub scope: VaultScope,
    /// The size the GM is buying from; `None` asks for a quote.
    pub from_slots: Option<i16>,
}

/// Why nothing was bought. [`ExpandRefusal::reason`] is the stable `reason`
/// of `expand_rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpandRefusal {
    DbUnavailable,
    QueryFailed,
    /// The buyer is in no organization of that type.
    NotInOrg,
    /// The lock-time check: `player_missing`, `no_such_org`, `not_a_member`,
    /// `wrong_org_type`.
    Lock(OrgLockMiss),
    /// The Command vault is fixed at 100.
    CommandVaultFixed,
    /// The buyer is not the Team's leader (D-BV28).
    NotLeader,
    /// The vault is at 100.
    AtCeiling,
    /// No price row (or a zero price) for the step: never free.
    PriceMissing,
    /// The vault is no longer at the size the GM named: already bought.
    Replay,
    /// The treasury holds less than the price.
    InsufficientOrgCash,
    /// Every check held, yet the keyed `UPDATE` matched nothing.
    RowChanged,
}

impl ExpandRefusal {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            ExpandRefusal::DbUnavailable => "db_unavailable",
            ExpandRefusal::QueryFailed => "query_failed",
            ExpandRefusal::NotInOrg => "not_in_org",
            ExpandRefusal::Lock(miss) => miss.reason(),
            ExpandRefusal::CommandVaultFixed => "command_vault_fixed",
            ExpandRefusal::NotLeader => "not_leader",
            ExpandRefusal::AtCeiling => "at_ceiling",
            ExpandRefusal::PriceMissing => "price_missing",
            ExpandRefusal::Replay => "replay",
            ExpandRefusal::InsufficientOrgCash => "insufficient_org_cash",
            ExpandRefusal::RowChanged => "row_changed",
        }
    }

    fn feedback(self, s: &Snapshot, scope: VaultScope) -> String {
        let slots = s.vault_slots.unwrap_or(0);
        let price = s.price.unwrap_or(0);
        let cash = s.org_cash.unwrap_or(0);
        let tail = "Nothing was charged.";
        match self {
            ExpandRefusal::DbUnavailable
            | ExpandRefusal::QueryFailed
            | ExpandRefusal::Lock(OrgLockMiss::PlayerMissing)
            | ExpandRefusal::RowChanged => {
                format!("orgvaultexpand: the purchase failed. {tail}")
            }
            ExpandRefusal::NotInOrg
            | ExpandRefusal::Lock(
                OrgLockMiss::NotAMember | OrgLockMiss::NoSuchOrg | OrgLockMiss::WrongOrgType,
            ) => format!(
                "orgvaultexpand: you are not in a {}. {tail}",
                org_label(scope)
            ),
            ExpandRefusal::CommandVaultFixed => {
                format!("orgvaultexpand: a Command vault is fixed at 100 slots. {tail}")
            }
            ExpandRefusal::NotLeader => {
                format!("orgvaultexpand: only the Team's leader may expand its vault. {tail}")
            }
            ExpandRefusal::AtCeiling => {
                format!("orgvaultexpand: the Team vault already has 100 slots. {tail}")
            }
            ExpandRefusal::PriceMissing => {
                format!("orgvaultexpand: the next step has no price. {tail}")
            }
            ExpandRefusal::Replay => format!(
                "orgvaultexpand: the Team vault has {slots} slots, not {}. {tail}",
                s.from_slots.unwrap_or(0)
            ),
            ExpandRefusal::InsufficientOrgCash => format!(
                "orgvaultexpand: the next step costs {price}; the treasury holds {cash}. {tail}"
            ),
        }
    }
}

/// What the handler had read when it stopped, for the log and the line.
#[derive(Debug, Clone, Copy, Default)]
struct Snapshot {
    account_id: Option<i32>,
    org_id: Option<i32>,
    org_type: Option<OrgType>,
    rank: Option<OrgRank>,
    vault_slots: Option<i16>,
    from_slots: Option<i16>,
    price: Option<i32>,
    org_cash: Option<i64>,
}

/// A committed step.
#[derive(Debug, Clone, Copy)]
struct Bought {
    account_id: i32,
    org_id: i32,
    rank: OrgRank,
    bank_slots: i16,
    from: i16,
    to: i16,
    price: i64,
    org_cash_before: i64,
    org_cash_after: i64,
}

enum Outcome {
    Quote(Snapshot),
    Bought(Bought),
}

/// `BankCellToBase::OrgVaultExpand`: quote or buy one Team vault step, or
/// refuse with a reason.
#[tracing::instrument(
    name = "bank.org_vault_expand",
    level = "info",
    skip_all,
    fields(entity_id = req.entity_id, player_id = req.player_id, scope = req.scope.as_str())
)]
pub async fn handle_org_vault_expand(req: OrgVaultExpandRequest, ctx: &OrgCtx<'_>) {
    let actor = Actor {
        ctx,
        player: OrgPlayer {
            account_id: req.account_id,
            player_id: req.player_id,
            entity_id: req.entity_id,
        },
    };
    let seen = Snapshot {
        from_slots: req.from_slots,
        ..Snapshot::default()
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        reject(&actor, &req, ExpandRefusal::DbUnavailable, seen, None).await;
        return;
    };
    match expand(pool, &req, seen).await {
        Err(e) => {
            let error = e.to_string();
            reject(&actor, &req, ExpandRefusal::QueryFailed, seen, Some(&error)).await;
        }
        Ok(Err((refusal, seen))) => reject(&actor, &req, refusal, seen, None).await,
        Ok(Ok(Outcome::Quote(s))) => quoted(&actor, &req, s).await,
        Ok(Ok(Outcome::Bought(b))) => bought(&actor, &req, b).await,
    }
}

type Refused = (ExpandRefusal, Snapshot);

/// The locked part. The outer `Err` is a database failure; the transaction
/// rolls back when it drops.
async fn expand(
    pool: &PgPool,
    req: &OrgVaultExpandRequest,
    mut seen: Snapshot,
) -> Result<Result<Outcome, Refused>, sqlx::Error> {
    let org_type = req.scope.org_type().map_or(0, |t| i16::from(t.as_u8()));
    // Unlocked: it only picks which organization to lock.
    let org_id: Option<i32> = sqlx::query_scalar(
        "SELECT org_id FROM sgw_organization_members WHERE player_id = $1 AND org_type = $2",
    )
    .bind(req.player_id)
    .bind(org_type)
    .fetch_optional(pool)
    .await?;
    let Some(org_id) = org_id else {
        return Ok(Err((ExpandRefusal::NotInOrg, seen)));
    };
    seen.org_id = Some(org_id);

    let mut tx = pool.begin().await?;
    let actor = match lock_actor(&mut tx, req.player_id, org_id, req.scope).await? {
        Ok(actor) => actor,
        Err(miss) => return Ok(Err((ExpandRefusal::Lock(miss), seen))),
    };
    seen.account_id = Some(actor.account_id);
    seen.org_type = Some(actor.access.org_type());
    seen.rank = Some(actor.access.rank());
    // The row is locked, so these are exact until the commit.
    let (vault_slots, cash): (i16, i64) =
        sqlx::query_as("SELECT vault_slots, cash FROM sgw_organizations WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&mut *tx)
            .await?;
    seen.org_cash = Some(cash);
    if req.scope != VaultScope::Team {
        return Ok(Err((ExpandRefusal::CommandVaultFixed, seen)));
    }
    seen.vault_slots = Some(vault_slots);
    if actor.access.rank() != OrgRank::LEADER {
        return Ok(Err((ExpandRefusal::NotLeader, seen)));
    }
    if vault_slots >= TEAM_VAULT_CEILING {
        return Ok(Err((ExpandRefusal::AtCeiling, seen)));
    }
    let price: Option<i32> = sqlx::query_scalar(
        "SELECT price_naquadah FROM resources.bank_expansion_price WHERE to_slots = $1",
    )
    .bind(vault_slots + STEP)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(price) = price.filter(|p| *p > 0) else {
        return Ok(Err((ExpandRefusal::PriceMissing, seen)));
    };
    seen.price = Some(price);
    let Some(from) = req.from_slots else {
        return Ok(Ok(Outcome::Quote(seen)));
    };
    if from != vault_slots {
        return Ok(Err((ExpandRefusal::Replay, seen)));
    }
    if cash < i64::from(price) {
        return Ok(Err((ExpandRefusal::InsufficientOrgCash, seen)));
    }

    // Grow-only and keyed on the size and the price: the checks above are
    // exact under the lock, and the statement holds them again.
    let row: Option<(i16, i64)> = sqlx::query_as(
        "UPDATE sgw_organizations o \
            SET vault_slots = o.vault_slots + 10, cash = o.cash - x.price_naquadah \
           FROM resources.bank_expansion_price x \
          WHERE o.org_id = $1 AND o.org_type = 1 AND o.vault_slots = $2 \
            AND o.vault_slots < 100 AND x.to_slots = o.vault_slots + 10 \
            AND x.price_naquadah = $3 AND o.cash >= x.price_naquadah \
         RETURNING o.vault_slots, o.cash",
    )
    .bind(org_id)
    .bind(from)
    .bind(price)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((to, cash_after)) = row else {
        return Ok(Err((ExpandRefusal::RowChanged, seen)));
    };
    let price = i64::from(price);
    let cash_before = cash_after + price;
    insert_cash_log(
        &mut tx,
        &CashLogRow {
            org_id,
            org_type: actor.access.org_type(),
            account_id: actor.account_id,
            player_id: req.player_id,
            rank: actor.access.rank(),
            direction: CashDirection::VaultExpansion,
            amount: price,
            player_cash: None,
            org_cash: (cash_before, cash_after),
            vault_slots: Some((from, to)),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Ok(Outcome::Bought(Bought {
        account_id: actor.account_id,
        org_id,
        rank: actor.access.rank(),
        bank_slots: actor.bank_slots,
        from,
        to,
        price,
        org_cash_before: cash_before,
        org_cash_after: cash_after,
    })))
}

async fn quoted(actor: &Actor<'_>, req: &OrgVaultExpandRequest, s: Snapshot) {
    let player_label = known_names::player_name(req.player_id);
    tracing::debug!(
        target: "bank",
        event = "expand_quote",
        offered = true,
        account_id = s.account_id,
        account_name = known_names::account_name(s.account_id),
        player_id = req.player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        scope = req.scope.as_str(),
        org_id = s.org_id,
        org_name = known_names::org_name(s.org_id),
        vault_slots = s.vault_slots,
        price = s.price,
        org_cash = s.org_cash,
        trigger = "gm_console",
        "expand_quote: the Team vault's next step, paid from the treasury"
    );
    let slots = s.vault_slots.unwrap_or(0);
    actor
        .send_line(&format!(
            "orgvaultexpand: the Team vault has {slots} slots. The next +10 costs {} from the \
             treasury, which holds {}. Type .orgvaultexpand {slots} to buy it.",
            s.price.unwrap_or(0),
            s.org_cash.unwrap_or(0)
        ))
        .await;
}

async fn bought(actor: &Actor<'_>, req: &OrgVaultExpandRequest, b: Bought) {
    let args =
        build_on_organization_cash_update(b.org_id, u64::try_from(b.org_cash_after).unwrap_or(0));
    let recipients = broadcast_to_org(
        actor.ctx,
        b.org_id,
        ON_ORGANIZATION_CASH_UPDATE,
        &args,
        None,
    )
    .await;
    let player_label = known_names::player_name(req.player_id);
    tracing::info!(
        target: "bank",
        event = "expand",
        account_id = b.account_id,
        account_name = known_names::account_name(b.account_id),
        player_id = req.player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        scope = req.scope.as_str(),
        org_id = b.org_id,
        org_name = known_names::org_name(b.org_id),
        org_type = OrgType::Team.name(),
        rank = b.rank.as_u8(),
        vault_slots_before = b.from,
        vault_slots_after = b.to,
        price = b.price,
        org_cash_before = b.org_cash_before,
        org_cash_after = b.org_cash_after,
        gm_override = true,
        trigger = "gm_console",
        "expand: the Team vault grew one step, paid from the treasury"
    );
    let player_label = known_names::player_name(req.player_id);
    tracing::info!(
        target: "bank",
        event = "org_cash_transfer",
        account_id = b.account_id,
        account_name = known_names::account_name(b.account_id),
        player_id = req.player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        org_id = b.org_id,
        org_name = known_names::org_name(b.org_id),
        org_type = OrgType::Team.name(),
        rank = b.rank.as_u8(),
        direction = CashDirection::VaultExpansion.as_str(),
        amount = b.price,
        org_cash_before = b.org_cash_before,
        org_cash_after = b.org_cash_after,
        vault_slots_before = b.from,
        vault_slots_after = b.to,
        recipients,
        "org_cash_transfer: the treasury paid for a Team vault step"
    );
    let bag_info = org_vault_bag_info(i32::from(b.bank_slots), i32::from(b.to));
    actor
        .send("bag_info", method_idx::ON_BAG_INFO, &bag_info)
        .await;
    actor
        .send_line(&format!(
            "orgvaultexpand: the Team vault now has {} slots. The treasury paid {} and holds {}.",
            b.to, b.price, b.org_cash_after
        ))
        .await;
}

async fn reject(
    actor: &Actor<'_>,
    req: &OrgVaultExpandRequest,
    refusal: ExpandRefusal,
    s: Snapshot,
    error: Option<&str>,
) {
    let account_id = s
        .account_id
        .or(req.account_id.and_then(|a| i32::try_from(a).ok()));
    let player_label = known_names::player_name(req.player_id);
    tracing::warn!(
        target: "bank",
        event = "expand_rejected",
        account_id,
        account_name = known_names::account_name(account_id),
        player_id = req.player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        scope = req.scope.as_str(),
        org_id = s.org_id,
        org_name = known_names::org_name(s.org_id),
        org_type = s.org_type.map(OrgType::name),
        rank = s.rank.map(OrgRank::as_u8),
        vault_slots = s.vault_slots,
        offered_slots = s.from_slots,
        price = s.price,
        org_cash = s.org_cash,
        gm_override = true,
        trigger = "gm_console",
        reason = refusal.reason(),
        error,
        "expand_rejected: nothing bought -- the GM sees a chat line saying why"
    );
    actor.send_line(&refusal.feedback(&s, req.scope)).await;
}
