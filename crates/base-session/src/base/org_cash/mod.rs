//! Team and Command treasury deposits and withdrawals (bank-vault BV-08;
//! D-BV12, D-BV15, D-BV19).
//!
//! The client's vault window sends `organizationTransferCash` (CM 19) with a
//! signed amount: `Team.lua` / `Command.lua` pass `cashAmt` for a deposit and
//! `-cashAmt` for a withdrawal, and never zero (`if cashAmt > 0`). The cell
//! decodes it into a `CashDir` and forwards it as `OrgCellToBase::TransferCash`
//! (ORG-01, ORG-API); a zero amount is refused on the cell
//! (`cimmeria-cell-methods` `organization::forward::zero_cash`).
//!
//! [`handle_transfer_cash`]:
//!
//! 1. re-checks that the forward still names this session's character in
//!    the world (`resolve_actor`);
//! 2. runs [`persist::transfer_cash`]: the actor's row `FOR KEY SHARE`, the
//!    organization lock, the membership and the `DepositCash` or
//!    `WithdrawCash` bit read under it, then the guarded wallet and treasury
//!    `UPDATE`s and a `sgw_organization_cash_log` row, all in one
//!    transaction. Neither balance goes negative or overflows: the wallet
//!    is an `i32`, the treasury an `i64` with `CHECK (cash >= 0)`;
//! 3. after the commit, sends every online member the new treasury
//!    (`onOrganizationCashUpdate`, `broadcast_to_org`), and the actor the
//!    new wallet (`onCashChanged`) and a chat line.
//!
//! Every refusal sends the actor a chat line saying why (except when the
//! actor has no session to send to). A refusal on either balance also
//! resends the balance the client got wrong, so a stale spinner maximum
//! corrects itself.
//!
//! There is no Banker check: the treasury is not the vault, and the client
//! offers the buttons only in the vault window, but the server does not
//! know which window sent the call. Membership and the bits are the whole
//! authorization, as for every other organization call.
//!
//! Telemetry (target `bank`): INFO `org_cash_transfer` with both balances
//! before and after; WARN `org_cash_rejected` with a stable `reason`. Both
//! carry `account_id`, `player_id`, `entity_id` and `org_id`.

pub mod persist;
mod sends;

#[cfg(test)]
mod tests;

use cimmeria_entity::organization::CashDir;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_cash_update, ON_ORGANIZATION_CASH_UPDATE,
};

use super::organization::api::broadcast_to_org;
use super::organization::handlers::{resolve_actor, OrgCtx, OrgPlayer};
use persist::{transfer_cash, CashDirection, Seen, TransferOutcome, Transferred};
use sends::Actor;

/// Why a transfer moved nothing. [`CashRefusal::reason`] is the stable
/// `reason=` of `org_cash_rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CashRefusal {
    /// The forward no longer names this session's character in the world.
    ActorMismatch,
    /// The server has no database.
    DbUnavailable,
    /// A statement failed; the transaction rolled back.
    QueryFailed,
    /// No `sgw_player` row for the actor.
    PlayerMissing,
    /// The organization is gone (disbanded) or never existed.
    NoSuchOrg,
    /// The actor is not a member.
    NotAMember,
    /// The actor's rank lacks `DepositCash` (a deposit) or `WithdrawCash`.
    NoPermission,
    /// A deposit larger than the wallet.
    InsufficientPlayerCash,
    /// A withdrawal larger than the treasury.
    InsufficientOrgCash,
    /// A withdrawal that would take the wallet past `i32::MAX`.
    PlayerCashOverflow,
    /// A deposit that would take the treasury past `i64::MAX`.
    OrgCashOverflow,
}

impl CashRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            CashRefusal::ActorMismatch => "actor_mismatch",
            CashRefusal::DbUnavailable => "db_unavailable",
            CashRefusal::QueryFailed => "query_failed",
            CashRefusal::PlayerMissing => "player_missing",
            CashRefusal::NoSuchOrg => "no_such_org",
            CashRefusal::NotAMember => "not_a_member",
            CashRefusal::NoPermission => "no_permission",
            CashRefusal::InsufficientPlayerCash => "insufficient_player_cash",
            CashRefusal::InsufficientOrgCash => "insufficient_org_cash",
            CashRefusal::PlayerCashOverflow => "player_cash_overflow",
            CashRefusal::OrgCashOverflow => "org_cash_overflow",
        }
    }

    /// The actor's chat line.
    fn feedback(self, direction: CashDirection, amount: i64, seen: &Seen) -> String {
        match self {
            CashRefusal::ActorMismatch
            | CashRefusal::DbUnavailable
            | CashRefusal::QueryFailed
            | CashRefusal::PlayerMissing => {
                "The transfer failed. No naquadah was moved.".to_string()
            }
            CashRefusal::NoSuchOrg => "That organization no longer exists.".to_string(),
            CashRefusal::NotAMember => "You are not a member of that organization.".to_string(),
            CashRefusal::NoPermission => match direction {
                CashDirection::Withdraw => "Your rank may not withdraw naquadah.".to_string(),
                _ => "Your rank may not deposit naquadah.".to_string(),
            },
            CashRefusal::InsufficientPlayerCash => format!(
                "You cannot deposit {amount} naquadah: you have {}.",
                seen.player_cash.unwrap_or(0)
            ),
            CashRefusal::InsufficientOrgCash => format!(
                "You cannot withdraw {amount} naquadah: the treasury holds {}.",
                seen.org_cash.unwrap_or(0)
            ),
            CashRefusal::PlayerCashOverflow => {
                format!("You cannot carry {amount} more naquadah.")
            }
            CashRefusal::OrgCashOverflow => {
                format!("The treasury cannot hold {amount} more naquadah.")
            }
        }
    }
}

/// `OrgCellToBase::TransferCash`: move naquadah between the actor's wallet
/// and a Team's or Command's treasury, or refuse with a reason.
#[tracing::instrument(
    name = "bank.org_cash_transfer",
    level = "info",
    skip_all,
    fields(entity_id = entity_id, player_id = player_id, org_id = org_id)
)]
pub async fn handle_transfer_cash(
    ctx: &OrgCtx<'_>,
    player_id: i32,
    entity_id: u32,
    org_id: i32,
    dir: CashDir,
) {
    let (direction, amount) = CashDirection::of(dir);
    let Some(player) = resolve_actor(ctx, player_id, entity_id) else {
        // No session plays that character as that entity, so there is no
        // one to tell.
        tracing::warn!(
            target: "bank",
            event = "org_cash_rejected",
            player_id,
            entity_id,
            org_id,
            direction = direction.as_str(),
            amount,
            reason = CashRefusal::ActorMismatch.reason(),
            "org_cash_rejected: the transfer no longer matches a session in the world -- \
             nothing moved, no one to tell"
        );
        return;
    };
    let actor = Actor { ctx, player };
    let Some(pool) = ctx.db_pool.as_deref() else {
        reject(
            &actor,
            org_id,
            direction,
            amount,
            CashRefusal::DbUnavailable,
            Seen::default(),
            None,
        )
        .await;
        return;
    };
    match transfer_cash(pool, player_id, org_id, dir).await {
        Err(e) => {
            let error = e.to_string();
            reject(
                &actor,
                org_id,
                direction,
                amount,
                CashRefusal::QueryFailed,
                Seen::default(),
                Some(&error),
            )
            .await;
        }
        Ok(TransferOutcome::Refused(refusal, seen)) => {
            reject(&actor, org_id, direction, amount, refusal, seen, None).await;
        }
        Ok(TransferOutcome::Done(t)) => {
            accepted(&actor, org_id, direction, amount, t).await;
        }
    }
}

async fn accepted(
    actor: &Actor<'_>,
    org_id: i32,
    direction: CashDirection,
    amount: i64,
    t: Transferred,
) {
    let args =
        build_on_organization_cash_update(org_id, u64::try_from(t.org_cash_after).unwrap_or(0));
    let recipients =
        broadcast_to_org(actor.ctx, org_id, ON_ORGANIZATION_CASH_UPDATE, &args, None).await;
    let p: OrgPlayer = actor.player;
    tracing::info!(
        target: "bank",
        event = "org_cash_transfer",
        account_id = t.account_id,
        player_id = p.player_id,
        entity_id = p.entity_id,
        org_id,
        org_type = t.org_type.name(),
        rank = t.rank.as_u8(),
        direction = direction.as_str(),
        amount,
        player_cash_before = t.player_cash_before,
        player_cash_after = t.player_cash_after,
        org_cash_before = t.org_cash_before,
        org_cash_after = t.org_cash_after,
        recipients,
        "org_cash_transfer: naquadah moved between a wallet and a treasury"
    );
    actor.send_cash(t.player_cash_after).await;
    let line = match direction {
        CashDirection::Withdraw => format!(
            "You withdrew {amount} naquadah from the {} treasury. It now holds {}.",
            t.org_type.name(),
            t.org_cash_after
        ),
        _ => format!(
            "You deposited {amount} naquadah into the {} treasury. It now holds {}.",
            t.org_type.name(),
            t.org_cash_after
        ),
    };
    actor.send_line(&line).await;
}

async fn reject(
    actor: &Actor<'_>,
    org_id: i32,
    direction: CashDirection,
    amount: i64,
    refusal: CashRefusal,
    seen: Seen,
    error: Option<&str>,
) {
    let p = actor.player;
    let perm = match (refusal, direction) {
        (CashRefusal::NoPermission, CashDirection::Withdraw) => Some("WithdrawCash"),
        (CashRefusal::NoPermission, _) => Some("DepositCash"),
        _ => None,
    };
    // Nothing moved, so each balance is the same before and after.
    tracing::warn!(
        target: "bank",
        event = "org_cash_rejected",
        account_id = p.account_id,
        player_id = p.player_id,
        entity_id = p.entity_id,
        org_id,
        org_type = seen.org_type.map(|t| t.name()),
        rank = seen.rank.map(|r| r.as_u8()),
        permissions = seen.permissions.map(|m| m.bits()),
        perm,
        direction = direction.as_str(),
        amount,
        player_cash_before = seen.player_cash,
        player_cash_after = seen.player_cash,
        org_cash_before = seen.org_cash,
        org_cash_after = seen.org_cash,
        reason = refusal.reason(),
        error,
        "org_cash_rejected: nothing moved -- the player sees a chat line saying why"
    );
    // Resend the balance the client was wrong about. The treasury only to
    // a member.
    match refusal {
        CashRefusal::InsufficientPlayerCash => {
            if let Some(cash) = seen.player_cash {
                actor.send_cash(cash).await;
            }
        }
        CashRefusal::InsufficientOrgCash if seen.member => {
            if let Some(cash) = seen.org_cash {
                actor.send_org_cash(org_id, cash).await;
            }
        }
        _ => {}
    }
    actor
        .send_line(&refusal.feedback(direction, amount, &seen))
        .await;
}
