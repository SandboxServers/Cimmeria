//! The crafting station gate on the base (CR-05, D-CR05, D-CR21).
//!
//! Crafting, research, reverse engineering and alloying each need a way to
//! work. A verb passes when any of these holds:
//!
//! 1. the cell's station mask grants it (`CraftRequest::allowed`, computed
//!    per request from the stations in reach);
//! 2. the session has "craft anywhere" (`.allcraft`, D-CR17);
//! 3. for every verb but alloying, a Field Crafting Tool in the crafting bag
//!    covers the work: for crafting, the blueprint's discipline; for
//!    research and reverse engineering, any of the item's disciplines.
//!
//! Learning a discipline and respec need no station. The client does no
//! check of its own (CR-E1 Q2), so a forged request lands here.

use cimmeria_cell_catalog::crafting::Discipline;
use cimmeria_cell_catalog::crafting::{shared_crafting_catalog, CraftType, CraftingCatalog};

use super::feedback::CraftReject;
use super::options::craft_anywhere;
use super::request::CraftCtx;
use super::tools::{load_held_tools, HeldTool};
use crate::cell::messages::{CraftRequest, CraftVerb};

/// The verb a station or tool is needed for; `None` for the verbs that need
/// neither.
pub fn gated_verb(verb: &CraftVerb) -> Option<CraftType> {
    match verb {
        CraftVerb::Craft { .. } => Some(CraftType::Craft),
        CraftVerb::Research { .. } => Some(CraftType::Research),
        CraftVerb::ReverseEngineer { .. } => Some(CraftType::ReverseEngineering),
        CraftVerb::Alloy { .. } => Some(CraftType::Alloying),
        CraftVerb::Spend { .. } | CraftVerb::Respec => None,
    }
}

/// Whether any held tool covers any of `disciplines` for `verb`. Alloying
/// is never tool-covered (D-CR21).
pub fn tools_cover(verb: CraftType, tools: &[HeldTool], disciplines: &[&Discipline]) -> bool {
    verb != CraftType::Alloying
        && tools
            .iter()
            .any(|t| disciplines.iter().any(|d| t.spec.covers(d)))
}

/// Check `request` against the gate. `Ok` when the verb needs no station or
/// something allows it; otherwise the rejection to send.
pub async fn check(request: &CraftRequest, ctx: &CraftCtx<'_>) -> Result<(), CraftReject> {
    let Some(verb) = gated_verb(&request.verb) else {
        return Ok(());
    };
    if verb.allowed_by(request.allowed)
        || craft_anywhere(request.entity_id, ctx.connected, ctx.entity_to_addr)
    {
        return Ok(());
    }
    let refused = CraftReject::NoStationOrTool { verb };
    if verb == CraftType::Alloying {
        return Err(refused);
    }
    let Some(pool) = ctx.db_pool.as_deref() else {
        return Err(refused);
    };
    let tools = match load_held_tools(pool, request.player_id).await {
        Ok(tools) => tools,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "gate_tools_failed",
                entity_id = request.entity_id,
                player_id = request.player_id,
                error = %e,
                "crafting bag read failed; treating as no tool"
            );
            return Err(refused);
        }
    };
    if tools.is_empty() {
        return Err(refused);
    }
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "gate_catalog_failed",
                entity_id = request.entity_id,
                error = %e,
                "crafting catalog unavailable; treating as no tool"
            );
            return Err(refused);
        }
    };
    let discipline_ids = match &request.verb {
        CraftVerb::Craft { blueprint_id, .. } => catalog
            .blueprint(*blueprint_id)
            .and_then(|b| b.discipline_id)
            .into_iter()
            .collect(),
        CraftVerb::Research { item_id, .. } | CraftVerb::ReverseEngineer { item_id } => {
            item_disciplines(pool, &catalog, request.player_id, *item_id).await
        }
        _ => Vec::new(),
    };
    let disciplines: Vec<&Discipline> = discipline_ids
        .iter()
        .filter_map(|&id| catalog.discipline(id))
        .collect();
    if tools_cover(verb, &tools, &disciplines) {
        Ok(())
    } else {
        Err(refused)
    }
}

/// The disciplines of the item instance `item_id`, if `player_id` owns it.
/// An unknown or foreign instance has none, so no tool covers it; the verb's
/// own packet reports the bad item once the gate lets a station through.
async fn item_disciplines(
    pool: &sqlx::PgPool,
    catalog: &CraftingCatalog,
    player_id: i32,
    item_id: i32,
) -> Vec<i32> {
    let type_id: Option<i32> = sqlx::query_scalar(
        "SELECT type_id FROM sgw_inventory WHERE item_id = $1 AND character_id = $2",
    )
    .bind(item_id)
    .bind(player_id)
    .fetch_optional(pool)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(
            target: "crafting",
            event = "gate_item_failed",
            player_id,
            item_id,
            error = %e,
            "item lookup failed; treating as no discipline"
        );
        None
    });
    type_id
        .and_then(|t| catalog.item(t))
        .map(|attrs| attrs.discipline_ids.clone())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
