//! Finding the second player of an invite, kick or rank change.
//!
//! - An **invitee** must be online: [`online_target`] resolves the typed
//!   name against the online index (D-SS13: exact, then a unique
//!   case-insensitive match), the way tells and duel challenges do.
//! - A **kicked or re-ranked member** may be offline: [`member_by_name`]
//!   resolves the name among that organization's own member rows only,
//!   inside the caller's ORG-LOCK transaction.

use std::time::Instant;

use cimmeria_entity::organization::OrgRank;
use sqlx::{Postgres, Transaction};

use super::fanout::{online_members, OnlineMember};
use super::telemetry::OrgReject;
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::rank_from_db;
use crate::base::organization::persistence::OrgStoreError;
use crate::base::player_index::{NameLookup, OnlinePlayerIndex};
use crate::base::ConnectedClientState;

/// An online invitee, read under one lock of the connected-client map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineTarget {
    pub player_id: i32,
    pub entity_id: u32,
    pub account_id: u32,
    /// The character's own name, as the session holds it.
    pub name: String,
}

/// Resolve `typed` to an online character other than `actor` whose client
/// is ready and who does not ignore `actor`.
///
/// Refusals: [`OrgReject::TargetNotFound`], [`OrgReject::TargetAmbiguous`],
/// [`OrgReject::SelfTarget`], [`OrgReject::TargetInTransition`] (mid world
/// entry or gate travel) and [`OrgReject::Ignored`]. The ignore check reads
/// the target's cached Ignore list (SS-C1) by the actor's character id and
/// by the actor's session name, never by anything the client typed. On a
/// refusal after the name resolved, the target is returned with it so the
/// outcome row can name them.
pub fn online_target(
    ctx: &OrgCtx<'_>,
    actor: &OrgPlayer,
    actor_name: &str,
    typed: &str,
) -> Result<OnlineTarget, (OrgReject, Option<OnlineTarget>)> {
    let Ok(clients) = ctx.connected.lock() else {
        return Err((OrgReject::TargetNotFound, None));
    };
    let found = match OnlinePlayerIndex::new(&clients).lookup(typed) {
        NameLookup::Found(found) => found,
        NameLookup::Ambiguous => return Err((OrgReject::TargetAmbiguous, None)),
        NameLookup::NotFound => return Err((OrgReject::TargetNotFound, None)),
    };
    if found.player_id == actor.player_id {
        return Err((OrgReject::SelfTarget, None));
    }
    let Some(c) = clients.get(&found.addr) else {
        return Err((OrgReject::TargetNotFound, None));
    };
    let target = OnlineTarget {
        player_id: found.player_id,
        entity_id: c.player_entity_id.unwrap_or_default(),
        account_id: c.account_id,
        name: c.player_name.clone().unwrap_or_default(),
    };
    if c.player_entity_id.is_none() || !is_client_ready(c) {
        return Err((OrgReject::TargetInTransition, Some(target)));
    }
    if c.ignore.ignores_player(actor.player_id) || c.ignore.ignores(actor_name) {
        return Err((OrgReject::Ignored, Some(target)));
    }
    Ok(target)
}

/// The session's character is in the world and its client has created the
/// player entity (the duel challenge's rule): listed online and no world
/// entry step outstanding. Gate travel keeps the listing but sets the
/// pending steps, so a traveller mid-load is not ready.
fn is_client_ready(c: &ConnectedClientState) -> bool {
    c.listed_online
        && c.pending_world_entry.is_none()
        && c.pending_map_loaded.is_none()
        && c.pending_client_ready.is_none()
}

/// The actor's character name, from their own session.
pub fn actor_name(ctx: &OrgCtx<'_>, actor: &OrgPlayer) -> Option<String> {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()?
        .get(&actor.entity_id)
        .copied()?;
    let clients = ctx.connected.lock().ok()?;
    clients
        .get(&addr)
        .filter(|c| c.active_player_id == Some(actor.player_id))
        .and_then(|c| c.player_name.clone())
}

/// Run `f` on the actor's own session state, if the session still plays
/// that character.
pub fn with_actor_session<R>(
    ctx: &OrgCtx<'_>,
    actor: &OrgPlayer,
    f: impl FnOnce(&mut ConnectedClientState) -> R,
) -> Option<R> {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()?
        .get(&actor.entity_id)
        .copied()?;
    let mut clients = ctx.connected.lock().ok()?;
    clients
        .get_mut(&addr)
        .filter(|c| c.active_player_id == Some(actor.player_id))
        .map(f)
}

/// The online session of `player_id` now, re-resolved by character id (an
/// entity id read before a transaction may have been recycled since).
pub fn online_now(ctx: &OrgCtx<'_>, player_id: i32) -> Option<OnlineMember> {
    online_members(ctx, &[player_id]).into_iter().next()
}

/// Whether the actor may send another invite now (the inviter's rate
/// limit), read from their own session. Takes and drops the lock, so the
/// caller holds none across its `.await`s.
pub fn actor_may_send(ctx: &OrgCtx<'_>, actor: &OrgPlayer, now: Instant) -> bool {
    with_actor_session(ctx, actor, |c| c.org_invites.may_send(now)).unwrap_or(false)
}

/// One member of an organization, found by name among its member rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberTarget {
    pub player_id: i32,
    pub account_id: i32,
    pub name: String,
    pub rank: OrgRank,
}

/// Resolve `typed` among `org_id`'s members: an exact name first, then a
/// unique case-insensitive match (D-SS13, the rule every name lookup on the
/// server uses). Runs inside the caller's transaction, after its lock.
///
/// `Ok(Err(TargetNotMember | TargetAmbiguous))` for a typed miss.
pub async fn member_by_name(
    tx: &mut Transaction<'_, Postgres>,
    org_id: i32,
    typed: &str,
) -> Result<Result<MemberTarget, OrgReject>, OrgStoreError> {
    let rows: Vec<(i32, i32, String, i16)> = sqlx::query_as(
        "SELECT m.player_id, m.account_id, p.player_name, m.rank \
         FROM sgw_organization_members m \
         JOIN sgw_player p ON p.player_id = m.player_id \
         WHERE m.org_id = $1 AND lower(p.player_name) = lower($2)",
    )
    .bind(org_id)
    .bind(typed)
    .fetch_all(&mut **tx)
    .await?;
    let exact: Vec<_> = rows.iter().filter(|r| r.2 == typed).collect();
    let pick = match (exact.as_slice(), rows.as_slice()) {
        ([one], _) => *one,
        (_, [one]) => one,
        (_, []) => return Ok(Err(OrgReject::TargetNotMember)),
        _ => return Ok(Err(OrgReject::TargetAmbiguous)),
    };
    Ok(Ok(MemberTarget {
        player_id: pick.0,
        account_id: pick.1,
        name: pick.2.clone(),
        rank: rank_from_db(pick.3)?,
    }))
}
