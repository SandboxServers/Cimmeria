//! The organization state a member's client needs: sent at every world
//! entry for each Team and Command the player belongs to (login restore),
//! and by ORG-05 after a creation.
//!
//! **Order (ORG-E1 Q1).** `onOrganizationJoined` [35] first, so the client
//! has the organization; then its name [43], MOTD [45], cash [48],
//! experience [44], rank permissions [49] and custom rank names [50]; then
//! the roster [38], which the client stores with every member id 0
//! ("Offline", handler `0x00e4ea50`); then `onMemberJoinedOrganization`
//! [37] with `aNewMember = 0` and the live entity id for every online
//! member, which is the only message that sets a roster id (handler
//! `0x00e4e4c0` overwrites the id of an existing name in place).

use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, build_on_organization_cash_update,
    build_on_organization_experience_update, build_on_organization_joined,
    build_on_organization_motd_update, build_on_organization_name_update,
    build_on_organization_rank_name_update, build_on_organization_rank_update,
    build_on_organization_roster_info, RosterInfo, ON_MEMBER_JOINED_ORGANIZATION,
    ON_ORGANIZATION_CASH_UPDATE, ON_ORGANIZATION_EXPERIENCE_UPDATE, ON_ORGANIZATION_JOINED,
    ON_ORGANIZATION_MOTD_UPDATE, ON_ORGANIZATION_NAME_UPDATE, ON_ORGANIZATION_RANK_NAME_UPDATE,
    ON_ORGANIZATION_RANK_UPDATE, ON_ORGANIZATION_ROSTER_INFO,
};

use super::fanout::{online_members, send_to_player, OnlineMember};
use super::presence::announce;
use super::telemetry::{count, OrgReject};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::persistence::{
    load_memberships, load_ranks, load_roster, OrgMembership, OrgStoreError, RankRow, RosterMember,
};

/// What one organization's push sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushSummary {
    pub org_id: i32,
    pub roster_size: usize,
    /// Online members the push marked Online, the player included.
    pub online_members: usize,
}

/// Why a push did not go out.
#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("the server has no database")]
    NoDb,
    #[error("the player is not a member of that organization")]
    NotMember,
    #[error(transparent)]
    Store(#[from] OrgStoreError),
    #[error("the bundle was not sent: {0}")]
    Send(&'static str),
}

impl PushError {
    fn reason(&self) -> &'static str {
        match self {
            PushError::NoDb => "no_db",
            PushError::NotMember => "not_member",
            PushError::Store(e) => e.reason(),
            PushError::Send(reason) => reason,
        }
    }
}

/// A database width to a wire byte. The columns hold levels and archetype
/// ids far below 255; a hand-edited row saturates rather than wraps.
fn byte(v: i32) -> u8 {
    u8::try_from(v.max(0)).unwrap_or(u8::MAX)
}

/// The messages of one organization's push, in the ORG-E1 Q1 order. Pure,
/// so the byte order is pinned without a database.
pub fn org_state_messages(
    membership: &OrgMembership,
    ranks: &[RankRow],
    roster: &[RosterMember],
    online: &[OnlineMember],
    new_member: bool,
) -> Vec<(u16, Vec<u8>)> {
    let h = &membership.header;
    let org_id = h.org_id;
    let rank_masks: Vec<_> = ranks.iter().map(|r| (r.rank, r.permissions)).collect();
    // Only renamed ranks: a rank with no stored name keeps the client's
    // default label.
    let rank_names: Vec<_> = ranks
        .iter()
        .filter_map(|r| r.name.as_deref().map(|n| (r.rank, n)))
        .collect();
    let roster_info: Vec<RosterInfo> = roster
        .iter()
        .map(|m| RosterInfo {
            name: m.name.clone(),
            level: byte(m.level),
            archetype: byte(m.archetype),
            rank: m.rank,
            note: m.note.clone(),
            officer_note: m.officer_note.clone(),
        })
        .collect();

    let mut out = vec![
        (
            ON_ORGANIZATION_JOINED,
            build_on_organization_joined(org_id, h.org_type, membership.rank, new_member),
        ),
        (
            ON_ORGANIZATION_NAME_UPDATE,
            build_on_organization_name_update(org_id, &h.name),
        ),
        (
            ON_ORGANIZATION_MOTD_UPDATE,
            build_on_organization_motd_update(org_id, &h.motd),
        ),
        (
            ON_ORGANIZATION_CASH_UPDATE,
            build_on_organization_cash_update(org_id, u64::try_from(h.cash).unwrap_or(0)),
        ),
        (
            ON_ORGANIZATION_EXPERIENCE_UPDATE,
            build_on_organization_experience_update(
                org_id,
                u64::try_from(h.experience).unwrap_or(0),
            ),
        ),
        (
            ON_ORGANIZATION_RANK_UPDATE,
            build_on_organization_rank_update(org_id, &rank_masks),
        ),
        (
            ON_ORGANIZATION_RANK_NAME_UPDATE,
            build_on_organization_rank_name_update(org_id, &rank_names),
        ),
        (
            ON_ORGANIZATION_ROSTER_INFO,
            build_on_organization_roster_info(org_id, &roster_info),
        ),
    ];
    for o in online {
        let Some(m) = roster.iter().find(|m| m.player_id == o.player_id) else {
            continue;
        };
        out.push((
            ON_MEMBER_JOINED_ORGANIZATION,
            build_on_member_joined_organization(&m.name, o.entity_id as i32, org_id, m.rank, false),
        ));
    }
    out
}

/// Send `org_id`'s state to `player`'s client (ORG-05 calls it after a
/// creation with `new_member = true`; login restore with `false`).
///
/// Reads are display reads with no lock. Logs INFO `org.state_push` with
/// `roster_size` and `online_members` on success, WARN
/// `org.state_push_failed` with `reason` otherwise.
pub async fn push_org_state(
    ctx: &OrgCtx<'_>,
    org_id: i32,
    player: &OrgPlayer,
    new_member: bool,
) -> Result<PushSummary, PushError> {
    let result = async {
        let pool = ctx.db_pool.as_deref().ok_or(PushError::NoDb)?;
        let membership = load_memberships(pool, player.player_id)
            .await?
            .into_iter()
            .find(|m| m.header.org_id == org_id)
            .ok_or(PushError::NotMember)?;
        push_membership(ctx, &membership, player, new_member, "push")
            .await
            .map(|(summary, _)| summary)
    }
    .await;
    if let Err(e) = &result {
        state_push_failed(org_id, player, e);
    }
    result
}

/// One organization's push, from a membership already read. Returns the
/// summary and the roster it read, for the presence fanout.
async fn push_membership(
    ctx: &OrgCtx<'_>,
    membership: &OrgMembership,
    player: &OrgPlayer,
    new_member: bool,
    source: &'static str,
) -> Result<(PushSummary, Vec<RosterMember>), PushError> {
    let pool = ctx.db_pool.as_deref().ok_or(PushError::NoDb)?;
    let org_id = membership.header.org_id;
    let ranks = load_ranks(pool, org_id).await?;
    let roster = load_roster(pool, org_id).await?;
    let ids: Vec<i32> = roster.iter().map(|m| m.player_id).collect();
    let mut online = online_members(ctx, &ids);
    // The player is in the world by now even if the session is not yet
    // listed (the push runs on world entry): mark them Online too.
    if !online.iter().any(|o| o.player_id == player.player_id) {
        online.push(OnlineMember {
            player_id: player.player_id,
            entity_id: player.entity_id,
            account_id: player.account_id,
        });
        online.sort_unstable_by_key(|o| o.player_id);
    }
    let messages = org_state_messages(membership, &ranks, &roster, &online, new_member);
    send_to_player(ctx, player.entity_id, &messages)
        .await
        .map_err(PushError::Send)?;
    let summary = PushSummary {
        org_id,
        roster_size: roster.len(),
        online_members: online.len(),
    };
    tracing::info!(
        target: "org",
        event = "org.state_push",
        source,
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        org_id,
        org_type = membership.header.org_type.name(),
        rank = membership.rank.as_u8(),
        roster_size = summary.roster_size,
        online_members = summary.online_members,
        messages = messages.len(),
        new_member,
        "organization state pushed to a member"
    );
    Ok((summary, roster))
}

fn state_push_failed(org_id: i32, player: &OrgPlayer, e: &PushError) {
    tracing::warn!(
        target: "org",
        event = "org.state_push_failed",
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        org_id,
        reason = e.reason(),
        error = %e,
        "organization state could not be pushed to a member"
    );
}

/// Login restore (ORG-06): on every world entry, push each Team and Command
/// the player belongs to, then tell that organization's other online
/// members the player is online ([37] with the player's entity id). Gate
/// travel re-runs it, so a new entity id reaches the other members' rosters.
///
/// Ends in one INFO `org.login_restore` row with `org_count` (`outcome =
/// rejected`, `reason = no_db` / `db_error`, when the memberships could not
/// be read); each organization adds an `org.state_push` row and a
/// `member_online` presence row.
#[tracing::instrument(
    name = "org.login_restore",
    level = "info",
    skip_all,
    fields(player_id = player.player_id, entity_id = player.entity_id)
)]
pub async fn restore_on_login(ctx: &OrgCtx<'_>, player: &OrgPlayer) -> Vec<PushSummary> {
    let fail = |why: OrgReject, error: Option<&dyn std::fmt::Display>| {
        tracing::info!(
            target: "org",
            event = "org.login_restore",
            outcome = "rejected",
            reason = why.reason(),
            account_id = player.account_id,
            player_id = player.player_id,
            entity_id = player.entity_id,
            error = error.map(tracing::field::display),
            "organization login restore failed"
        );
        count("login_restore", "rejected", why.reason());
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        fail(OrgReject::NoDb, None);
        return Vec::new();
    };
    let memberships = match load_memberships(pool, player.player_id).await {
        Ok(m) => m,
        Err(e) => {
            fail(OrgReject::DbError, Some(&e));
            return Vec::new();
        }
    };
    let mut pushed = Vec::with_capacity(memberships.len());
    for m in &memberships {
        match push_membership(ctx, m, player, false, "login_restore").await {
            Ok((summary, roster)) => {
                announce(
                    ctx,
                    m.header.org_id,
                    m.header.org_type,
                    &roster,
                    player,
                    true,
                    None,
                )
                .await;
                pushed.push(summary);
            }
            Err(e) => state_push_failed(m.header.org_id, player, &e),
        }
    }
    tracing::info!(
        target: "org",
        event = "org.login_restore",
        outcome = "ok",
        account_id = player.account_id,
        player_id = player.player_id,
        entity_id = player.entity_id,
        org_count = memberships.len(),
        pushed = pushed.len(),
        "organization state restored at world entry"
    );
    count("login_restore", "ok", "none");
    pushed
}
