//! `craft` (cell method 96): make a known blueprint's product from one of
//! its component sets.
//!
//! The request is validated here, in this order: the quantity is in
//! `1..=`[`MAX_CRAFT_QUANTITY`]; the player knows the blueprint; it is not
//! an alloy (those go through `alloying`); its discipline is known; every
//! submitted instance is the player's and sits in the main or crafting bag;
//! the submitted designs are exactly one component set's; and the two bags
//! hold `quantity × component.quantity` of each component. Nothing is
//! consumed at the request: a passing craft becomes one induction
//! ([`super::session`]). When its bar runs out one [`super::transaction`]
//! consumes the set by design from the two bags, grants
//! `blueprint.quantity × quantity` of the product, checks under a share
//! lock on the player row that the blueprint and its discipline are still
//! known (a respec may have come in between), and adds
//! [`CRAFT_EXPERTISE_GAIN`] to the discipline (136 goes out with the item
//! updates). The player then reads a success line.
//!
//! The craft page names one instance per component (the last stack it
//! found), so a component that needs several stacks is counted and
//! consumed by design, never from the named instances alone. The named
//! instances only choose the set; the completion does not hold the craft
//! to them. Every refusal
//! sends its line and changes nothing. The craft page keeps its slots on
//! confirm, so a request-time refusal needs no inventory resync; a refusal
//! at completion gets one from the transaction.

use crate::base::crafting::telemetry as crafting_telemetry;
use cimmeria_cell_catalog::crafting::shared_crafting_catalog;
use cimmeria_entity::known_names;
use sqlx::PgPool;

use super::feedback::{feedback_text_args, reject, CraftReject};
use super::persistence::load_crafting_state;
use super::request::CraftCtx;
use super::session::{
    crafting_sessions, Completion, CraftingSessions, InductionEnv, InductionJob, JobFuture,
    JobOutcome, SubmitOutcome,
};
use super::sync::CraftClient;
use super::telemetry::{
    account_id_of, record_request, send_to_player, sql_error_class, witness_send_failure, Outcome,
};
use super::transaction::apply_craft_transaction;
use crate::base::helpers::send_to_witness_reliable;
use crate::mercury::{build_player_entity_method_packet, method_idx};

pub mod inventory;
pub mod rules;

#[cfg(test)]
mod tests;

pub use rules::{CraftPlan, NamedInstance};

/// The cell method name: the `verb` field and metric label.
pub const VERB: &str = "craft";

/// The text subject for this verb's `Unavailable` rejection.
const ACTION: &str = "Crafting";

/// Most times one request may run a blueprint.
pub const MAX_CRAFT_QUANTITY: i32 = 100;

/// Expertise a completed craft adds to the blueprint's discipline.
pub const CRAFT_EXPERTISE_GAIN: i32 = 1;

/// The line a completed craft sends.
pub fn crafted_text(product: &str, units: i32) -> String {
    format!("You crafted {product} x{units}.")
}

/// The line a craft that waits behind others sends; the induction bar is
/// the answer for one that starts at once.
pub fn queued_text(product: &str, units: i32, ahead: usize) -> String {
    format!("Crafting {product} x{units} is queued behind {ahead} other crafting job(s).")
}

/// Handle a `Craft` request on the server's crafting sessions.
pub async fn handle_craft(
    entity_id: u32,
    player_id: i32,
    blueprint_id: i32,
    items: &[i32],
    quantity: i32,
    ctx: &CraftCtx<'_>,
) {
    handle_craft_with(
        crafting_sessions(),
        entity_id,
        player_id,
        blueprint_id,
        items,
        quantity,
        ctx,
    )
    .await;
}

/// [`handle_craft`] on explicit sessions, so tests drive the induction.
/// Events: `rejected` (from [`reject`]) for a refusal, `lookup_failed`
/// (WARN) when a read the decision needs failed, and the engine's job
/// events for an accepted craft.
pub async fn handle_craft_with(
    sessions: &std::sync::Arc<CraftingSessions>,
    entity_id: u32,
    player_id: i32,
    blueprint_id: i32,
    items: &[i32],
    quantity: i32,
    ctx: &CraftCtx<'_>,
) {
    let client = ctx.client();
    let plan = match decide(entity_id, player_id, blueprint_id, items, quantity, ctx).await {
        Ok(plan) => plan,
        Err(why) => {
            reject(VERB, entity_id, player_id, &why, client).await;
            return;
        }
    };
    let env = InductionEnv::from_ctx(ctx);
    let (product_name, units) = (plan.product_name.clone(), plan.plan.product_quantity);
    match sessions
        .submit(entity_id, player_id, Box::new(CraftJob(plan)), &env)
        .await
    {
        SubmitOutcome::Started => record_request(VERB, Outcome::Accepted),
        SubmitOutcome::Queued { position } => {
            record_request(VERB, Outcome::Accepted);
            send_note(
                client,
                entity_id,
                player_id,
                &queued_text(&product_name, units, position),
            )
            .await;
        }
        // Already answered and counted by the engine's refusal.
        SubmitOutcome::QueueFull => {}
        // No session plays the character: nobody to answer.
        SubmitOutcome::NotConnected => {}
    }
}

/// A validated craft and the product's display name.
#[derive(Debug, Clone)]
struct Decided {
    plan: CraftPlan,
    product_name: String,
}

/// Validate the request against the catalog, the player's crafting state
/// and inventory; the plan the induction will apply, or why not.
async fn decide(
    entity_id: u32,
    player_id: i32,
    blueprint_id: i32,
    items: &[i32],
    quantity: i32,
    ctx: &CraftCtx<'_>,
) -> Result<Decided, CraftReject> {
    rules::check_quantity(blueprint_id, quantity)?;
    let who = Who {
        account_id: account_id_of(entity_id, ctx.connected, ctx.entity_to_addr),
        player_id,
        entity_id,
        blueprint_id,
    };
    let unavailable = |phase: &'static str, error: &dyn LookupError| {
        who.lookup_failed(phase, error);
        CraftReject::Unavailable { action: ACTION }
    };
    let Some(pool) = ctx.db_pool.as_deref() else {
        return Err(unavailable("no_pool", &NoRow("no database pool")));
    };
    let catalog = shared_crafting_catalog(pool)
        .await
        .map_err(|e| unavailable("catalog", &e))?;
    let state = load_crafting_state(pool, player_id)
        .await
        .map_err(|e| unavailable("crafting_state", &e))?;
    let blueprint = rules::check_blueprint(&state, &catalog, blueprint_id)?;
    let Some(product_id) = blueprint.product_id else {
        return Err(unavailable(
            "blueprint_product",
            &NoRow("blueprint has no product"),
        ));
    };
    let found = inventory::named_instances(pool, player_id, items)
        .await
        .map_err(|e| unavailable("inventory", &e))?;
    let named = rules::check_named(items, &found)?;
    let available = inventory::carried_totals(pool, player_id, &rules::submitted_types(&named))
        .await
        .map_err(|e| unavailable("inventory", &e))?;
    let plan = rules::plan_craft(blueprint, product_id, &named, &available, quantity)?;
    let product_name = product_name(pool, product_id, &who).await;
    Ok(Decided { plan, product_name })
}

/// The identity a request-time `lookup_failed` WARN carries.
#[derive(Debug, Clone, Copy)]
struct Who {
    account_id: Option<u32>,
    player_id: i32,
    entity_id: u32,
    blueprint_id: i32,
}

impl Who {
    fn lookup_failed(&self, phase: &'static str, error: &dyn LookupError) {
        let player_label = known_names::player_name(self.player_id);
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            verb = VERB,
            phase,
            account_id = self.account_id,
            account_name = known_names::account_name(self.account_id),
            player_id = self.player_id,
            player_name = player_label,
            entity_id = self.entity_id,
            entity_name = player_label,
            blueprint_id = self.blueprint_id,
            blueprint_name = crafting_telemetry::blueprint_name(self.blueprint_id),
            error_class = error.class(),
            error = %error.text(),
            "craft: a lookup the decision needs failed"
        );
    }
}

/// The product's name for the player's lines; its id when the name cannot
/// be read (the craft itself does not depend on it).
async fn product_name(pool: &PgPool, product_id: i32, who: &Who) -> String {
    match inventory::item_name(pool, product_id).await {
        Ok(Some(name)) => name,
        Ok(None) => {
            who.lookup_failed("product_name", &NoRow("no resources.items row"));
            format!("item {product_id}")
        }
        Err(e) => {
            who.lookup_failed("product_name", &e);
            format!("item {product_id}")
        }
    }
}

/// A failed lookup, as the `error_class` and `error` of `lookup_failed`.
trait LookupError {
    fn class(&self) -> &'static str;
    fn text(&self) -> String;
}

impl LookupError for sqlx::Error {
    fn class(&self) -> &'static str {
        sql_error_class(self)
    }
    fn text(&self) -> String {
        self.to_string()
    }
}

/// A lookup that found nothing where the data should have had a row.
struct NoRow(&'static str);

impl LookupError for NoRow {
    fn class(&self) -> &'static str {
        "miss"
    }
    fn text(&self) -> String {
        self.0.to_string()
    }
}

/// Send a `CHAN_FEEDBACK` line that is not a refusal. A send that does not
/// go out is `client_sync_failed` (WARN).
async fn send_note(client: CraftClient<'_>, entity_id: u32, player_id: i32, text: &str) {
    let args = feedback_text_args(text);
    let method = method_idx::ON_PLAYER_COMMUNICATION;
    let outcome = send_to_witness_reliable(
        client.transport,
        client.connected,
        client.entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(key, seq, acks, entity_id, method, &args, version)
        },
    )
    .await;
    if let Some(reason) = witness_send_failure(&outcome) {
        let account_id = account_id_of(entity_id, client.connected, client.entity_to_addr);
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            target: "crafting",
            event = "client_sync_failed",
            verb = VERB,
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = player_label,
            entity_id,
            entity_name = player_label,
            what = "craft_queued",
            method,
            reason,
            "craft queued line not sent -- the player sees nothing for this press"
        );
    }
}

/// One queued craft.
struct CraftJob(Decided);

impl InductionJob for CraftJob {
    fn verb(&self) -> &'static str {
        VERB
    }

    fn timer_id(&self) -> i32 {
        self.0.plan.blueprint_id
    }

    fn complete<'a>(self: Box<Self>, done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async move {
            let Decided { plan, product_name } = self.0;
            let applied =
                match apply_craft_transaction(done.env, &done.ids, &plan.transaction).await {
                    Ok(applied) => applied,
                    // The transaction already sent the line and the resync.
                    Err(_) => return JobOutcome::Failed,
                };
            send_to_player(
                done.env,
                &done.ids,
                method_idx::ON_PLAYER_COMMUNICATION,
                &feedback_text_args(&crafted_text(&product_name, plan.product_quantity)),
                "craft_result",
            )
            .await;
            let mut report = applied.report();
            report.blueprint_id = Some(plan.blueprint_id);
            report.component_set_id = Some(plan.component_set_id);
            report.quantity = Some(plan.quantity);
            JobOutcome::Completed(Box::new(report))
        })
    }
}
