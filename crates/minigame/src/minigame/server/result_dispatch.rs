//! The single terminal outcome path from SmartFox to Base.

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

use crate::minigame::session::{DepositOutcome, MinigameSession, SessionRegistry};

pub(super) const RESULT_CANCELED: u8 = 0;
pub(super) const RESULT_VICTORY: u8 = 1;
pub(super) const RESULT_DEFEAT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DispatchOutcome {
    Delivered,
    Refused { player_canceled: bool },
    DeliveryFailed,
}

pub(super) async fn send_minigame_result(
    registry: &SessionRegistry,
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    session: &MinigameSession,
    result_code: u8,
    phase: &'static str,
    notify_client: bool,
) -> DispatchOutcome {
    send_minigame_result_with_sink(
        registry,
        result_tx,
        session,
        result_code,
        phase,
        notify_client,
        &cimmeria_discord::emit,
    )
    .await
}

async fn send_minigame_result_with_sink(
    registry: &SessionRegistry,
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    session: &MinigameSession,
    result_code: u8,
    phase: &'static str,
    notify_client: bool,
    event_sink: &(dyn Fn(cimmeria_discord::Event) + Sync),
) -> DispatchOutcome {
    if let DepositOutcome::Refused { player_canceled } = registry
        .deposit_result_decision(session, result_code, notify_client)
        .await
    {
        tracing::warn!(entity_id = session.entity_id,
            entity_name = session.player_name.as_deref(),
            player_id = session.player_id,
            player_name = session.player_name.as_deref(),
            game = %session.game_name, phase, reason = "result_deposit_refused",
            "Minigame terminal result refused");
        return DispatchOutcome::Refused { player_canceled };
    }
    if let Err(e) = result_tx
        .send(CellToBaseMsg::MinigameResult {
            ticket: session.ticket.clone(),
        })
        .await
    {
        registry.revoke_result(&session.ticket).await;
        tracing::error!(entity_id = session.entity_id,
            entity_name = session.player_name.as_deref(), player_id = session.player_id,
            player_name = session.player_name.as_deref(), game = %session.game_name,
            result_code, result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(result_code),
            chain_count = if result_code == RESULT_VICTORY { session.on_victory_chains.len() } else { 0 },
            phase, reason = "result_delivery_failed",
            "Minigame: result delivery failed -- chains will not fire: {e}");
        return DispatchOutcome::DeliveryFailed;
    }
    if let Some(event) = discord_result_event(
        &session.game_name,
        &session.discord_player(),
        session.entity_id,
        result_code,
        if result_code == RESULT_VICTORY {
            &session.on_victory_chains
        } else {
            &[]
        },
    ) {
        event_sink(event);
    }
    DispatchOutcome::Delivered
}

/// Discord only reflects a successfully enqueued victory or defeat.
pub(super) fn discord_result_event(
    game_name: &str,
    player: &cimmeria_discord::Named,
    entity_id: u32,
    result_code: u8,
    on_victory_chains: &[i64],
) -> Option<cimmeria_discord::Event> {
    matches!(result_code, RESULT_VICTORY | RESULT_DEFEAT).then(|| {
        cimmeria_discord::minigame_result_event(
            game_name,
            player.clone().or_entity(entity_id),
            result_code == RESULT_VICTORY,
            on_victory_chains
                .iter()
                .map(|&c| cimmeria_discord::Named::new(c, None))
                .collect(),
        )
    })
}
#[cfg(test)]
#[path = "result_dispatch_tests.rs"]
mod tests;
