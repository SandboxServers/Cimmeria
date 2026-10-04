//! Team and Command console commands: `.org_disband <orgId>` (ORG-06),
//! `.org_join <orgId> [player]` and `.org_rank <player> <rank> [orgId]`
//! (ORG-07), and `.org_info [player]`, `.org_list` and
//! `.org_set_perms <orgId> <rank> <mask>` (ORG-10).
//!
//! **Audit row (D-ORG13, ORG-10).** Every GM organization command, refused
//! or not, ends in exactly one INFO `org.gm_action` row with `action`, the
//! GM's identity and the result. A refusal decided here (a malformed
//! argument, a caller with no character, a dead base channel) writes it
//! here, beside the command's own row where it has one (`org.disband`,
//! `org.gm_join`, `org.gm_rank`); everything after the forward is the
//! base's. The ORG-10 commands have no row of their own: `org.gm_action` is
//! their outcome row.
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
        refused_row(DISBAND, reason, caller_id, space_mgr, org_id);
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
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            org_id, // nt:id-only organization names live on the base, not the cell
            reason = "cell_to_base_closed",
            "org_disband could not reach the base",
        );
        refused("cell_to_base_closed", Some(org_id));
    }
}

/// One GM organization command's names: its own outcome `event`, and the
/// `action` of its `org.gm_action` audit row (the base's
/// `org_actions_total` label for the same command).
#[derive(Debug, Clone, Copy)]
pub(super) struct GmCommand {
    pub event: &'static str,
    pub action: &'static str,
}

/// The `event` of the audit row every GM organization command writes.
pub(super) const GM_ACTION: &str = "org.gm_action";

const DISBAND: GmCommand = GmCommand {
    event: "org.disband",
    action: "disband",
};
const JOIN: GmCommand = GmCommand {
    event: "org.gm_join",
    action: "gm_org_join",
};
const RANK: GmCommand = GmCommand {
    event: "org.gm_rank",
    action: "gm_org_rank",
};
const INFO: GmCommand = GmCommand {
    event: GM_ACTION,
    action: "gm_org_info",
};
const LIST: GmCommand = GmCommand {
    event: GM_ACTION,
    action: "gm_org_list",
};
const SET_PERMS: GmCommand = GmCommand {
    event: GM_ACTION,
    action: "gm_org_set_perms",
};

/// The GM's character id, or the `caller_not_player` refusal (row and
/// line) under `event`.
async fn gm_player_id(
    caller_id: u32,
    event: GmCommand,
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

/// One console-side refusal (the base never saw the command): the
/// command's own row, then its `org.gm_action` audit twin when the command
/// has an event of its own.
fn refused_row(
    cmd: GmCommand,
    reason: &'static str,
    caller_id: u32,
    space_mgr: &SpaceManager,
    org_id: Option<i32>,
) {
    let id = space_mgr.player_identity(caller_id);
    let events: &[&'static str] = if cmd.event == GM_ACTION {
        &[GM_ACTION]
    } else {
        &[cmd.event, GM_ACTION]
    };
    for &event in events {
        tracing::info!(
            target: "org",
            event,
            action = cmd.action,
            outcome = "rejected",
            reason,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            org_id, // nt:id-only organization names live on the base, not the cell
            "organization action rejected",
        );
    }
}

/// Hand `msg` to the base; a closed channel is WARN `org.forward_failed`
/// plus the refusal row.
async fn to_base(
    msg: OrgCellToBase,
    event: GmCommand,
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
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            org_id, // nt:id-only organization names live on the base, not the cell
            kind,
            reason = "cell_to_base_closed",
            "GM organization command could not reach the base",
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
        refused_row(JOIN, "org_id_invalid", caller_id, space_mgr, None);
        return;
    };
    let Some(player_id) =
        gm_player_id(caller_id, JOIN, "org_join", Some(org_id), tx, space_mgr).await
    else {
        return;
    };
    let msg = OrgCellToBase::GmJoin {
        player_id,
        entity_id: caller_id,
        org_id,
        target_name: args.get(1).map(|s| (*s).to_owned()),
    };
    to_base(msg, JOIN, caller_id, Some(org_id), tx, space_mgr).await;
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
        refused_row(RANK, "usage", caller_id, space_mgr, None);
        send_gm_feedback(caller_id, "usage: .org_rank <player> <rank> [orgId]", tx).await;
        return;
    };
    let Some(rank) = args.get(1).and_then(|s| s.parse::<u8>().ok()) else {
        refused_row(RANK, "rank_invalid", caller_id, space_mgr, None);
        send_gm_feedback(caller_id, "org_rank: rank must be 0-255", tx).await;
        return;
    };
    let org_id = match args.get(2) {
        None => None,
        Some(_) => match parse_i32(caller_id, args, 2, "orgId", tx).await {
            Some(id) => Some(id),
            None => {
                refused_row(RANK, "org_id_invalid", caller_id, space_mgr, None);
                return;
            }
        },
    };
    let Some(player_id) = gm_player_id(caller_id, RANK, "org_rank", org_id, tx, space_mgr).await
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
    to_base(msg, RANK, caller_id, org_id, tx, space_mgr).await;
}

/// `.org_info [player]`: every Team and Command `player` (default: the
/// caller) belongs to, with the rank and its permission mask. The base
/// answers; the player may be offline.
#[tracing::instrument(name = "org.gm_info", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn info(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = gm_player_id(caller_id, INFO, "org_info", None, tx, space_mgr).await
    else {
        return;
    };
    let msg = OrgCellToBase::GmInfo {
        player_id,
        entity_id: caller_id,
        target_name: args.first().map(|s| (*s).to_owned()),
    };
    to_base(msg, INFO, caller_id, None, tx, space_mgr).await;
}

/// `.org_list`: every Team and Command, from the base.
#[tracing::instrument(name = "org.gm_list", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn list(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = gm_player_id(caller_id, LIST, "org_list", None, tx, space_mgr).await
    else {
        return;
    };
    let msg = OrgCellToBase::GmList {
        player_id,
        entity_id: caller_id,
    };
    to_base(msg, LIST, caller_id, None, tx, space_mgr).await;
}

/// A permission mask as the GM types it: decimal, or hex with `0x`.
pub(super) fn parse_mask(s: &str) -> Option<u32> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => s.parse().ok(),
    }
}

/// `.org_set_perms <orgId> <rank> <mask>`: set a rank's permission mask.
/// The base clamps the mask to the bits the type's rank editor shows
/// (D-ORG09 (6)) and refuses the `Leader` row.
#[tracing::instrument(name = "org.gm_set_perms", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn set_perms(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(org_id) = parse_i32(caller_id, args, 0, "orgId", tx).await else {
        refused_row(SET_PERMS, "org_id_invalid", caller_id, space_mgr, None);
        return;
    };
    let Some(rank) = args.get(1).and_then(|s| s.parse::<u8>().ok()) else {
        refused_row(
            SET_PERMS,
            "rank_invalid",
            caller_id,
            space_mgr,
            Some(org_id),
        );
        send_gm_feedback(caller_id, "org_set_perms: rank must be 0-255", tx).await;
        return;
    };
    let Some(mask) = args.get(2).copied().and_then(parse_mask) else {
        refused_row(
            SET_PERMS,
            "mask_invalid",
            caller_id,
            space_mgr,
            Some(org_id),
        );
        send_gm_feedback(
            caller_id,
            "org_set_perms: mask must be a number, decimal or 0x-prefixed hex",
            tx,
        )
        .await;
        return;
    };
    let Some(player_id) = gm_player_id(
        caller_id,
        SET_PERMS,
        "org_set_perms",
        Some(org_id),
        tx,
        space_mgr,
    )
    .await
    else {
        return;
    };
    let msg = OrgCellToBase::GmSetPerms {
        player_id,
        entity_id: caller_id,
        org_id,
        rank,
        mask,
    };
    to_base(msg, SET_PERMS, caller_id, Some(org_id), tx, space_mgr).await;
}
