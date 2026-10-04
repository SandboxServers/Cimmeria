//! The one place a minigame outcome leaves the minigame server.
//!
//! Every outcome — victory, defeat, and (since CA04) an abandoned session —
//! funnels through [`send_minigame_result`] so the Discord emit, the result
//! codes and the send-failure log stay in one place.

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

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
    player: &cimmeria_discord::Named,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    phase: &'static str,
) {
    // Discord gameplay-channel: a minigame finished (on by default — low
    // volume / high signal). The player is the `player_id` and character
    // name the base registered the session with; the victory chains are
    // what the game was played for (chains have no name, so `#id`).
    //
    // `RESULT_CANCELED` is excluded: a player closing the minigame window
    // is not a game outcome, and reporting it would put a "lost" line in
    // the channel every time someone changes their mind.
    if let Some(event) = discord_result_event(
        game_name,
        player,
        entity_id,
        result_code,
        &on_victory_chains,
    ) {
        cimmeria_discord::emit(event);
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
            entity_name = player.name(),
            player_id = player.id,
            player_name = player.name(),
            game = %game_name,
            result_code,
            result = cimmeria_wire::cell::client_methods::minigame::minigame_result_name(result_code),
            chain_count,
            phase,
            "Minigame: result delivery failed -- chains will not fire: {e}",
        );
    }
}

/// The Discord event for a minigame outcome, `None` for
/// [`RESULT_CANCELED`]. Split out of [`send_minigame_result`] so a test can
/// see the event without the global Discord runtime.
pub(super) fn discord_result_event(
    game_name: &str,
    player: &cimmeria_discord::Named,
    entity_id: u32,
    result_code: u8,
    on_victory_chains: &[i64],
) -> Option<cimmeria_discord::Event> {
    (result_code != RESULT_CANCELED).then(|| {
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
            &cimmeria_discord::Named::new(7, Some("Hacker".into())),
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
            event.has_field("result", "victory"),
            "the result code is named next to its number (NT-31): {event:#?}"
        );
        // Rule 6: the player the outcome is lost for is named, not just
        // the entity slot.
        for (key, want) in [
            ("entity_name", "Hacker"),
            ("player_id", "7"),
            ("player_name", "Hacker"),
        ] {
            assert!(
                event.has_field(key, want),
                "a lost minigame result must name its player: expected \
                 {key}={want}; got {event:#?}"
            );
        }
        assert!(
            event.has_field("chain_count", "2"),
            "chain_count must reflect the on_victory_chains length so a \
             quest-stall investigation can correlate the count of lost \
             chains: {:#?}",
            event
        );
    }

    /// The log name of every code this server sends (NT-31).
    #[test]
    fn result_codes_have_log_names() {
        use cimmeria_wire::cell::client_methods::minigame::minigame_result_name;
        assert_eq!(minigame_result_name(RESULT_CANCELED), Some("canceled"));
        assert_eq!(minigame_result_name(RESULT_VICTORY), Some("victory"));
        assert_eq!(minigame_result_name(RESULT_DEFEAT), Some("defeat"));
        assert_eq!(minigame_result_name(3), None);
    }

    /// Happy-path: receiver alive, exactly one `MinigameResult` arrives
    /// with the right shape. Guards against a regression that drops
    /// the result silently when the channel IS healthy (e.g., a future
    /// refactor that adds a "skip-if-empty-chains" branch).
    #[tokio::test]
    async fn send_minigame_result_dispatches_through_open_channel() {
        let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(1);

        send_minigame_result(
            &tx,
            99,
            "hack",
            &cimmeria_discord::Named::default(),
            RESULT_DEFEAT,
            vec![],
            "failure_tick",
        )
        .await;

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

    /// The character name the base hands the registry reaches the Discord
    /// event, paired with the session's `player_id` (NT-10).
    #[tokio::test]
    async fn registered_player_name_reaches_the_discord_event() {
        let reg = crate::minigame::SessionRegistry::new();
        let ticket = reg
            .register(
                4401,
                7,
                "Hack".into(),
                1,
                1,
                0,
                0,
                0,
                1,
                vec![1017],
                Some("Hacker".into()),
            )
            .await
            .unwrap();
        let session = reg.authenticate(4401, &ticket, "Hack").await.unwrap();
        let event = discord_result_event(
            "Hack",
            &session.discord_player(),
            session.entity_id,
            RESULT_VICTORY,
            &session.on_victory_chains,
        );
        match event {
            Some(cimmeria_discord::Event::MinigameResult {
                character,
                success,
                victory_chains,
                ..
            }) => {
                assert_eq!(
                    character,
                    cimmeria_discord::Named::new(7, Some("Hacker".into()))
                );
                assert!(success);
                assert_eq!(
                    victory_chains,
                    vec![cimmeria_discord::Named::new(1017, None)]
                );
            }
            other => panic!("expected a MinigameResult, got {other:?}"),
        }
        assert!(discord_result_event(
            "Hack",
            &session.discord_player(),
            4401,
            RESULT_CANCELED,
            &[]
        )
        .is_none());
    }
}
