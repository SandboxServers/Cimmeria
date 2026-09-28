//! Crafting respec, in two steps.
//!
//! 1. `.respeccraft` (a console line any player may type, forwarded by the
//!    cell as `CellToBaseMsg::RespecCraftOpen`): if the player has anything
//!    to clear, the base sends `onCraftingRespecPrompt(0)` (112) and opens a
//!    respec for [`RESPEC_WINDOW`] ([`handle_respec_open`]).
//! 2. The prompt's Yes sends `respecCrafting` (100). With a respec open for
//!    that player, one transaction clears every discipline and its expertise
//!    and refunds the applied science points spent on disciplines since the
//!    last respec (a GM-granted discipline refunds nothing)
//!    ([`handle_respec_confirm`]); then `onDisciplineRespec` (137), the
//!    unchanged known-crafts list (139) and the new ASP total.
//!
//! The client sends 100 only from that Yes and has no UI that opens a
//! respec, and 100 carries no arguments, so the open respec is the only
//! thing that tells a confirmation from a stray send. It is taken by the
//! first 100: a replayed or second 100 is refused and writes nothing. The
//! respec is free, and it keeps blueprints and racial paradigm levels,
//! which come from items the player used rather than from disciplines.
//!
//! Every refusal (nothing to clear, nothing open, the window closed, no
//! database) gets its text line through [`reject`].

mod pending;
mod transaction;

pub use pending::{take, with_session, PendingRespec, Taken, RESPEC_WINDOW};
pub use transaction::{respec_in_db, RespecFailure, Respecced};

use std::time::Instant;

use cimmeria_wire::crafting::RespecCraftOpen;

use super::feedback::{reject, CraftReject};
use super::persistence::load_crafting_state;
use super::request::CraftCtx;
use super::session::{crafting_sessions, CraftingSessions, DropReason};
use super::sync::{push_asp, push_discipline_respec, push_known_crafts, push_respec_prompt};
use super::telemetry::{account_id_of, record_request, sql_error_class, Outcome};

/// What a crafting respec costs, in naquadah: nothing.
pub const RESPEC_COST: i32 = 0;

/// The `verb` of the console step: the command name, since it is not a
/// cell method.
pub const VERB_OPEN: &str = "respeccraft";

/// The `verb` of the confirmation: the cell method name.
pub const VERB_CONFIRM: &str = "respecCrafting";

/// The text subject for this verb's `Unavailable` rejection.
const ACTION: &str = "Crafting respec";

/// Handle a player's `.respeccraft`: prompt and open a respec, or refuse.
/// Events: `request`, then `respec_prompted`, or `rejected`
/// (`nothing_to_respec`, `unavailable`); `lookup_failed` (WARN) when the
/// player has no session to hold the respec.
#[tracing::instrument(
    name = "crafting.request",
    level = "info",
    skip_all,
    fields(verb = VERB_OPEN)
)]
pub async fn handle_respec_open(msg: RespecCraftOpen, ctx: &CraftCtx<'_>) {
    let RespecCraftOpen {
        entity_id,
        player_id,
    } = msg;
    let client = ctx.client();
    let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
    tracing::info!(
        target: "crafting",
        event = "request",
        verb = VERB_OPEN,
        account_id,
        player_id,
        entity_id,
        "crafting request"
    );
    let unavailable = CraftReject::Unavailable { action: ACTION };
    let Some(pool) = ctx.db_pool else {
        warn_persist_failed(VERB_OPEN, "no_pool", None, account_id, player_id, entity_id);
        reject(VERB_OPEN, entity_id, player_id, &unavailable, client).await;
        return;
    };
    let state = match load_crafting_state(pool, player_id).await {
        Ok(state) => state,
        Err(e) => {
            warn_persist_failed(
                VERB_OPEN,
                "load_state",
                Some(&e),
                account_id,
                player_id,
                entity_id,
            );
            reject(VERB_OPEN, entity_id, player_id, &unavailable, client).await;
            return;
        }
    };
    if state.discipline_ids.is_empty() && state.expertise.is_empty() {
        reject(
            VERB_OPEN,
            entity_id,
            player_id,
            &CraftReject::NothingToRespec,
            client,
        )
        .await;
        return;
    }

    let pending = PendingRespec::open(player_id, Instant::now());
    let Some(replaced) = with_session(entity_id, ctx.connected, ctx.entity_to_addr, |s| {
        s.pending_respec.replace(pending).is_some()
    }) else {
        // No session means no client to prompt either.
        tracing::warn!(
            target: "crafting",
            event = "lookup_failed",
            phase = "session",
            verb = VERB_OPEN,
            account_id,
            player_id,
            entity_id,
            "crafting respec for an entity with no session; nothing opened"
        );
        return;
    };
    tracing::info!(
        target: "crafting",
        event = "respec_prompted",
        verb = VERB_OPEN,
        account_id,
        player_id,
        entity_id,
        cost = RESPEC_COST,
        disciplines = state.discipline_ids.len(),
        asp = state.applied_science_points,
        window_secs = RESPEC_WINDOW.as_secs(),
        replaced,
        "crafting respec prompted"
    );
    record_request(VERB_OPEN, Outcome::Accepted);
    push_respec_prompt(entity_id, player_id, RESPEC_COST, client).await;
}

/// Handle `respecCrafting` (100): carry out the respec the player opened,
/// or refuse. Events: `respec` on success, `rejected`
/// (`no_pending_respec`, `respec_expired`, `nothing_to_respec`,
/// `unavailable`), `persist_failed` (WARN) on a failed transaction.
pub async fn handle_respec_confirm(entity_id: u32, player_id: i32, ctx: &CraftCtx<'_>) {
    confirm_with(crafting_sessions(), entity_id, player_id, ctx).await;
}

/// [`handle_respec_confirm`] against `sessions`, so a test can drive its
/// own induction engine. A confirmed respec drops the player's induction
/// queue (`queue_dropped`, `reason = respec`) before the transaction.
pub(crate) async fn confirm_with(
    sessions: &CraftingSessions,
    entity_id: u32,
    player_id: i32,
    ctx: &CraftCtx<'_>,
) {
    let client = ctx.client();
    let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
    let taken = with_session(entity_id, ctx.connected, ctx.entity_to_addr, |s| {
        take(&mut s.pending_respec, player_id, Instant::now())
    })
    .unwrap_or(Taken::Nothing);
    match taken {
        Taken::Open => {}
        Taken::Nothing => {
            reject(
                VERB_CONFIRM,
                entity_id,
                player_id,
                &CraftReject::NoPendingRespec,
                client,
            )
            .await;
            return;
        }
        Taken::Expired => {
            let why = CraftReject::RespecExpired {
                window_secs: RESPEC_WINDOW.as_secs(),
            };
            reject(VERB_CONFIRM, entity_id, player_id, &why, client).await;
            return;
        }
    }

    let unavailable = CraftReject::Unavailable { action: ACTION };
    let Some(pool) = ctx.db_pool else {
        warn_persist_failed(
            VERB_CONFIRM,
            "no_pool",
            None,
            account_id,
            player_id,
            entity_id,
        );
        reject(VERB_CONFIRM, entity_id, player_id, &unavailable, client).await;
        return;
    };
    // A queued craft, research or alloy must not complete for a discipline
    // the respec clears, so the queue goes before the transaction. A
    // completion already running holds the player-wide inventory key, and
    // the respec waits for it.
    sessions.drop_player(entity_id, DropReason::Respec, "respec_confirmed");
    let done = match respec_in_db(pool, player_id).await {
        Ok(Ok(done)) => done,
        Ok(Err(why)) => {
            reject(VERB_CONFIRM, entity_id, player_id, &why, client).await;
            return;
        }
        Err(failure) => {
            match &failure.error {
                Some(e) => warn_persist_failed(
                    VERB_CONFIRM,
                    failure.phase,
                    Some(e),
                    account_id,
                    player_id,
                    entity_id,
                ),
                None => tracing::warn!(
                    target: "crafting",
                    event = "persist_failed",
                    verb = VERB_CONFIRM,
                    phase = failure.phase,
                    reason = "rows_affected_short",
                    rows_affected = failure.rows_affected,
                    expected = 1u64,
                    account_id,
                    player_id,
                    entity_id,
                    "respec: the player row is missing, rolled back"
                ),
            }
            reject(VERB_CONFIRM, entity_id, player_id, &unavailable, client).await;
            return;
        }
    };

    tracing::info!(
        target: "crafting",
        event = "respec",
        verb = VERB_CONFIRM,
        account_id,
        player_id,
        entity_id,
        cleared = %format_cleared(&done.cleared),
        disciplines_cleared = done.cleared.len(),
        asp_refund = done.refund,
        expertise_rows_deleted = done.expertise_rows_deleted,
        asp_before = done.asp_before,
        asp_after = done.asp_after,
        cost = RESPEC_COST,
        blueprints_kept = done.blueprint_ids.len(),
        paradigm_levels_kept = %format_paradigms(&done.paradigm_levels),
        "crafting respec done"
    );
    record_request(VERB_CONFIRM, Outcome::Accepted);
    push_discipline_respec(entity_id, player_id, client).await;
    push_known_crafts(entity_id, player_id, &done.blueprint_ids, client).await;
    push_asp(entity_id, player_id, done.asp_after, client).await;
}

/// `discipline_id:expertise_before→0`, comma-separated, for the `respec`
/// event.
pub fn format_cleared(cleared: &[(i32, i32)]) -> String {
    cleared
        .iter()
        .map(|(id, before)| format!("{id}:{before}→0"))
        .collect::<Vec<_>>()
        .join(",")
}

/// `paradigm_id:level`, comma-separated, for the `respec` event.
pub fn format_paradigms(levels: &[(i32, i8)]) -> String {
    levels
        .iter()
        .map(|(id, level)| format!("{id}:{level}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn warn_persist_failed(
    verb: &'static str,
    phase: &'static str,
    error: Option<&sqlx::Error>,
    account_id: Option<u32>,
    player_id: i32,
    entity_id: u32,
) {
    tracing::warn!(
        target: "crafting",
        event = "persist_failed",
        verb,
        phase,
        error_class = error.map(sql_error_class),
        error = error.map(tracing::field::display),
        account_id,
        player_id,
        entity_id,
        "crafting respec: nothing was changed"
    );
}

#[cfg(test)]
mod refund_tests;
#[cfg(test)]
mod tests;
