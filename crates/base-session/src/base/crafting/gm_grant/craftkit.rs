//! `.craftkit <blueprint> [count]`: grant the items of a blueprint's
//! component set 1, each `count` times its quantity, so a tester can craft
//! it `count` times.
//!
//! The grant runs through the crafting transaction with a grant-only plan:
//! each item goes to the first carried bag its `container_sets` allow (the
//! crafting bag for the `{17,15}` crafting components, where the crafting
//! verbs read them), merging into a stack with room or taking free slots.
//! The whole kit commits or nothing does. The target's client gets the
//! inventory update and the cell its inventory events, as after a craft.

use cimmeria_cell_catalog::crafting::{shared_crafting_catalog, CraftingCatalog};

use super::{caller_is_gm, gm_line, lookup_failed, GrantIds};
use crate::base::crafting::request::CraftCtx;
use crate::base::crafting::session::InductionEnv;
use crate::base::crafting::telemetry::JobIds;
use crate::base::crafting::transaction::{
    apply_grant_transaction, CraftApplied, CraftTransaction, CraftTxError,
};

/// The component set a kit grants. Sets are alternative recipes; set 1 is
/// the one every blueprint with components has.
pub const KIT_COMPONENT_SET: i32 = 1;

/// The most crafts' worth one `.craftkit` grants. The crafting bag holds
/// 100 slots and no crafting component stacks, so a larger kit only fills
/// the bag and is refused.
pub const MAX_KIT_COUNT: i32 = 10;

/// The `verb` a kit's transaction carries on its `persist_failed` events.
const KIT_VERB: &str = "craftkit";

/// Why a kit cannot be built, before anything is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitRefusal {
    /// `count` is outside `1..=MAX_KIT_COUNT`.
    BadCount,
    /// No blueprint with that id.
    UnknownBlueprint,
    /// The blueprint has no component set 1 (blueprint 21 has none).
    NoComponents,
}

impl KitRefusal {
    /// The `reason` field of a refused `gm_craftkit`.
    pub fn reason(self) -> &'static str {
        match self {
            KitRefusal::BadCount => "bad_count",
            KitRefusal::UnknownBlueprint => "unknown_blueprint",
            KitRefusal::NoComponents => "no_components",
        }
    }

    /// The GM's line.
    pub fn text(self, blueprint_id: i32, count: i32) -> String {
        match self {
            KitRefusal::BadCount => {
                format!("craftkit: refused, count {count} is not between 1 and {MAX_KIT_COUNT}.")
            }
            KitRefusal::UnknownBlueprint => {
                format!("craftkit: refused, there is no blueprint {blueprint_id}.")
            }
            KitRefusal::NoComponents => format!(
                "craftkit: refused, blueprint {blueprint_id} has no component set \
                 {KIT_COMPONENT_SET}."
            ),
        }
    }
}

/// The `(design_id, quantity)` grants of a kit: every component of the
/// blueprint's set 1, `count` times its quantity, in the set's item order.
pub fn kit_grants(
    catalog: &CraftingCatalog,
    blueprint_id: i32,
    count: i32,
) -> Result<Vec<(i32, i32)>, KitRefusal> {
    if !(1..=MAX_KIT_COUNT).contains(&count) {
        return Err(KitRefusal::BadCount);
    }
    let blueprint = catalog
        .blueprint(blueprint_id)
        .ok_or(KitRefusal::UnknownBlueprint)?;
    let set = blueprint
        .component_set(KIT_COMPONENT_SET)
        .filter(|set| !set.components.is_empty())
        .ok_or(KitRefusal::NoComponents)?;
    let mut grants: Vec<(i32, i32)> = Vec::with_capacity(set.components.len());
    for c in &set.components {
        // A seeded quantity is small and `count` is capped, so the
        // product cannot overflow; saturate rather than trust that.
        let quantity = c.quantity.max(1).saturating_mul(count);
        match grants.iter_mut().find(|(item_id, _)| *item_id == c.item_id) {
            Some((_, q)) => *q = q.saturating_add(quantity),
            None => grants.push((c.item_id, quantity)),
        }
    }
    Ok(grants)
}

/// The GM's line after a granted kit.
pub fn granted_text(
    entity_id: u32,
    blueprint_id: i32,
    count: i32,
    applied: &CraftApplied,
) -> String {
    let items: i32 = applied.granted.iter().map(|g| g.quantity()).sum();
    format!(
        "craftkit [{entity_id}]: blueprint {blueprint_id} x{count}, {items} items granted \
         ({}).",
        applied.granted_field()
    )
}

/// Handle `.craftkit` for one target.
#[tracing::instrument(
    name = "crafting.craftkit",
    level = "info",
    skip_all,
    fields(entity_id = ids.entity_id, player_id = ids.player_id, gm = ids.gm_entity_id)
)]
pub(super) async fn handle_craftkit(
    ids: GrantIds,
    blueprint_id: i32,
    count: i32,
    ctx: &CraftCtx<'_>,
) {
    if !caller_is_gm("gm_craftkit", "craftkit", ids, ctx).await {
        return;
    }
    let refused = |reason: &'static str| {
        tracing::info!(
            target: "crafting",
            event = "gm_craftkit",
            outcome = "refused",
            reason,
            account_id = ids.account_id,
            player_id = ids.player_id,
            entity_id = ids.entity_id,
            gm_entity_id = ids.gm_entity_id,
            blueprint_id,
            count,
            "craftkit refused; nothing was granted"
        );
    };
    let Some(pool) = ctx.db_pool.as_ref() else {
        lookup_failed("craftkit", "db_pool", ids, "no database pool");
        gm_line(ids, "craftkit: failed, no database.", ctx).await;
        return;
    };
    let catalog = match shared_crafting_catalog(pool).await {
        Ok(c) => c,
        Err(e) => {
            lookup_failed("craftkit", "catalog", ids, &e.to_string());
            gm_line(
                ids,
                "craftkit: failed, the crafting catalog could not be loaded.",
                ctx,
            )
            .await;
            return;
        }
    };
    let grants = match kit_grants(&catalog, blueprint_id, count) {
        Ok(g) => g,
        Err(why) => {
            refused(why.reason());
            gm_line(ids, &why.text(blueprint_id, count), ctx).await;
            return;
        }
    };

    // A kit is not an induction: job id 0 marks its transaction events.
    let job = JobIds {
        job_id: 0,
        verb: KIT_VERB,
        account_id: ids.account_id.unwrap_or(0),
        player_id: ids.player_id,
        entity_id: ids.entity_id,
    };
    let plan = CraftTransaction {
        grant: grants,
        ..CraftTransaction::default()
    };
    let env = InductionEnv::from_ctx(ctx);
    match apply_grant_transaction(&env, pool, &job, &plan).await {
        Ok(applied) => {
            tracing::info!(
                target: "crafting",
                event = "gm_craftkit",
                outcome = "granted",
                account_id = ids.account_id,
                player_id = ids.player_id,
                entity_id = ids.entity_id,
                gm_entity_id = ids.gm_entity_id,
                blueprint_id,
                count,
                component_set = KIT_COMPONENT_SET,
                granted = %applied.granted_field(),
                "craftkit granted"
            );
            gm_line(
                ids,
                &granted_text(ids.entity_id, blueprint_id, count, &applied),
                ctx,
            )
            .await;
        }
        Err(CraftTxError::Rejected(why)) => {
            refused(why.reason());
            gm_line(
                ids,
                &format!(
                    "craftkit: refused ({}), the target's bags cannot take the kit; \
                     nothing was granted.",
                    why.reason()
                ),
                ctx,
            )
            .await;
        }
        // `apply_grant_transaction` logged `persist_failed` with its phase.
        Err(_) => {
            gm_line(
                ids,
                "craftkit: failed, the grant could not be saved; nothing was granted.",
                ctx,
            )
            .await;
        }
    }
}
