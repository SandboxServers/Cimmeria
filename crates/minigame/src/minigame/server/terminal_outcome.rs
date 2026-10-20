//! Map one terminal game output to its dispatch and closing evidence.

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

use super::result_dispatch::{send_minigame_result, DispatchOutcome, RESULT_VICTORY};
use super::EndReason;
use crate::minigame::games;
use crate::minigame::session::{MinigameSession, SessionRegistry};

pub(super) async fn report_terminal(
    registry: &SessionRegistry,
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    session: &MinigameSession,
    result_code: u8,
    phase: &'static str,
) -> EndReason {
    let outcome =
        send_minigame_result(registry, result_tx, session, result_code, phase, true).await;
    match outcome {
        DispatchOutcome::Delivered => {
            let victory = result_code == RESULT_VICTORY;
            tracing::info!(entity_id = session.entity_id,
                entity_name = session.player_name.as_deref(), player_id = session.player_id,
                player_name = session.player_name.as_deref(), game = %session.game_name,
                validation = games::validation(&session.game_name),
                "Minigame {}", if victory { "victory" } else { "failure" });
            registry
                .record_event(
                    session.entity_id,
                    &session.ticket,
                    if victory {
                        "result_victory"
                    } else {
                        "result_defeat"
                    },
                    None,
                )
                .await;
            if victory {
                EndReason::Victory
            } else {
                EndReason::Defeat
            }
        }
        DispatchOutcome::Refused {
            player_canceled: true,
        } => EndReason::PlayerCanceled,
        DispatchOutcome::Refused {
            player_canceled: false,
        } => EndReason::ResultRefused,
        DispatchOutcome::DeliveryFailed => EndReason::ResultDeliveryFailed,
    }
}
