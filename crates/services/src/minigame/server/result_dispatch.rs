//! The one place a minigame outcome leaves the minigame server.
//!
//! Every outcome — victory, defeat, and (since CA04) an abandoned session —
//! funnels through [`send_minigame_result`] so the Discord emit, the result
//! codes and the send-failure log stay in one place.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

// Result codes, matching C++ `MinigameResult`
// (`deprecated/cpp/src/baseapp/minigame.hpp`) and
// `deprecated/python/common/Constants.py`. `Canceled` is a *distinct* code
// from `Defeat`: the original reported it whenever a session ended without
// the game producing an outcome, and the cell treats it as inert.
pub(super) const RESULT_CANCELED: u8 = 0;
pub(super) const RESULT_VICTORY: u8 = 1;
pub(super) const RESULT_DEFEAT: u8 = 2;

/// Dispatch a `MinigameResult` upstream and log on send failure.
///
/// Extracted from the inline call sites in [`super::run_session`] so the
/// five (game-driven / tick-driven × victory / failure, plus abort)
/// variants share one error-handling path. `phase` names which call site
/// fired so the ops log distinguishes them. Historically these were
/// `let _ = result_tx.send(...).await`, silently swallowing the loss of a
/// minigame outcome — chains never fired, quest stalled.
pub(super) async fn send_minigame_result(
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    game_name: &str,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    phase: &'static str,
) {
    // Discord gameplay-channel: a minigame finished (on by default — low
    // volume / high signal). The minigame server only holds the entity id,
    // so the character name is best-effort (`entity:<id>`); resolving the
    // display name would require a cross-service round-trip not worth the
    // coupling here.
    //
    // `RESULT_CANCELED` is excluded: a player closing the minigame window
    // is not a game outcome, and reporting it would put a "lost" line in
    // the channel every time someone changes their mind.
    if result_code != RESULT_CANCELED {
        cimmeria_discord::emit_minigame_result(
            game_name,
            format!("entity:{entity_id}"),
            result_code == RESULT_VICTORY,
        );
    }

    let chain_count = on_victory_chains.len();
    if let Err(e) = result_tx
        .send(CellToBaseMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        })
        .await
    {
        tracing::error!(
            entity_id,
            game = %game_name,
            result_code,
            chain_count,
            phase,
            "Minigame: result delivery failed -- chains will not fire: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// `send_minigame_result` is the shared helper for every outcome call
    /// site (game-driven victory/failure, tick-driven victory/failure, and
    /// the abort report). Dropping the receiver simulates a downed
    /// cell↔base channel; the guard asserts the ERROR fires naming the
    /// `phase` so all sites stay distinguishable in ops logs.
    #[tokio::test]
    async fn send_minigame_result_logs_error_when_receiver_closed() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel::<CellToBaseMsg>(1);
        drop(rx);

        send_minigame_result(
            &tx,
            /* entity_id */ 4242,
            /* game_name */ "livewire",
            RESULT_VICTORY,
            /* on_victory_chains */ vec![100, 200],
            /* phase */ "victory_message",
        )
        .await;

        let event = capture
            .find_message(Level::ERROR, "Minigame: result delivery failed")
            .expect("negative-logging convention: closed result_tx must emit ERROR");
        assert!(
            event.has_field("phase", "victory_message"),
            "phase field must be carried so every call site \
             (victory_message / failure_message / victory_tick / failure_tick / \
             aborted) stays distinguishable in ops logs: {:#?}",
            event
        );
        assert!(
            event.has_field("chain_count", "2"),
            "chain_count must reflect the on_victory_chains length so a \
             quest-stall investigation can correlate the count of lost \
             chains: {:#?}",
            event
        );
    }

    /// Happy-path: receiver alive, exactly one `MinigameResult` arrives
    /// with the right shape. Guards against a regression that drops
    /// the result silently when the channel IS healthy (e.g., a future
    /// refactor that adds a "skip-if-empty-chains" branch).
    #[tokio::test]
    async fn send_minigame_result_dispatches_through_open_channel() {
        let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(1);

        send_minigame_result(&tx, 99, "hack", RESULT_DEFEAT, vec![], "failure_tick").await;

        match rx.try_recv() {
            Ok(CellToBaseMsg::MinigameResult {
                entity_id,
                result_code,
                on_victory_chains,
            }) => {
                assert_eq!(entity_id, 99);
                assert_eq!(result_code, RESULT_DEFEAT);
                assert!(on_victory_chains.is_empty());
            }
            other => panic!("expected MinigameResult, got {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "exactly one message");
    }
}
