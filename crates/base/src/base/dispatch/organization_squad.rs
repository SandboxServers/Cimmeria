//! The squad forms of 0xD0 and 0xD1, forwarded to the cell (ORG-03), and
//! the Ignore check the base makes before a squad invite leaves it
//! (ORG-07).
//!
//! Squads live on the cell (D-ORG03), but Ignore lists are base-side
//! session state (SS-C1's `IgnoreCache`), so the base checks whether the
//! invitee ignores the inviter before forwarding `SquadInvite`: the same
//! rule, and the same line, as a Team or Command invite. Everything else
//! about the invitee (online, ambiguous, self, already squadded) stays the
//! cell's check.

use std::net::SocketAddr;

use cimmeria_base_session::base::organization::handlers::answer::IGNORED_TEXT;
use cimmeria_base_session::base::organization::handlers::fanout::feedback;
use cimmeria_base_session::base::organization::handlers::{OrgCtx, OrgPlayer};
use cimmeria_base_session::base::player_index::{NameLookup, OnlinePlayerIndex};
use cimmeria_wire::base::organization::OrgBaseCall;
use cimmeria_wire::cell::client_methods::organization::ORG_NOT_AVAILABLE_TEXT;

use crate::cell::messages::{BaseToCellMsg, OrgBaseToCell};

use super::organization::{answer, answer_instance};

/// The squad message a call forwards to the cell. The caller has routed it
/// here: 0xD0 with type 0, or 0xD1 with a squad-range org id.
fn squad_message(call: &OrgBaseCall, player: &OrgPlayer) -> OrgBaseToCell {
    match call {
        OrgBaseCall::Kick {
            org_id,
            player_name,
        } => OrgBaseToCell::SquadKick {
            player_id: player.player_id,
            entity_id: player.entity_id,
            org_id: *org_id,
            target_name: player_name.clone(),
        },
        OrgBaseCall::InviteByType { player_name, .. }
        | OrgBaseCall::Invite { player_name, .. }
        | OrgBaseCall::RankChange { player_name, .. } => OrgBaseToCell::SquadInvite {
            player_id: player.player_id,
            entity_id: player.entity_id,
            target_name: player_name.clone(),
        },
    }
}

/// Forward a squad invite or kick to the cell, after the Ignore check for
/// an invite.
pub(super) async fn forward_squad_call(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    addr: SocketAddr,
    call: &OrgBaseCall,
    target_name: &str,
) {
    let fwd = squad_message(call, player);
    if matches!(fwd, OrgBaseToCell::SquadInvite { .. }) {
        if let Some(target_player_id) = invitee_ignores_inviter(ctx, player, target_name) {
            // One outcome row on `squad`, since the cell never sees it.
            tracing::info!(
                target: "squad",
                event = "squad.invite",
                outcome = "rejected",
                reason = "ignored",
                account_id = player.account_id,
                player_id = player.player_id,
                entity_id = player.entity_id,
                target_player_id,
                "squad action rejected"
            );
            cimmeria_observability::counter!(
                "squad_actions_total",
                "action" => "invite",
                "outcome" => "rejected",
                "reason" => "ignored",
            );
            feedback(ctx, player.entity_id, IGNORED_TEXT).await;
            return;
        }
    }
    let kind = fwd.kind();
    let forwarded = match ctx.cell_tx {
        Some(tx) => tx.send(BaseToCellMsg::Org(fwd)).await.is_ok(),
        None => false,
    };
    if forwarded {
        tracing::debug!(
            target: "org",
            event = "org.squad_forwarded",
            %addr,
            account_id = player.account_id,
            player_id = player.player_id,
            entity_id = player.entity_id,
            kind,
            "squad call forwarded to the cell"
        );
        return;
    }
    tracing::warn!(
        target: "org",
        event = "org.squad_forward_failed",
        %addr,
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        kind,
        reason = "cell_unreachable",
        "squad call could not reach the cell -- answering with feedback"
    );
    unreachable_outcome(kind, player);
    answer(
        answer_instance(call),
        ORG_NOT_AVAILABLE_TEXT,
        player.entity_id,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}

/// The invitee's `player_id` when `typed` resolves to one online character
/// whose cached Ignore list holds the inviter (by character id, or by the
/// inviter's own session name). Any other lookup result forwards, and the
/// cell answers it.
fn invitee_ignores_inviter(ctx: &OrgCtx<'_>, player: &OrgPlayer, typed: &str) -> Option<i32> {
    let clients = ctx.connected.lock().ok()?;
    let inviter_name = clients
        .values()
        .find(|c| {
            c.active_player_id == Some(player.player_id)
                && c.player_entity_id == Some(player.entity_id)
        })
        .and_then(|c| c.player_name.clone())
        .unwrap_or_default();
    let NameLookup::Found(target) = OnlinePlayerIndex::new(&clients).lookup(typed) else {
        return None;
    };
    let t = clients.get(&target.addr)?;
    let ignores = t.ignore.ignores_player(player.player_id)
        || (!inviter_name.is_empty() && t.ignore.ignores(&inviter_name));
    ignores.then_some(target.player_id)
}

/// The squad action never reached the cell, so the cell logs no outcome
/// row for it and counts nothing: this is that row and that count, in the
/// shape of the cell's `Outcome::emit` (`squad/telemetry.rs`) and on the
/// same `squad_actions_total{action, outcome, reason}` series as
/// `cimmeria_cell_world::cell::squad::count_action`, which the base cannot
/// call (it does not depend on the cell crates).
fn unreachable_outcome(kind: &str, player: &OrgPlayer) {
    let (event, action) = if kind == "squad_kick" {
        ("squad.kick", "kick")
    } else {
        ("squad.invite", "invite")
    };
    tracing::info!(
        target: "squad",
        event,
        outcome = "rejected",
        reason = "cell_unreachable",
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        "squad action rejected"
    );
    cimmeria_observability::counter!(
        "squad_actions_total",
        "action" => action,
        "outcome" => "rejected",
        "reason" => "cell_unreachable",
    );
}
