//! `.allcraft`: a GM tool that makes a player able to try every
//! crafting page at once, for UAT.
//!
//! It sets every racial paradigm to 7, learns every discipline at expertise
//! 100, grants every blueprint, persists that in one save, pushes it to the
//! client (136 per discipline, 138 per paradigm, 139), and turns on "craft
//! anywhere" for the target's session: the options name the player as its
//! own machine in all four sections, and the gate lets every verb through.
//! It changes no applied-science points.
//!
//! The cell's `.`-console is GM-gated already; the base checks the caller's
//! session `access_level` again before writing anything.

use cimmeria_cell_catalog::crafting::shared_crafting_catalog;
use cimmeria_entity::crafting::CraftingState;
use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};
use cimmeria_wire::cell::client_methods::player::{
    ON_UPDATE_DISCIPLINE, ON_UPDATE_KNOWN_CRAFTS, ON_UPDATE_RACIAL_PARADIGM_LEVEL,
};
use cimmeria_wire::crafting::{
    known_crafts_args, racial_paradigm_level_args, update_discipline_args, GmAllCraft,
};

use super::options::enable_craft_anywhere;
use super::persistence::{load_crafting_state, save_crafting_state};
use super::request::CraftCtx;
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::helpers::send_bundle_to_witness_reliable;
use crate::base::session_identity::identity_for_entity;

/// The paradigm level `.allcraft` sets, as the legacy command did.
pub const ALL_CRAFT_PARADIGM_LEVEL: i8 = 7;
/// The expertise `.allcraft` sets on every discipline (the legacy command
/// used 50).
pub const ALL_CRAFT_EXPERTISE: i32 = 100;

/// Minimum `access_level` for `.allcraft`: GameMaster.
const GM_ACCESS_LEVEL: u32 = 2;

/// Give `state` every discipline at [`ALL_CRAFT_EXPERTISE`], every blueprint
/// and every paradigm at [`ALL_CRAFT_PARADIGM_LEVEL`]. Ids end up sorted.
pub fn apply_all_craft(
    state: &mut CraftingState,
    discipline_ids: &[i32],
    blueprint_ids: &[i32],
    paradigm_ids: &[i32],
) {
    for &id in discipline_ids {
        state.set_expertise(id, ALL_CRAFT_EXPERTISE);
        if !state.discipline_ids.contains(&id) {
            state.discipline_ids.push(id);
        }
    }
    state.discipline_ids.sort_unstable();
    for &id in blueprint_ids {
        if !state.blueprint_ids.contains(&id) {
            state.blueprint_ids.push(id);
        }
    }
    state.blueprint_ids.sort_unstable();
    for &id in paradigm_ids {
        state
            .racial_paradigm_levels
            .insert(id, ALL_CRAFT_PARADIGM_LEVEL);
    }
}

/// The client update for a crafting state: 136 per known discipline, 138
/// per paradigm, then 139, all on the player's own entity.
pub fn crafting_state_bundle(entity_id: u32, state: &CraftingState) -> ChannelBundle {
    let mut bundle = ChannelBundle::new(true);
    for &id in &state.discipline_ids {
        let expertise = state.get_expertise(id).unwrap_or(0);
        bundle.append_entity_method(
            ON_UPDATE_DISCIPLINE,
            IDBASE_SGW_PLAYER,
            entity_id,
            &update_discipline_args(id, expertise),
        );
    }
    let mut paradigms: Vec<(i32, i8)> = state
        .racial_paradigm_levels
        .iter()
        .map(|(&id, &level)| (id, level))
        .collect();
    paradigms.sort_unstable();
    for (id, level) in paradigms {
        bundle.append_entity_method(
            ON_UPDATE_RACIAL_PARADIGM_LEVEL,
            IDBASE_SGW_PLAYER,
            entity_id,
            &racial_paradigm_level_args(id, level),
        );
    }
    bundle.append_entity_method(
        ON_UPDATE_KNOWN_CRAFTS,
        IDBASE_SGW_PLAYER,
        entity_id,
        &known_crafts_args(&state.blueprint_ids),
    );
    bundle
}

/// The caller's session access level; 0 when it has no session.
fn caller_access_level(gm_entity_id: u32, ctx: &CraftCtx<'_>) -> u32 {
    let Some(addr) = ctx
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&gm_entity_id).copied())
    else {
        return 0;
    };
    ctx.connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).map(|c| c.access_level))
        .unwrap_or(0)
}

/// Paradigm levels in id order, for the `gm_allcraft` before/after fields.
fn paradigm_levels(state: &CraftingState) -> Vec<(i32, i8)> {
    let mut levels: Vec<(i32, i8)> = state
        .racial_paradigm_levels
        .iter()
        .map(|(&id, &level)| (id, level))
        .collect();
    levels.sort_unstable();
    levels
}

/// Handle `CellToBaseMsg::GmAllCraft`.
#[tracing::instrument(
    name = "crafting.allcraft",
    level = "info",
    skip_all,
    fields(entity_id = msg.entity_id, player_id = msg.player_id, gm = msg.gm_entity_id)
)]
pub async fn handle_gm_all_craft(msg: GmAllCraft, ctx: &CraftCtx<'_>) {
    let GmAllCraft {
        entity_id,
        player_id,
        gm_entity_id,
    } = msg;
    let account_id = identity_for_entity(ctx.connected, ctx.entity_to_addr, entity_id).account_id;
    let feedback = |text: String| async move {
        send_gm_feedback_to_client(
            gm_entity_id,
            &text,
            ctx.transport,
            ctx.connected,
            ctx.entity_to_addr,
        )
        .await;
    };
    let lookup_failed = |phase: &'static str, error: &dyn std::fmt::Display| {
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            phase,
            account_id,
            player_id,
            entity_id,
            gm_entity_id,
            error = %error,
            "allcraft could not read what it grants"
        );
    };

    let access_level = caller_access_level(gm_entity_id, ctx);
    if access_level < GM_ACCESS_LEVEL {
        tracing::warn!(
            target: "crafting",
            event = "gm_allcraft",
            outcome = "refused",
            account_id,
            player_id,
            entity_id,
            gm_entity_id,
            access_level,
            "allcraft from a caller below GameMaster; refused"
        );
        feedback("allcraft: refused, GameMaster access is required.".into()).await;
        return;
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        lookup_failed("db_pool", &"no database pool");
        feedback("allcraft: failed, no database.".into()).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(c) => c,
        Err(e) => {
            lookup_failed("catalog", &e);
            feedback("allcraft: failed, the crafting catalog could not be loaded.".into()).await;
            return;
        }
    };
    let paradigm_ids: Vec<i32> =
        match sqlx::query_scalar("SELECT id FROM resources.racial_paradigm ORDER BY id")
            .fetch_all(pool)
            .await
        {
            Ok(ids) => ids,
            Err(e) => {
                lookup_failed("paradigms", &e);
                feedback("allcraft: failed, the racial paradigms could not be read.".into()).await;
                return;
            }
        };
    let mut state = match load_crafting_state(pool, player_id).await {
        Ok(s) => s,
        Err(e) => {
            lookup_failed("load_state", &e);
            feedback("allcraft: failed, the crafting state could not be loaded.".into()).await;
            return;
        }
    };
    let (disciplines_before, blueprints_before, paradigms_before) = (
        state.discipline_ids.len(),
        state.blueprint_ids.len(),
        paradigm_levels(&state),
    );
    let mut discipline_ids: Vec<i32> = catalog.disciplines.keys().copied().collect();
    let mut blueprint_ids: Vec<i32> = catalog.blueprints.keys().copied().collect();
    discipline_ids.sort_unstable();
    blueprint_ids.sort_unstable();
    apply_all_craft(&mut state, &discipline_ids, &blueprint_ids, &paradigm_ids);
    if let Err(e) = save_crafting_state(pool, player_id, &state).await {
        // `save_crafting_state` reports an UPDATE that matched no player row
        // as `RowNotFound`.
        let rows_affected: Option<u64> = matches!(e, sqlx::Error::RowNotFound).then_some(0);
        tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            phase = "save_crafting_state",
            rows_affected,
            expected = 1u64,
            account_id,
            player_id,
            entity_id,
            error = %e,
            "allcraft save failed; nothing was granted"
        );
        feedback("allcraft: failed, the crafting state could not be saved.".into()).await;
        return;
    }

    let outcome = send_bundle_to_witness_reliable(
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
        entity_id,
        crafting_state_bundle(entity_id, &state),
    )
    .await;
    if let Some(reason) = outcome.failure_reason() {
        tracing::warn!(
            target: "crafting",
            event = "send_failed",
            reason,
            method = "136/138/139",
            account_id,
            player_id,
            entity_id,
            "allcraft's discipline, paradigm and blueprint update did not reach the client"
        );
    }
    enable_craft_anywhere(entity_id, ctx.transport, ctx.connected, ctx.entity_to_addr).await;

    tracing::info!(
        target: "crafting",
        event = "gm_allcraft",
        outcome = "granted",
        account_id,
        player_id,
        entity_id,
        gm_entity_id,
        disciplines_before,
        disciplines_after = state.discipline_ids.len(),
        blueprints_before,
        blueprints_after = state.blueprint_ids.len(),
        paradigm_levels_before = ?paradigms_before,
        paradigm_levels_after = ?paradigm_levels(&state),
        "allcraft granted"
    );
    feedback(format!(
        "allcraft [{entity_id}]: {} disciplines at {ALL_CRAFT_EXPERTISE}, {} blueprints,          {} paradigms at {ALL_CRAFT_PARADIGM_LEVEL}; craft anywhere is on until logout.",
        discipline_ids.len(),
        blueprint_ids.len(),
        paradigm_ids.len(),
    ))
    .await;
}

#[cfg(test)]
#[path = "allcraft_tests.rs"]
mod tests;
