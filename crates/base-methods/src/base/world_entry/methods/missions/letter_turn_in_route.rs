//! Fenced plugin route from Marsh's cell interaction to the durable A1 transaction.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::helpers::{ExpectedLetterTurnInEntry, LetterTurnInSendOutcome};
use cimmeria_base_session::base::plugin::{
    BaseCtx, BasePlugin, BasePluginBuilder, BoxFuture, PluginMsg,
};
use cimmeria_base_session::base::ConnectedClientState;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{
    BaseToCellMsg, CommittedLetterTurnIn, LetterTurnInOutcome, LetterTurnInRefusal,
    LetterTurnInRequest, LetterTurnInResult,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::letter_turn_in::{
    commit_letter_turn_in, CommittedLetterTurnIn as SavedCommit, LetterTurnInRefusal as DbRefusal,
    LetterTurnInVerdict,
};
use crate::base::world_entry::methods::inventory::sync_committed_letter;

pub struct LetterTurnInPlugin;

impl BasePlugin for LetterTurnInPlugin {
    fn name(&self) -> &'static str {
        "letter_turn_in"
    }

    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.on_cell_message::<LetterTurnInRequest>(route_letter_turn_in);
    }
}

#[derive(Clone)]
struct OwnedCtx {
    pool: Option<Arc<PgPool>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl OwnedCtx {
    fn from_borrowed(ctx: BaseCtx<'_>) -> Self {
        Self {
            pool: ctx.db_pool.clone(),
            cell_tx: ctx.cell_tx.clone(),
            transport: Arc::clone(ctx.transport),
            connected: Arc::clone(ctx.connected),
            entity_to_addr: Arc::clone(ctx.entity_to_addr),
        }
    }

    fn borrowed(&self) -> BaseCtx<'_> {
        BaseCtx {
            db_pool: &self.pool,
            cell_tx: &self.cell_tx,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }
}

fn accepted_entry(
    request: &LetterTurnInRequest,
    ctx: BaseCtx<'_>,
) -> Option<ExpectedLetterTurnInEntry> {
    if request.entity_id == 0 || request.player_id <= 0 {
        return None;
    }
    let mapping = ctx.entity_to_addr.lock().ok()?;
    let addr = *mapping.get(&request.entity_id)?;
    let clients = ctx.connected.lock().ok()?;
    let state = clients.get(&addr)?;
    ExpectedLetterTurnInEntry::capture(addr, state, request)
}

/// The BaseService awaits plugin routing serially. This handler does all
/// admission synchronously, then hands database work to an owned task.
pub(super) fn route_letter_turn_in(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        let request = match msg.downcast::<LetterTurnInRequest>() {
            Ok(request) => request,
            Err(_) => {
                tracing::error!(target: "harset.letter_turn_in", event = "payload_mismatch");
                return;
            }
        };
        let Some(expected) = accepted_entry(&request, ctx) else {
            tracing::debug!(
                target: "harset.letter_turn_in",
                event = "stale_request",
                entity_id = request.entity_id,
                player_id = request.player_id,
                "Letter turn-in request did not match the live entry"
            );
            return;
        };
        let owned = OwnedCtx::from_borrowed(ctx);
        tokio::spawn(async move { run_accepted(request, expected, owned).await });
    })
}

async fn run_accepted(
    request: LetterTurnInRequest,
    expected: ExpectedLetterTurnInEntry,
    ctx: OwnedCtx,
) {
    let outcome = if let Some(pool) = ctx.pool.as_ref() {
        let attempt = lowercase_hex(request.attempt.bytes());
        let token = lowercase_hex(request.token.bytes());
        match commit_letter_turn_in(pool, request.player_id, &attempt, &token).await {
            Ok(verdict) => map_verdict(verdict),
            Err(error) => {
                tracing::error!(
                    target: "harset.letter_turn_in",
                    event = "transaction_uncertain",
                    entity_id = request.entity_id,
                    player_id = request.player_id,
                    %error,
                    "Letter turn-in commit outcome is uncertain"
                );
                LetterTurnInOutcome::UnknownRetryable
            }
        }
    } else {
        LetterTurnInOutcome::UnknownRetryable
    };

    if !expected.is_current(&ctx.connected, &ctx.entity_to_addr) {
        return;
    }
    let outcome = match outcome {
        LetterTurnInOutcome::Committed(ref committed)
        | LetterTurnInOutcome::AlreadyCommitted(ref committed) => {
            let Some(pool) = ctx.pool.as_ref() else {
                return;
            };
            match sync_committed_letter(
                pool,
                &expected,
                committed.item_id,
                committed.stack_after,
                ctx.borrowed(),
            )
            .await
            {
                Ok(LetterTurnInSendOutcome::Sent) => outcome,
                Ok(LetterTurnInSendOutcome::Stale) => return,
                Ok(other) => {
                    tracing::warn!(
                        target: "harset.letter_turn_in",
                        event = "client_sync_uncertain",
                        entity_id = request.entity_id,
                        player_id = request.player_id,
                        ?other,
                        "Committed letter inventory sync needs same-attempt retry"
                    );
                    LetterTurnInOutcome::UnknownRetryable
                }
                Err(error) => {
                    tracing::error!(
                        target: "harset.letter_turn_in",
                        event = "client_sync_query_failed",
                        entity_id = request.entity_id,
                        player_id = request.player_id,
                        %error
                    );
                    LetterTurnInOutcome::UnknownRetryable
                }
            }
        }
        _ => outcome,
    };
    send_result_if_current(LetterTurnInResult { request, outcome }, &expected, &ctx).await;
}

async fn send_result_if_current(
    result: LetterTurnInResult,
    expected: &ExpectedLetterTurnInEntry,
    ctx: &OwnedCtx,
) {
    let Some(tx) = ctx.cell_tx.as_ref() else {
        return;
    };
    let Ok(permit) = tx.reserve().await else {
        return;
    };
    let Ok(mapping) = ctx.entity_to_addr.lock() else {
        return;
    };
    if mapping.get(&expected.entity_id) != Some(&expected.addr) {
        return;
    }
    let Ok(clients) = ctx.connected.lock() else {
        return;
    };
    if expected.matches_connected(clients.get(&expected.addr)) {
        permit.send(BaseToCellMsg::LetterTurnInResult(result));
    }
}

pub(super) fn lowercase_hex(bytes: [u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(32);
    for byte in bytes {
        text.push(HEX[(byte >> 4) as usize] as char);
        text.push(HEX[(byte & 15) as usize] as char);
    }
    text
}

fn map_commit(commit: SavedCommit) -> CommittedLetterTurnIn {
    CommittedLetterTurnIn {
        mission: commit.mission,
        item_id: commit.item_id,
        container_id: commit.container_id,
        stack_before: commit.stack_before,
        stack_after: commit.stack_after,
    }
}

fn map_refusal(refusal: DbRefusal) -> LetterTurnInRefusal {
    match refusal {
        DbRefusal::InvalidCorrelation => LetterTurnInRefusal::InvalidCorrelation,
        DbRefusal::NoPlayer => LetterTurnInRefusal::NoPlayer,
        DbRefusal::AlreadyCompleted => LetterTurnInRefusal::AlreadyCompleted,
        DbRefusal::NoAccessibleLetter => LetterTurnInRefusal::NoAccessibleLetter,
        DbRefusal::WrongMissionState => LetterTurnInRefusal::WrongMissionState,
        DbRefusal::StateChanged => LetterTurnInRefusal::StateChanged,
    }
}

fn map_verdict(verdict: LetterTurnInVerdict) -> LetterTurnInOutcome {
    match verdict {
        LetterTurnInVerdict::Committed(commit) => {
            LetterTurnInOutcome::Committed(map_commit(commit))
        }
        LetterTurnInVerdict::AlreadyCommitted(commit) => {
            LetterTurnInOutcome::AlreadyCommitted(map_commit(commit))
        }
        LetterTurnInVerdict::Refused(refusal) => LetterTurnInOutcome::Refused(map_refusal(refusal)),
    }
}
