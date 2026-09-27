//! SS-C3 (D-SS26, audit A-27): each Communicator base method the server
//! does not implement, 0xC6-0xCE, answers through
//! `dispatch_sgw_player_base_method` with its own feedback line and a
//! `chat.method_unsupported` WARN, and no longer reaches the silent
//! catch-all. One type 12 test per arm.

use std::time::{Duration, Instant};

use super::super::communicator_unsupported::*;
use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

const ADDR: &str = "127.0.0.1:54950";
const EID: u32 = 9200;

struct Harness {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

impl Harness {
    fn new() -> Self {
        let addr: SocketAddr = ADDR.parse().unwrap();
        let mut s = test_default_connected_client_state();
        s.player_entity_id = Some(EID);
        s.active_player_id = Some(0x7300_0320);
        s.account_id = 556;
        let transport = Arc::new(TestTransport::default());
        Self {
            addr,
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, s)]))),
        }
    }

    /// Through the real dispatcher, as the connect loop calls it.
    async fn press(&self, msg_id: u8) {
        let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
        dispatch_sgw_player_base_method(
            msg_id,
            &[1, 2, 3],
            &Some("Presser".to_string()),
            self.addr,
            &self.dyn_transport,
            [0u8; 32],
            &self.connected,
            &entity_manager,
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
            &None,
        )
        .await
        .expect("dispatch");
    }

    fn texts(&self) -> Vec<String> {
        self.transport
            .filter_to(self.addr)
            .iter()
            .map(|p| feedback_text(p))
            .collect()
    }
}

fn feedback_text(packet: &[u8]) -> String {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    assert_eq!(u32::from_le_bytes(body[3..7].try_into().unwrap()), EID);
    let args = &body[7..];
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let mut offset = 4 + speaker_len * 2 + 2;
    let n = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([args[offset + i * 2], args[offset + i * 2 + 1]]))
        .collect();
    String::from_utf16(&units).unwrap()
}

/// One press: exactly the method's own line, `chat.method_unsupported` at
/// WARN with `method`, `reason = not_implemented` and the ids, and not the
/// catch-all's "Unhandled SGWPlayer base method".
async fn assert_arm(msg_id: u8, method: &str, text: &str) {
    let capture = LogCapture::install();
    let h = Harness::new();
    h.press(msg_id).await;

    assert_eq!(
        h.texts(),
        vec![text.to_string()],
        "{method}: one feedback line"
    );
    let event = capture
        .find_event(
            Level::WARN,
            "not implemented on this server",
            "not_implemented",
        )
        .unwrap_or_else(|| panic!("{method}: chat.method_unsupported"));
    assert_eq!(event.target, "chat");
    assert!(event.has_field("event", "chat.method_unsupported"));
    assert!(event.has_field("method", method), "{event:#?}");
    assert!(event.has_field("player_id", &0x7300_0320.to_string()));
    assert!(event.has_field("account_id", "556"));
    assert!(event.has_field("entity_id", &EID.to_string()));
    assert!(event.has_field("payload_len", "3"));
    assert!(
        capture
            .find_message(Level::WARN, "Unhandled SGWPlayer base method")
            .is_none(),
        "{method}: must not fall into the catch-all"
    );
}

#[tokio::test]
async fn chat_friend_answers_with_feedback() {
    assert_arm(0xC6, "chatFriend", CHAT_FRIEND_TEXT).await;
}

#[tokio::test]
async fn chat_list_answers_with_feedback() {
    assert_arm(0xC7, "chatList", CHAT_LIST_TEXT).await;
}

#[tokio::test]
async fn chat_mute_answers_with_feedback() {
    assert_arm(0xC8, "chatMute", CHAT_MUTE_TEXT).await;
}

#[tokio::test]
async fn chat_kick_answers_with_feedback() {
    assert_arm(0xC9, "chatKick", CHAT_KICK_TEXT).await;
}

#[tokio::test]
async fn chat_op_answers_with_feedback() {
    assert_arm(0xCA, "chatOp", CHAT_OP_TEXT).await;
}

#[tokio::test]
async fn chat_ban_answers_with_feedback() {
    assert_arm(0xCB, "chatBan", CHAT_BAN_TEXT).await;
}

#[tokio::test]
async fn chat_password_answers_with_feedback() {
    assert_arm(0xCC, "chatPassword", CHAT_PASSWORD_TEXT).await;
}

#[tokio::test]
async fn petition_answers_with_feedback() {
    assert_arm(0xCD, "petition", PETITION_TEXT).await;
}

#[tokio::test]
async fn announce_petition_answers_with_feedback() {
    assert_arm(0xCE, "announcePetition", ANNOUNCE_PETITION_TEXT).await;
}

/// A flood of these presses costs chat tokens: five lines in a second, then
/// the one "too quickly" line, then silence, not one feedback packet per
/// request packet.
#[tokio::test]
async fn unsupported_presses_share_the_chat_bucket() {
    let h = Harness::new();
    let feedback = feedback::FeedbackCtx {
        transport: &h.dyn_transport,
        connected: &h.connected,
    };
    let t0 = Instant::now();
    for i in 0..8u64 {
        handle_unsupported_communicator(
            0xCD,
            0,
            &feedback,
            h.addr,
            t0 + Duration::from_millis(i * 10),
        )
        .await;
    }
    let texts = h.texts();
    assert_eq!(texts.len(), 6, "{texts:?}");
    assert!(texts[..5].iter().all(|t| t == PETITION_TEXT));
    assert_eq!(
        texts[5],
        crate::base::rate_limit::RateCategory::Chat.feedback_text()
    );
}
