//! Team and Command console commands: `.org_disband <orgId>` (ORG-06),
//! `.org_join <orgId> [player]` and `.org_rank <player> <rank> [orgId]`
//! (ORG-07).
//!
//! `.org_join` and `.org_rank` go the same way as `.org_disband`
//! (`OrgCellToBase::GmJoin`, `GmRank`): the base re-reads the access level,
//! skips the member permission and rank checks, keeps the type and
//! one-per-type rules, and writes the `org.gm_join` / `org.gm_rank` row.
//!
//! The cell has no database, so the command is forwarded to the base
//! (`OrgCellToBase::GmDisband`), which re-reads the GM's access level from
//! its own session (D-ORG13), honours the vault check (D-ORG20), fans
//! `onOrganizationLeft(Disbanded)` out to the online members and answers the
//! GM on the feedback channel. The base writes the `org.disband` outcome
//! row; the only refusals decided here are the ones the base cannot see (a
//! caller with no character, a malformed id, a dead base channel), each an
//! `org.disband` row too. Those three are not counted on
//! `org_actions_total`: the console crate has no metrics dependency, and a
//! GM typo is not an organization action.

use tokio::sync::mpsc;

use super::parse::parse_i32;
use super::send_gm_feedback;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;

/// `.org_disband <orgId>`: hand the disband to the base.
#[tracing::instrument(name = "org.disband", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn disband(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    let refused = |reason: &'static str, org_id: Option<i32>| {
        tracing::info!(
            target: "org",
            event = "org.disband",
            outcome = "rejected",
            reason,
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id = caller_id,
            org_id,
            "organization action rejected"
        );
    };
    let Some(org_id) = parse_i32(caller_id, args, 0, "orgId", tx).await else {
        refused("org_id_invalid", None);
        return;
    };
    let Some(player_id) = id.player_id else {
        refused("caller_not_player", Some(org_id));
        send_gm_feedback(caller_id, "org_disband: you have no character id", tx).await;
        return;
    };
    let msg = CellToBaseMsg::Org(OrgCellToBase::GmDisband {
        player_id,
        entity_id: caller_id,
        org_id,
    });
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "org",
            event = "org.forward_failed",
            account_id = id.account_id,
            player_id,
            entity_id = caller_id,
            org_id,
            reason = "cell_to_base_closed",
            "org_disband could not reach the base"
        );
        refused("cell_to_base_closed", Some(org_id));
    }
}

/// The GM's character id, or the `caller_not_player` refusal (row and
/// line) under `event`.
async fn gm_player_id(
    caller_id: u32,
    event: &'static str,
    command: &'static str,
    org_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<i32> {
    let id = space_mgr.player_identity(caller_id);
    if id.player_id.is_none() {
        refused_row(event, "caller_not_player", caller_id, space_mgr, org_id);
        send_gm_feedback(
            caller_id,
            &format!("{command}: you have no character id"),
            tx,
        )
        .await;
    }
    id.player_id
}

/// One console-side refusal row (the base never saw the command).
fn refused_row(
    event: &'static str,
    reason: &'static str,
    caller_id: u32,
    space_mgr: &SpaceManager,
    org_id: Option<i32>,
) {
    let id = space_mgr.player_identity(caller_id);
    tracing::info!(
        target: "org",
        event,
        outcome = "rejected",
        reason,
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id = caller_id,
        org_id,
        "organization action rejected"
    );
}

/// Hand `msg` to the base; a closed channel is WARN `org.forward_failed`
/// plus the refusal row.
async fn to_base(
    msg: OrgCellToBase,
    event: &'static str,
    caller_id: u32,
    org_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let kind = msg.kind();
    if tx.send(CellToBaseMsg::Org(msg)).await.is_err() {
        let id = space_mgr.player_identity(caller_id);
        tracing::warn!(
            target: "org",
            event = "org.forward_failed",
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id = caller_id,
            org_id,
            kind,
            reason = "cell_to_base_closed",
            "GM organization command could not reach the base"
        );
        refused_row(event, "cell_to_base_closed", caller_id, space_mgr, org_id);
    }
}

/// `.org_join <orgId> [player]`: add `player` (default: the caller) to the
/// organization. The player must be online.
#[tracing::instrument(name = "org.gm_join", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn join(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(org_id) = parse_i32(caller_id, args, 0, "orgId", tx).await else {
        refused_row("org.gm_join", "org_id_invalid", caller_id, space_mgr, None);
        return;
    };
    let Some(player_id) = gm_player_id(
        caller_id,
        "org.gm_join",
        "org_join",
        Some(org_id),
        tx,
        space_mgr,
    )
    .await
    else {
        return;
    };
    let msg = OrgCellToBase::GmJoin {
        player_id,
        entity_id: caller_id,
        org_id,
        target_name: args.get(1).map(|s| (*s).to_owned()),
    };
    to_base(msg, "org.gm_join", caller_id, Some(org_id), tx, space_mgr).await;
}

/// `.org_rank <player> <rank> [orgId]`: set a member's rank (1-7; `Leader`
/// is never assigned). Without `orgId` the member must be in only one Team
/// or Command.
#[tracing::instrument(name = "org.gm_rank", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn rank(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(target_name) = args.first().map(|s| (*s).to_owned()) else {
        refused_row("org.gm_rank", "usage", caller_id, space_mgr, None);
        send_gm_feedback(caller_id, "usage: .org_rank <player> <rank> [orgId]", tx).await;
        return;
    };
    let Some(rank) = args.get(1).and_then(|s| s.parse::<u8>().ok()) else {
        refused_row("org.gm_rank", "rank_invalid", caller_id, space_mgr, None);
        send_gm_feedback(caller_id, "org_rank: rank must be 0-255", tx).await;
        return;
    };
    let org_id = match args.get(2) {
        None => None,
        Some(_) => match parse_i32(caller_id, args, 2, "orgId", tx).await {
            Some(id) => Some(id),
            None => {
                refused_row("org.gm_rank", "org_id_invalid", caller_id, space_mgr, None);
                return;
            }
        },
    };
    let Some(player_id) =
        gm_player_id(caller_id, "org.gm_rank", "org_rank", org_id, tx, space_mgr).await
    else {
        return;
    };
    let msg = OrgCellToBase::GmRank {
        player_id,
        entity_id: caller_id,
        target_name,
        rank,
        org_id,
    };
    to_base(msg, "org.gm_rank", caller_id, org_id, tx, space_mgr).await;
}
