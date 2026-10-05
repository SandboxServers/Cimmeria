//! `.learnblueprint <id>`: teach the target one blueprint.
//!
//! It covers the blueprints no Blueprint item teaches. The blueprint must
//! exist in the crafting catalog and not be known yet. The load, the
//! change and the save run in one transaction holding the player row, like
//! every other crafting write, and the client gets the full saved list in
//! `onUpdateKnownCrafts` (139). The success event is `blueprint_learned`,
//! in the shape a Blueprint item's use logs, with `source=gm`.

use crate::base::crafting::telemetry as crafting_telemetry;
use cimmeria_cell_catalog::crafting::shared_crafting_catalog;
use cimmeria_entity::crafting::CraftingState;
use cimmeria_entity::known_names;

use super::{caller_is_gm, gm_line, lookup_failed, GrantIds};
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::base::crafting::persistence::{load_crafting_state_locked, save_crafting_state_in};
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::sync::push_known_crafts;
use crate::base::crafting::telemetry::sql_error_class;

/// Why a blueprint was not taught.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnRefusal {
    /// No blueprint with that id.
    UnknownBlueprint,
    /// The target knows it already.
    AlreadyKnown,
}

impl LearnRefusal {
    /// The `reason` field of a refused `gm_learnblueprint`.
    pub fn reason(self) -> &'static str {
        match self {
            LearnRefusal::UnknownBlueprint => "unknown_blueprint",
            LearnRefusal::AlreadyKnown => "already_known",
        }
    }

    /// The GM's line.
    pub fn text(self, entity_id: u32, blueprint_id: i32) -> String {
        match self {
            LearnRefusal::UnknownBlueprint => {
                format!("learnblueprint: refused, there is no blueprint {blueprint_id}.")
            }
            LearnRefusal::AlreadyKnown => format!(
                "learnblueprint [{entity_id}]: refused, blueprint {blueprint_id} is already known."
            ),
        }
    }
}

/// Teach `blueprint_id` to `state`, keeping the list sorted. Returns the
/// known count before and after.
pub fn learn(state: &mut CraftingState, blueprint_id: i32) -> Result<(usize, usize), LearnRefusal> {
    if state.blueprint_ids.contains(&blueprint_id) {
        return Err(LearnRefusal::AlreadyKnown);
    }
    let before = state.blueprint_ids.len();
    state.blueprint_ids.push(blueprint_id);
    state.blueprint_ids.sort_unstable();
    Ok((before, state.blueprint_ids.len()))
}

/// Handle `.learnblueprint` for one target.
#[tracing::instrument(
    name = "crafting.learnblueprint",
    level = "info",
    skip_all,
    fields(entity_id = ids.entity_id, player_id = ids.player_id, gm = ids.gm_entity_id)
)]
pub(super) async fn handle_learn_blueprint(ids: GrantIds, blueprint_id: i32, ctx: &CraftCtx<'_>) {
    if !caller_is_gm("gm_learnblueprint", "learnblueprint", ids, ctx).await {
        return;
    }
    let refused = |why: LearnRefusal| {
        tracing::info!(
            target: "crafting",
            event = "gm_learnblueprint",
            outcome = "refused",
            reason = why.reason(),
            account_id = ids.account_id,
            account_name = known_names::account_name(ids.account_id),
            player_id = ids.player_id,
            player_name = known_names::player_name(ids.player_id),
            entity_id = ids.entity_id,
            entity_name = known_names::player_name(ids.player_id),
            gm_entity_id = ids.gm_entity_id,
            gm_entity_name = ids.gm_name,
            blueprint_id,
            blueprint_name = crafting_telemetry::blueprint_name(blueprint_id),
            "learnblueprint refused; nothing was taught"
        );
    };
    let persist_failed = |phase: &'static str, e: &sqlx::Error| {
        // A player row that is not there is `RowNotFound`: no row matched.
        let rows_affected: Option<u64> = matches!(e, sqlx::Error::RowNotFound).then_some(0);
        tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            command = "learnblueprint",
            phase,
            rows_affected,
            expected = 1u64,
            error_class = sql_error_class(e),
            account_id = ids.account_id,
            account_name = known_names::account_name(ids.account_id),
            player_id = ids.player_id,
            player_name = known_names::player_name(ids.player_id),
            entity_id = ids.entity_id,
            entity_name = known_names::player_name(ids.player_id),
            gm_entity_id = ids.gm_entity_id,
            gm_entity_name = ids.gm_name,
            blueprint_id,
            blueprint_name = crafting_telemetry::blueprint_name(blueprint_id),
            error = %e,
            "learnblueprint save failed; nothing was taught"
        );
    };
    let failed_line = "learnblueprint: failed, the crafting state could not be saved.";

    let Some(pool) = ctx.db_pool.as_deref() else {
        lookup_failed("learnblueprint", "db_pool", ids, "no database pool");
        gm_line(ids, "learnblueprint: failed, no database.", ctx).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(c) => c,
        Err(e) => {
            lookup_failed("learnblueprint", "catalog", ids, &e.to_string());
            gm_line(
                ids,
                "learnblueprint: failed, the crafting catalog could not be loaded.",
                ctx,
            )
            .await;
            return;
        }
    };
    if catalog.blueprint(blueprint_id).is_none() {
        refused(LearnRefusal::UnknownBlueprint);
        gm_line(
            ids,
            &LearnRefusal::UnknownBlueprint.text(ids.entity_id, blueprint_id),
            ctx,
        )
        .await;
        return;
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            persist_failed("begin", &e);
            gm_line(ids, failed_line, ctx).await;
            return;
        }
    };
    // The player-wide advisory key first, as every other crafting write
    // takes it: a crafting completion holds it (not the `sgw_player` row)
    // while it writes expertise, and the save below rewrites every
    // expertise row from this load, so without it a completion that
    // commits between the load and the save is written back over.
    if let Err(e) = take_inventory_locks(&mut tx, ids.player_id, &[]).await {
        persist_failed("advisory_lock", &e);
        gm_line(ids, failed_line, ctx).await;
        return;
    }
    let mut state = match load_crafting_state_locked(&mut tx, ids.player_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            persist_failed("load_crafting_state_locked", &sqlx::Error::RowNotFound);
            gm_line(ids, failed_line, ctx).await;
            return;
        }
        Err(e) => {
            lookup_failed("learnblueprint", "load_state", ids, &e.to_string());
            gm_line(
                ids,
                "learnblueprint: failed, the crafting state could not be loaded.",
                ctx,
            )
            .await;
            return;
        }
    };
    let (known_before, known_after) = match learn(&mut state, blueprint_id) {
        Ok(counts) => counts,
        Err(why) => {
            // Nothing was written; dropping the transaction rolls it back.
            refused(why);
            gm_line(ids, &why.text(ids.entity_id, blueprint_id), ctx).await;
            return;
        }
    };
    let saved = match save_crafting_state_in(&mut tx, ids.player_id, &state).await {
        Ok(()) => tx.commit().await.map_err(|e| ("commit", e)),
        Err(e) => Err(("save_crafting_state", e)),
    };
    if let Err((phase, e)) = saved {
        persist_failed(phase, &e);
        gm_line(ids, failed_line, ctx).await;
        return;
    }

    tracing::info!(
        target: "crafting",
        event = "blueprint_learned",
        source = "gm",
        account_id = ids.account_id,
        account_name = known_names::account_name(ids.account_id),
        player_id = ids.player_id,
        player_name = known_names::player_name(ids.player_id),
        entity_id = ids.entity_id,
        entity_name = known_names::player_name(ids.player_id),
        gm_entity_id = ids.gm_entity_id,
        gm_entity_name = ids.gm_name,
        blueprint_id,
        blueprint_name = crafting_telemetry::blueprint_name(blueprint_id),
        blueprints = %format!("{blueprint_id}:false→true"),
        known_before,
        known_after,
        "blueprint taught by a GM"
    );
    push_known_crafts(
        ids.entity_id,
        ids.player_id,
        &state.blueprint_ids,
        ctx.client(),
    )
    .await;
    gm_line(
        ids,
        &format!(
            "learnblueprint [{}]: blueprint {blueprint_id} learned ({known_after} known).",
            ids.entity_id
        ),
        ctx,
    )
    .await;
}
