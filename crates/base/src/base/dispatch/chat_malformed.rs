//! WARN rows for chat lines the base refuses before a channel sees them.
//!
//! Two families, both on the `chat` target:
//!
//! - `chat.malformed`: a payload that does not decode (`sendPlayerCommunication`
//!   or `chatJoin`), so no text reaches a handler.
//! - `chat.relay_failed`: a line that passed every gate but was not delivered,
//!   because the cell channel is closed or absent, or the speaker has no entity.
//!
//! The player's feedback is unchanged; these rows only make the drop visible.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::session_identity::identity_for_addr;
use cimmeria_entity::cell_entity::PlayerIdentity;

use super::super::ConnectedClientState;

/// The `chat.malformed` WARN row, for a payload the base could not decode.
///
/// Takes the `connected` lock itself, so it must not be called while that
/// lock is held.
pub(super) fn log_malformed(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    reason: &'static str,
    payload_len: usize,
) {
    let ident = identity_for_addr(connected, addr);
    tracing::warn!(
        target: "chat",
        event = "chat.malformed",
        %addr,
        player_id = ident.player_id,
        player_name = ident.player_name,
        account_id = ident.account_id,
        account_name = ident.account_name,
        reason,
        payload_len,
        "chat payload malformed, not handled",
    );
}

/// The `chat.relay_failed` WARN row, for a line that was not delivered.
///
/// `entity_id` is `None` when the speaker has no entity; the entity name is
/// then left off with it (Rule 6).
pub(super) fn log_relay_failed(
    addr: SocketAddr,
    ident: PlayerIdentity,
    entity_id: Option<u32>,
    channel: u8,
    reason: &'static str,
) {
    tracing::warn!(
        target: "chat",
        event = "chat.relay_failed",
        %addr,
        player_id = ident.player_id,
        player_name = ident.player_name,
        account_id = ident.account_id,
        account_name = ident.account_name,
        entity_id,
        entity_name = entity_id.and(ident.player_name),
        channel,
        reason,
        "chat line not delivered, the speaker's message was dropped",
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use cimmeria_mercury::transport::Transport;
    use tokio::sync::mpsc;
    use tracing::Level;

    use super::super::chat::{handle_chat_join, send_player_communication_at, ChatRoutes};
    use crate::base::ConnectedClientState;
    use crate::cell::messages::BaseToCellMsg;
    use crate::mercury::write_wstring;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

    const ADDR: &str = "127.0.0.1:54680";
    const PLAYER_EID: u32 = 4680;
    const MALFORMED_MSG: &str = "chat payload malformed, not handled";
    const RELAY_MSG: &str = "chat line not delivered";

    fn addr() -> SocketAddr {
        ADDR.parse().unwrap()
    }

    /// One session on [`ADDR`]; `with_entity` decides whether it owns an entity.
    fn connected(with_entity: bool) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
        let mut state = test_default_connected_client_state();
        state.player_entity_id = with_entity.then_some(PLAYER_EID);
        state.active_player_id = Some(77);
        Arc::new(Mutex::new(HashMap::from([(addr(), state)])))
    }

    /// A `sendPlayerCommunication` payload on the say channel, no target.
    fn say_payload(text: &str) -> Vec<u8> {
        let mut payload = vec![0];
        write_wstring(&mut payload, "");
        write_wstring(&mut payload, text);
        payload
    }

    /// Runs `sendPlayerCommunication` for `payload` against `cell_tx`.
    async fn send(
        payload: &[u8],
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    ) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
        send_player_communication_at(
            payload,
            &Some("Tester".to_string()),
            addr(),
            &transport,
            connected,
            ChatRoutes {
                cell_tx: &cell_tx,
                entity_to_addr: &Arc::new(Mutex::new(HashMap::new())),
                db_pool: &None,
            },
            Instant::now(),
        )
        .await;
    }

    #[tokio::test]
    async fn empty_payload_is_logged_as_malformed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        send(&[], &connected, None).await;
        assert!(capture
            .find_event(Level::WARN, MALFORMED_MSG, "empty_payload")
            .is_some());
    }

    #[tokio::test]
    async fn bad_target_is_logged_as_malformed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        // Channel byte only: the target WSTRING has no length prefix.
        send(&[0], &connected, None).await;
        let row = capture
            .find_event(Level::WARN, MALFORMED_MSG, "bad_target")
            .expect("chat.malformed bad_target row");
        assert!(row.has_field("payload_len", "1"));
    }

    #[tokio::test]
    async fn bad_text_is_logged_as_malformed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        // A valid empty target, then no text WSTRING at all.
        let mut payload = vec![0];
        write_wstring(&mut payload, "");
        send(&payload, &connected, None).await;
        assert!(capture
            .find_event(Level::WARN, MALFORMED_MSG, "bad_text")
            .is_some());
    }

    #[tokio::test]
    async fn bad_channel_name_is_logged_as_malformed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
        handle_chat_join(&[], addr(), &transport, &connected).await;
        assert!(capture
            .find_event(Level::WARN, MALFORMED_MSG, "bad_channel_name")
            .is_some());
    }

    #[tokio::test]
    async fn closed_cell_channel_is_logged_as_relay_failed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
        drop(rx);
        send(&say_payload("hello"), &connected, Some(tx)).await;
        assert!(capture
            .find_event(Level::WARN, RELAY_MSG, "cell_channel_closed")
            .is_some());
    }

    #[tokio::test]
    async fn absent_cell_channel_is_logged_as_relay_failed() {
        let capture = LogCapture::install();
        let connected = connected(true);
        send(&say_payload("hello"), &connected, None).await;
        assert!(capture
            .find_event(Level::WARN, RELAY_MSG, "no_cell_channel")
            .is_some());
    }

    #[tokio::test]
    async fn speaker_without_entity_is_logged_as_relay_failed() {
        let capture = LogCapture::install();
        let connected = connected(false);
        let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
        send(&say_payload("hello"), &connected, Some(tx)).await;
        assert!(capture
            .find_event(Level::WARN, RELAY_MSG, "entity_missing")
            .is_some());
    }
}
