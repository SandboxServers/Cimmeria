//! The GM crafting grants for UAT: `.craftkit <blueprint> [count]` and
//! `.learnblueprint <id>` (`CellToBaseMsg::GmCraftGrant`).
//!
//! The cell's `.`-console is GM-gated already; the base checks the caller's
//! session `access_level` again before writing anything, as `.allcraft`
//! does. Every outcome, a refusal included, sends the caller one line.
//!
//! - [`craftkit`]: the items of a blueprint's component set 1, `count`
//!   times over, placed by the crafting transaction's grant (a crafting
//!   component lands in the crafting bag).
//! - [`learn_blueprint`]: teach one blueprint and push the full list (139).

use cimmeria_wire::crafting::{GmCraftGrant, GmCraftGrantKind};

use super::allcraft::{caller_access_level, GM_ACCESS_LEVEL};
use super::request::CraftCtx;
use super::telemetry::account_id_of;
use crate::base::gm_feedback::send_gm_feedback_to_client;

pub mod craftkit;
pub mod learn_blueprint;

#[cfg(test)]
mod tests;

/// Who a grant is for and who asked: the identity every event carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GrantIds {
    /// The target's session account, `None` once the session is gone.
    pub account_id: Option<u32>,
    pub player_id: i32,
    pub entity_id: u32,
    pub gm_entity_id: u32,
}

/// Handle `CellToBaseMsg::GmCraftGrant`.
pub async fn handle_gm_craft_grant(msg: GmCraftGrant, ctx: &CraftCtx<'_>) {
    let ids = GrantIds {
        account_id: account_id_of(msg.entity_id, ctx.connected, ctx.entity_to_addr),
        player_id: msg.player_id,
        entity_id: msg.entity_id,
        gm_entity_id: msg.gm_entity_id,
    };
    match msg.grant {
        GmCraftGrantKind::Kit {
            blueprint_id,
            count,
        } => craftkit::handle_craftkit(ids, blueprint_id, count, ctx).await,
        GmCraftGrantKind::LearnBlueprint { blueprint_id } => {
            learn_blueprint::handle_learn_blueprint(ids, blueprint_id, ctx).await
        }
    }
}

/// Whether the caller may run a GM crafting grant. A caller below
/// GameMaster is logged (WARN, `event` = the command's event,
/// `reason=not_gm`) and told so; nothing is written.
async fn caller_is_gm(
    event: &'static str,
    command: &'static str,
    ids: GrantIds,
    ctx: &CraftCtx<'_>,
) -> bool {
    let access_level = caller_access_level(ids.gm_entity_id, ctx);
    if access_level >= GM_ACCESS_LEVEL {
        return true;
    }
    tracing::warn!(
        target: "crafting",
        event,
        outcome = "refused",
        reason = "not_gm",
        account_id = ids.account_id,
        player_id = ids.player_id,
        entity_id = ids.entity_id,
        gm_entity_id = ids.gm_entity_id,
        access_level,
        "GM crafting grant from a caller below GameMaster; refused"
    );
    gm_line(
        ids,
        &format!("{command}: refused, GameMaster access is required."),
        ctx,
    )
    .await;
    false
}

/// A `lookup_failed` WARN for a read the grant could not make.
fn lookup_failed(command: &'static str, phase: &'static str, ids: GrantIds, error: &str) {
    tracing::warn!(
        target: "crafting",
        event = "lookup_failed",
        command,
        phase,
        account_id = ids.account_id,
        player_id = ids.player_id,
        entity_id = ids.entity_id,
        gm_entity_id = ids.gm_entity_id,
        error,
        "GM crafting grant could not read what it needs"
    );
}

/// Send the GM who ran the command one line.
async fn gm_line(ids: GrantIds, text: &str, ctx: &CraftCtx<'_>) {
    send_gm_feedback_to_client(
        ids.gm_entity_id,
        text,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}
