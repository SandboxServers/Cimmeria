//! Presence: telling a Team's or Command's online members that one of them
//! came online or went offline.
//!
//! Both directions are `onMemberJoinedOrganization` [37] with `aNewMember =
//! 0`: the live entity id on login, 0 on logout. The client handler
//! (`0x00e4e4c0`, ORG-E1 Q1) finds the record by name and overwrites its id
//! in place whenever the new id differs, unregistering the old
//! entity-id lookup and registering the new one only when it is not 0, so
//! id 0 turns the row "Offline" and keeps it. `onMemberLeftOrganization`
//! [39] would delete the row, which is why logout never uses it.

use cimmeria_entity::organization::OrgType;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, ON_MEMBER_JOINED_ORGANIZATION,
};

use super::fanout::{online_members, send_to_members};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::persistence::{load_memberships, load_roster, RosterMember};

/// Send [37] for `player` to the other online members of the organization
/// whose roster is `roster`: their entity id when `online`, else 0. Logs one
/// INFO presence row, `event = member_online` / `member_offline`, with
/// `recipients` and the `disconnect_reason` of a logout.
pub(super) async fn announce(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    org_type: OrgType,
    roster: &[RosterMember],
    player: &OrgPlayer,
    online: bool,
    disconnect_reason: Option<&'static str>,
) -> usize {
    let event = if online {
        "member_online"
    } else {
        "member_offline"
    };
    let Some(me) = roster.iter().find(|m| m.player_id == player.player_id) else {
        // The roster was read after the membership: the player left (or was
        // removed) in between, so there is no row to update.
        tracing::warn!(
            target: "org",
            event = "org.presence_skipped",
            presence = event,
            account_id = player.account_id,
            player_id = player.player_id,
            org_id,
            reason = "not_in_roster",
            "presence not announced: the player is no longer on the roster"
        );
        return 0;
    };
    let others: Vec<i32> = roster
        .iter()
        .map(|m| m.player_id)
        .filter(|&p| p != player.player_id)
        .collect();
    let recipients = online_members(ctx, &others);
    let member_id = if online { player.entity_id as i32 } else { 0 };
    let args = build_on_member_joined_organization(&me.name, member_id, org_id, me.rank, false);
    let sent = send_to_members(
        ctx,
        org_id,
        &recipients,
        &[(ON_MEMBER_JOINED_ORGANIZATION, args)],
        event,
    )
    .await;
    tracing::info!(
        target: "org",
        event,
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        org_id,
        org_type = org_type.name(),
        member_id,
        recipients = sent,
        online_members = recipients.len(),
        disconnect_reason,
        "organization presence announced"
    );
    sent
}

/// Tell every organization the player belongs to that they went offline.
/// Called once per session end, from the teardown hook
/// (`base::session_presence`) and `logOff`.
///
/// Reads with no lock: presence is display. A failed read logs WARN
/// `org.presence_failed` with `reason`.
#[tracing::instrument(
    name = "org.presence",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id, disconnect_reason)
)]
pub async fn announce_offline(
    ctx: &OrgCtx<'_>,
    player: &OrgPlayer,
    disconnect_reason: &'static str,
) {
    let fail = |reason: &'static str, error: Option<String>| {
        tracing::warn!(
            target: "org",
            event = "org.presence_failed",
            presence = "member_offline",
            account_id = player.account_id,
            player_id = player.player_id,
            entity_id = player.entity_id,
            disconnect_reason,
            reason,
            error,
            "organization offline presence could not be announced"
        );
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        fail("no_db", None);
        return;
    };
    let memberships = match load_memberships(pool, player.player_id).await {
        Ok(m) => m,
        Err(e) => {
            fail(e.reason(), Some(e.to_string()));
            return;
        }
    };
    for m in &memberships {
        let org_id = m.header.org_id;
        match load_roster(pool, org_id).await {
            Ok(roster) => {
                announce(
                    ctx,
                    org_id,
                    m.header.org_type,
                    &roster,
                    player,
                    false,
                    Some(disconnect_reason),
                )
                .await;
            }
            Err(e) => fail(e.reason(), Some(e.to_string())),
        }
    }
}
