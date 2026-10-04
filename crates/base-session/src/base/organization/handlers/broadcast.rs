//! `broadcast_to_org`: one client method to every online member of a Team
//! or Command (ORG-API, work-packets.md § "Bank campaign API").
//!
//! Call it **after the commit**: it reads the roster and the rank table
//! with no lock (display reads), so it sees what the transaction wrote.
//! "Online" is the connected-client view ([`online_members`]), resolved by
//! character id at send time, never an entity id read earlier.

use cimmeria_entity::organization::OrgPermission;

use super::fanout::{online_members, send_to_members};
use super::OrgCtx;
use crate::base::organization::persistence::{load_ranks, load_roster};

/// Send client method `method_idx` with `args` to every online member of
/// `org_id`, or, when `required` is set, only to those whose rank holds
/// every bit of it (for example `VIEW_BANK_LOGS` for the bank log).
/// Returns how many members it reached.
///
/// Logs DEBUG `org.broadcast` with `recipients` (sent), `online_members`
/// and the `required` mask; each failed send is WARN `org.send_failed` with
/// `reason` ([`send_to_members`]); a roster that could not be read is WARN
/// `org.broadcast_failed` with `reason` (`no_db`, `db_error`), and nothing
/// is sent.
pub async fn broadcast_to_org(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    method_idx: u16,
    args: &[u8],
    required: Option<OrgPermission>,
) -> usize {
    broadcast_except(ctx, org_id, method_idx, args, required, None, "broadcast").await
}

/// [`broadcast_to_org`] to everyone but `except`, the actor of a change
/// who was sent its own, fuller update (the Bank's vault fan-out: the
/// mover's client gets its bag and the vault rows, the other members only
/// the vault rows). `what` names the fanout in the logs.
pub async fn broadcast_to_org_except(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    method_idx: u16,
    args: &[u8],
    required: Option<OrgPermission>,
    except: i32,
    what: &'static str,
) -> usize {
    broadcast_except(ctx, org_id, method_idx, args, required, Some(except), what).await
}

/// [`broadcast_to_org`] minus one character (`except`), for a fanout whose
/// subject was told separately (a new member gets the full state push, not
/// the [37] about themself). `what` names the fanout in the logs.
pub(super) async fn broadcast_except(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    method_idx: u16,
    args: &[u8],
    required: Option<OrgPermission>,
    except: Option<i32>,
    what: &'static str,
) -> usize {
    let failed = |reason: &'static str, error: Option<String>| {
        tracing::warn!(
            target: "org",
            event = "org.broadcast_failed",
            what,
            org_id,
            method_index = method_idx,
            method_name = cimmeria_wire::names::player_client_method(method_idx),
            reason,
            error,
            "organization broadcast not sent: the roster could not be read"
        );
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        failed("no_db", None);
        return 0;
    };
    let roster = match load_roster(pool, org_id).await {
        Ok(r) => r,
        Err(e) => {
            failed(e.reason(), Some(e.to_string()));
            return 0;
        }
    };
    let allowed: Vec<i32> = match required {
        None => roster.iter().map(|m| m.player_id).collect(),
        Some(bits) => {
            let ranks = match load_ranks(pool, org_id).await {
                Ok(r) => r,
                Err(e) => {
                    failed(e.reason(), Some(e.to_string()));
                    return 0;
                }
            };
            roster
                .iter()
                .filter(|m| {
                    ranks
                        .iter()
                        .find(|r| r.rank == m.rank)
                        .is_some_and(|r| r.permissions.contains(bits))
                })
                .map(|m| m.player_id)
                .collect()
        }
    };
    let ids: Vec<i32> = allowed.into_iter().filter(|&p| Some(p) != except).collect();
    let recipients = online_members(ctx, &ids);
    let sent = send_to_members(
        ctx,
        org_id,
        &recipients,
        &[(method_idx, args.to_vec())],
        what,
    )
    .await;
    tracing::debug!(
        target: "org",
        event = "org.broadcast",
        what,
        org_id,
        method_index = method_idx,
        method_name = cimmeria_wire::names::player_client_method(method_idx),
        required = required.map(|r| r.bits()),
        roster_size = roster.len(),
        online_members = recipients.len(),
        recipients = sent,
        "organization broadcast sent"
    );
    sent
}
