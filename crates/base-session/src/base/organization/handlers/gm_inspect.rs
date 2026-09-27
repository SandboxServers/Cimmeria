//! GM inspection and re-push (ORG-10, D-ORG13): `.org_info [player]`,
//! `.org_list` and `gmReloadOrganizations` (`SGWGmPlayer` cell method 164).
//!
//! The cell forwards each with the GM's own character; the base resolves
//! that session by character **and** entity id and re-reads its access
//! level ([`super::gm::gm_session`]). The answers are feedback lines to the
//! GM. None of the three changes the database; the reads are display reads
//! with no lock.
//!
//! Each command ends in exactly one INFO `org.gm_action` row (`action` =
//! `gm_org_info` \| `gm_org_list` \| `gm_reload_organizations`) with the
//! GM's identity, the target's where there is one, `count` (memberships,
//! organizations or organizations re-sent) and the result, and counts once
//! on `org_actions_total`.

use cimmeria_entity::organization::OrgType;

use super::answer::{db_failed, refuse};
use super::disband::{GmCaller, GM_ACCESS_LEVEL};
use super::fanout::feedback;
use super::gm::gm_session;
use super::push::push_org_state;
use super::telemetry::{ActionRow, OrgReject, GM_ACTION_EVENT};
use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::org_type_from_db;
use crate::base::organization::persistence::load_memberships;

/// How many organizations `.org_list` prints before it stops: the feedback
/// channel is a chat window, not a report.
pub const ORG_LIST_MAX: usize = 50;

/// One character `.org_info` names, from `sgw_player`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Character {
    player_id: i32,
    account_id: i32,
    name: String,
}

/// The GM's session and access level, or the `not_gm` refusal.
fn authorize(ctx: &OrgCtx<'_>, gm: GmCaller, row: &mut ActionRow) -> Option<OrgPlayer> {
    let (caller, level) = gm_session(ctx, gm)?;
    row.account_id = caller.account_id;
    (level >= GM_ACCESS_LEVEL).then_some(caller)
}

/// `.org_info [player]`: every Team and Command `target_name` (default: the
/// GM) belongs to, with the rank and the rank's permission mask. The
/// character may be offline: the name is matched in `sgw_player`, exactly
/// first, then case-insensitively when that is unique.
#[tracing::instrument(
    name = "org.gm_info",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id)
)]
pub async fn gm_info(
    ctx: &OrgCtx<'_>,
    gm: GmCaller,
    target_name: Option<&str>,
) -> Result<usize, OrgReject> {
    let mut row = ActionRow {
        event: GM_ACTION_EVENT,
        action: "gm_org_info",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, text: String| async move {
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    let Some(caller) = authorize(ctx, gm, &mut row) else {
        return fail(
            row,
            OrgReject::NotGm,
            "org_info: refused, GameMaster access is required.".into(),
        )
        .await;
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(
            row,
            OrgReject::NoDb,
            "org_info: failed, no database.".into(),
        )
        .await;
    };
    let lookup = match target_name {
        None => character_by_id(pool, caller.player_id).await,
        Some(name) => character_by_name(pool, name).await,
    };
    let target = match lookup {
        Ok(Ok(c)) => c,
        Ok(Err(why)) => {
            let text = match why {
                OrgReject::TargetAmbiguous => {
                    "org_info: more than one character matches that name; type it exactly."
                }
                _ => "org_info: no character has that name.",
            };
            return fail(row, why, text.into()).await;
        }
        Err(e) => {
            let why = db_failed(&row, &e);
            return fail(row, why, "org_info: failed, database error.".into()).await;
        }
    };
    row.target_player_id = Some(target.player_id);
    row.target_account_id = u32::try_from(target.account_id).ok();
    let memberships = match load_memberships(pool, target.player_id).await {
        Ok(m) => m,
        Err(e) => {
            let why = db_failed(&row, &e);
            return fail(row, why, "org_info: failed, database error.".into()).await;
        }
    };
    row.count = Some(memberships.len());
    feedback(
        ctx,
        gm.entity_id,
        &format!(
            "org_info: {} (character {}): {} organization(s).",
            target.name,
            target.player_id,
            memberships.len()
        ),
    )
    .await;
    for m in &memberships {
        feedback(
            ctx,
            gm.entity_id,
            &format!(
                "  {} '{}' (id {}): rank {}, permissions {:#09x}.",
                type_label(m.header.org_type),
                m.header.name,
                m.header.org_id,
                m.rank.as_u8(),
                m.display_permissions.bits()
            ),
        )
        .await;
    }
    row.ok("listed");
    Ok(memberships.len())
}

/// `.org_list`: every Team and Command, oldest first, with its member count
/// and leader; at most [`ORG_LIST_MAX`] lines.
#[tracing::instrument(
    name = "org.gm_list",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id)
)]
pub async fn gm_list(ctx: &OrgCtx<'_>, gm: GmCaller) -> Result<usize, OrgReject> {
    let mut row = ActionRow {
        event: GM_ACTION_EVENT,
        action: "gm_org_list",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, text: String| async move {
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    if authorize(ctx, gm, &mut row).is_none() {
        return fail(
            row,
            OrgReject::NotGm,
            "org_list: refused, GameMaster access is required.".into(),
        )
        .await;
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(
            row,
            OrgReject::NoDb,
            "org_list: failed, no database.".into(),
        )
        .await;
    };
    // One more than the cap tells "exactly 50" from "more than 50".
    type Listed = (i32, i16, String, i64, Option<String>);
    let rows: Vec<Listed> = match sqlx::query_as(
        "SELECT o.org_id, o.org_type, o.name, \
                (SELECT count(*) FROM sgw_organization_members m WHERE m.org_id = o.org_id), \
                (SELECT p.player_name FROM sgw_organization_members m \
                   JOIN sgw_player p ON p.player_id = m.player_id \
                  WHERE m.org_id = o.org_id AND m.rank = 8) \
         FROM sgw_organizations o ORDER BY o.org_id LIMIT $1",
    )
    .bind(ORG_LIST_MAX as i64 + 1)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            let why = db_failed(&row, &e);
            return fail(row, why, "org_list: failed, database error.".into()).await;
        }
    };
    let more = rows.len() > ORG_LIST_MAX;
    let shown = &rows[..rows.len().min(ORG_LIST_MAX)];
    row.count = Some(shown.len());
    let header = if shown.is_empty() {
        "org_list: there are no Teams or Commands.".to_owned()
    } else {
        format!("org_list: {} organization(s):", shown.len())
    };
    feedback(ctx, gm.entity_id, &header).await;
    for (org_id, org_type, name, members, leader) in shown {
        let kind = org_type_from_db(*org_type).map_or("Organization", type_label);
        feedback(
            ctx,
            gm.entity_id,
            &format!(
                "  {kind} '{name}' (id {org_id}): {members} member(s), leader {}.",
                leader.as_deref().unwrap_or("none")
            ),
        )
        .await;
    }
    if more {
        feedback(
            ctx,
            gm.entity_id,
            &format!("  ... more than {ORG_LIST_MAX}; the list stops here."),
        )
        .await;
    }
    row.ok("listed");
    Ok(shown.len())
}

/// `gmReloadOrganizations` (cell method 164): re-send the GM's own state for
/// every Team and Command they belong to, the same bundle as the world-entry
/// push ([`push_org_state`], ORG-06), so a client whose organization window
/// has drifted can be put right without a relog. The other members are not
/// told anything: nothing changed for them.
#[tracing::instrument(
    name = "org.gm_reload",
    level = "info",
    skip_all,
    fields(entity_id = gm.entity_id)
)]
pub async fn gm_reload(ctx: &OrgCtx<'_>, gm: GmCaller) -> Result<usize, OrgReject> {
    let mut row = ActionRow {
        event: GM_ACTION_EVENT,
        action: "gm_reload_organizations",
        player_id: Some(gm.player_id),
        entity_id: Some(gm.entity_id),
        ..ActionRow::default()
    };
    let fail = |row: ActionRow, why: OrgReject, text: String| async move {
        refuse(ctx, &row, gm.entity_id, why, &text).await
    };
    let Some(caller) = authorize(ctx, gm, &mut row) else {
        return fail(
            row,
            OrgReject::NotGm,
            "ReloadOrganizations: refused, GameMaster access is required.".into(),
        )
        .await;
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        return fail(
            row,
            OrgReject::NoDb,
            "ReloadOrganizations: failed, no database.".into(),
        )
        .await;
    };
    let memberships = match load_memberships(pool, caller.player_id).await {
        Ok(m) => m,
        Err(e) => {
            let why = db_failed(&row, &e);
            return fail(
                row,
                why,
                "ReloadOrganizations: failed, database error.".into(),
            )
            .await;
        }
    };
    let mut pushed = 0;
    for m in &memberships {
        // A failed push logs its own WARN `org.state_push_failed`.
        if push_org_state(ctx, m.header.org_id, &caller, false)
            .await
            .is_ok()
        {
            pushed += 1;
        }
    }
    row.count = Some(pushed);
    let text = match (memberships.len(), memberships.len() - pushed) {
        (0, _) => "ReloadOrganizations: you are in no Team or Command.".to_owned(),
        (n, 0) => format!("ReloadOrganizations: re-sent {n} organization(s)."),
        (n, failed) => format!(
            "ReloadOrganizations: re-sent {pushed} of {n} organization(s); {failed} failed."
        ),
    };
    feedback(ctx, gm.entity_id, &text).await;
    row.ok("reloaded");
    Ok(pushed)
}

/// "Team" or "Command" for a GM line.
fn type_label(t: OrgType) -> &'static str {
    match t {
        OrgType::Squad => "Squad",
        OrgType::Team => "Team",
        OrgType::Command => "Command",
    }
}

async fn character_by_id(
    pool: &sqlx::PgPool,
    player_id: i32,
) -> Result<Result<Character, OrgReject>, sqlx::Error> {
    let row: Option<(i32, i32, String)> = sqlx::query_as(
        "SELECT player_id, account_id, player_name FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|(player_id, account_id, name)| Character {
            player_id,
            account_id,
            name,
        })
        .ok_or(OrgReject::TargetNotFound))
}

/// An exact name match wins; otherwise a case-insensitive match must be
/// unique.
async fn character_by_name(
    pool: &sqlx::PgPool,
    name: &str,
) -> Result<Result<Character, OrgReject>, sqlx::Error> {
    let rows: Vec<(i32, i32, String)> = sqlx::query_as(
        "SELECT player_id, account_id, player_name FROM sgw_player \
         WHERE lower(player_name) = lower($1) ORDER BY player_id",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;
    let exact = rows.iter().find(|r| r.2 == name);
    let pick = match (exact, rows.as_slice()) {
        (Some(r), _) | (None, [r]) => Ok(r.clone()),
        (None, []) => Err(OrgReject::TargetNotFound),
        (None, _) => Err(OrgReject::TargetAmbiguous),
    };
    Ok(pick.map(|(player_id, account_id, name)| Character {
        player_id,
        account_id,
        name,
    }))
}
