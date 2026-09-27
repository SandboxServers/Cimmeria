//! SS-00: the chat flood limit (D-SS14) and length cap (D-SS12) on
//! `sendPlayerCommunication`, both enforced before the cell forward.
//!
//! Type 12 (negative log) plus the forward itself: each guard asserts the
//! `rate_limit` / `chat` event AND that no `ChatMessage` reached the cell,
//! AND the one feedback line the player sees.

use std::time::{Duration, Instant};

use super::super::chat::{
    send_player_communication_at, CHAT_BAD_CHARACTER_TEXT, CHAT_TOO_LONG_TEXT,
};
use super::super::*;
use crate::base::rate_limit::{RateCategory, RateDecision};
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

const ADDR: &str = "127.0.0.1:54600";
const PLAYER_EID: u32 = 4321;

pub(super) struct Harness {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Harness {
    pub(super) fn new(access_level: u32) -> Self {
        let addr: SocketAddr = ADDR.parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(PLAYER_EID);
        state.active_player_id = Some(77);
        state.access_level = access_level;
        let transport = Arc::new(TestTransport::default());
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(64);
        Self {
            addr,
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            cell_tx: Some(tx),
            cell_rx: rx,
        }
    }

    async fn say(&self, text: &str, now: Instant) {
        self.speak(0, text, now).await; // say
    }

    /// `sendPlayerCommunication` on `channel` with no target.
    pub(super) async fn speak(&self, channel: u8, text: &str, now: Instant) {
        let mut payload = vec![channel];
        crate::mercury::write_wstring(&mut payload, "");
        crate::mercury::write_wstring(&mut payload, text);
        send_player_communication_at(
            &payload,
            &Some("Tester".to_string()),
            self.addr,
            &self.dyn_transport,
            &self.connected,
            &self.cell_tx,
            now,
        )
        .await;
    }

    /// Every line forwarded to the cell so far.
    pub(super) fn forwarded(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(msg) = self.cell_rx.try_recv() {
            match msg {
                BaseToCellMsg::ChatMessage { text, .. } => out.push(text),
                _ => panic!("only ChatMessage is expected on the cell channel"),
            }
        }
        out
    }

    /// Every feedback line sent to the player so far, decoded.
    fn feedback(&self) -> Vec<String> {
        self.transport
            .filter_to(self.addr)
            .iter()
            .map(|p| decode_feedback_text(p))
            .collect()
    }
}

/// Decrypt one `onPlayerCommunication` packet (all-zero test key) and
/// return its text, checking it is addressed to the player on channel 9.
fn decode_feedback_text(packet: &[u8]) -> String {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    assert_eq!(
        u32::from_le_bytes(body[3..7].try_into().unwrap()),
        PLAYER_EID,
        "feedback must be addressed to the player's own entity"
    );
    let args = &body[7..];
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let channel = args[4 + speaker_len * 2 + 1];
    assert_eq!(channel, 9, "feedback rides the tell channel");
    let mut offset = 4 + speaker_len * 2 + 2;
    let text_len = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let units: Vec<u16> = (0..text_len)
        .map(|i| u16::from_le_bytes(args[offset + i * 2..offset + i * 2 + 2].try_into().unwrap()))
        .collect();
    String::from_utf16(&units).unwrap()
}

/// D-SS14: burst 5, refill 1/s. Five lines inside one second reach the
/// cell; the sixth is dropped before the forward, logs
/// `rate_limit.exceeded category=chat` at WARN and sends one feedback line.
#[tokio::test]
async fn chat_bucket_drops_sixth_line_in_one_second() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    let t0 = Instant::now();
    for i in 0..6u64 {
        h.say(&format!("line {i}"), t0 + Duration::from_millis(i * 150))
            .await;
    }

    assert_eq!(
        h.forwarded(),
        vec!["line 0", "line 1", "line 2", "line 3", "line 4"],
        "the sixth line inside one second must not reach the cell"
    );
    let event = capture
        .find_event(Level::WARN, "rate_limit.exceeded", "bucket_empty")
        .expect("the dropped line must log rate_limit.exceeded at WARN");
    assert_eq!(event.target, "rate_limit");
    assert!(event.has_field("category", "chat"));
    assert!(event.has_field("event", "rate_limit.exceeded"));
    // Telemetry: who, and the bucket state the drop was decided on.
    assert!(event.has_field("player_id", "77"));
    assert!(event.has_field("account_id", "0"));
    assert!(event.has_field("entity_id", &PLAYER_EID.to_string()));
    assert!(event.has_field("tokens", "0"));
    assert!(event.has_field("burst", "5"));
    assert!(event.has_field("refill_ms", "1000"));
    // 750 ms into the first period (the sixth line is at t0 + 750 ms).
    assert!(event.has_field("next_token_ms", "250"));
    assert_eq!(
        h.feedback(),
        vec!["You are sending messages too quickly."],
        "the first drop tells the player, once"
    );
    let accepted_rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::INFO && c.message_contains("sendPlayerCommunication"))
        .collect();
    assert_eq!(
        accepted_rows.len(),
        5,
        "the per-line INFO row logs accepted lines only, so a flood cannot flood the log"
    );
    // Rule 5 ids on the accepted-line row too.
    for row in &accepted_rows {
        assert!(row.has_field("player_id", "77"));
        assert!(row.has_field("account_id", "0"));
        assert!(row.has_field("entity_id", &PLAYER_EID.to_string()));
    }
}

/// The feedback for a flood is itself limited to one line per 5 seconds;
/// the drops in between log at DEBUG only. After the refill, lines flow
/// again.
#[tokio::test]
async fn chat_flood_notifies_once_per_five_seconds_and_recovers() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    let t0 = Instant::now();
    for _ in 0..5 {
        h.say("burst", t0).await;
    }
    // 20 more lines in the same instant: all dropped, one notify.
    for _ in 0..20 {
        h.say("flood", t0).await;
    }
    assert_eq!(h.forwarded().len(), 5);
    assert_eq!(
        h.feedback().len(),
        1,
        "one feedback line per 5 s, not per drop"
    );
    let silent = capture
        .find_event(Level::DEBUG, "notify suppressed", "bucket_empty")
        .expect("the silent drops log at DEBUG");
    assert!(silent.has_field("entity_id", &PLAYER_EID.to_string()));

    // One second later one token is back.
    h.say("after refill", t0 + Duration::from_secs(1)).await;
    assert_eq!(h.forwarded(), vec!["after refill"]);

    // Still inside the 5 s notify window: a new drop stays silent.
    h.say("again", t0 + Duration::from_secs(1)).await;
    assert_eq!(h.feedback().len(), 1);
    // Past it: the next drop notifies again.
    // Past it (t0 + 5 s): four tokens have come back since t0 + 1 s, so the
    // fifth line is the next drop, and it notifies again.
    for _ in 0..5 {
        h.say("x", t0 + Duration::from_secs(5)).await;
    }
    assert_eq!(h.forwarded().len(), 4);
    assert_eq!(h.feedback().len(), 2);
}

/// D-SS14: GameMaster (2) and above skip the chat bucket.
#[tokio::test]
async fn chat_bucket_exempts_gamemaster_and_above() {
    for (access, forwarded) in [(0u32, 5usize), (1, 5), (2, 20), (3, 20)] {
        let mut h = Harness::new(access);
        let t0 = Instant::now();
        for _ in 0..20 {
            h.say("hi", t0).await;
        }
        assert_eq!(
            h.forwarded().len(),
            forwarded,
            "access level {access}: only GameMaster and above are exempt"
        );
    }
}

/// D-SS12: a line over 255 UTF-16 units is refused before the cell (not
/// truncated), logged on the `chat` target, and the player told.
#[tokio::test]
async fn chat_rejects_text_over_cap() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    let t0 = Instant::now();

    h.say(&"x".repeat(256), t0).await;

    assert!(
        h.forwarded().is_empty(),
        "an over-cap line must never reach the cell"
    );
    let event = capture
        .find_event(Level::WARN, "chat text rules", "too_long")
        .expect("the refused line must log chat.rejected reason=too_long at WARN");
    assert_eq!(event.target, "chat");
    assert!(event.has_field("event", "chat.rejected"));
    assert!(event.has_field("text_units", "256"));
    assert!(event.has_field("player_id", "77"));
    assert!(event.has_field("account_id", "0"));
    assert!(event.has_field("entity_id", &PLAYER_EID.to_string()));
    assert_eq!(h.feedback(), vec![CHAT_TOO_LONG_TEXT]);
}

/// D-SS12 with the D-ORG10 character rules (one implementation,
/// `org_text::validate(TextField::ChatText, ..)`): controls, bidi and
/// zero-width characters and newlines are refused before the cell, each with
/// its own `reason`.
#[tokio::test]
async fn chat_rejects_forbidden_characters() {
    for (text, reason) in [
        ("hi\u{202E}olleh", "bidi_control"),
        ("in\u{200B}visible", "zero_width"),
        ("tab\there", "control_char"),
        ("two\nlines", "control_char"),
        ("soft\u{00AD}hyphen", "format_char"),
        ("sep\u{2028}arator", "line_separator"),
    ] {
        let capture = LogCapture::install();
        let mut h = Harness::new(0);
        h.say(text, Instant::now()).await;
        assert!(h.forwarded().is_empty(), "{text:?} must not reach the cell");
        let event = capture
            .find_event(Level::WARN, "chat text rules", reason)
            .unwrap_or_else(|| panic!("{text:?} must log chat.rejected reason={reason}"));
        assert_eq!(event.target, "chat");
        assert!(event.has_field("entity_id", &PLAYER_EID.to_string()));
        assert_eq!(h.feedback(), vec![CHAT_BAD_CHARACTER_TEXT], "{text:?}");
    }
    // Ordinary non-Latin text is fine.
    let mut h = Harness::new(0);
    h.say("Kree! Jaffa, ça va? 界", Instant::now()).await;
    assert_eq!(h.forwarded(), vec!["Kree! Jaffa, ça va? 界"]);
}

/// The cap counts UTF-16 units: 255 is accepted, and a supplementary-plane
/// character counts two.
#[tokio::test]
async fn chat_cap_counts_utf16_units() {
    let mut h = Harness::new(0);
    let t0 = Instant::now();
    let exactly_255 = "x".repeat(255);
    h.say(&exactly_255, t0).await;
    // 127 rockets = 254 units, plus one ASCII = 255: accepted.
    let rockets_255 = format!("{}x", "🚀".repeat(127));
    h.say(&rockets_255, t0).await;
    // 128 rockets = 256 units: refused although it is only 128 chars.
    h.say(&"🚀".repeat(128), t0).await;
    assert_eq!(h.forwarded(), vec![exactly_255, rockets_255]);
    assert_eq!(h.feedback(), vec![CHAT_TOO_LONG_TEXT]);
}

/// The public dispatch arm goes through the same gates (wiring guard for
/// `handle_send_player_communication` itself). The arm reads the real clock,
/// so the test never counts wall-clock refills: it empties the bucket at an
/// instant an hour ahead, where a clock behind the bucket earns nothing, and
/// then every dispatched line must be dropped however slow the run is.
/// Exact timing is the explicit-clock tests' job.
#[tokio::test]
async fn dispatch_arm_applies_the_chat_bucket() {
    let mut h = Harness::new(0);
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let dispatch_say = |text: &'static str| {
        let mut payload = vec![0u8];
        crate::mercury::write_wstring(&mut payload, "");
        crate::mercury::write_wstring(&mut payload, text);
        let (transport, connected, cell_tx) = (&h.dyn_transport, &h.connected, &h.cell_tx);
        let (entity_manager, entity_to_addr, addr) = (&entity_manager, &entity_to_addr, h.addr);
        async move {
            dispatch_sgw_player_base_method(
                sgw_player_base::SEND_PLAYER_COMMUNICATION,
                &payload,
                &Some("Tester".to_string()),
                addr,
                transport,
                [0; 32],
                connected,
                entity_manager,
                cell_tx,
                entity_to_addr,
                &None,
            )
            .await
            .unwrap();
        }
    };

    // A full bucket lets a line through the arm.
    dispatch_say("before").await;
    // Empty it where the real clock cannot catch up.
    {
        let later = Instant::now() + Duration::from_secs(3600);
        let mut clients = h.connected.lock().unwrap();
        let limits = &mut clients.get_mut(&h.addr).unwrap().rate_limits;
        while limits.check(RateCategory::Chat, later) == RateDecision::Allowed {}
    }
    for _ in 0..3 {
        dispatch_say("after").await;
    }
    assert_eq!(
        h.forwarded(),
        vec!["before".to_string()],
        "the arm must consult the chat bucket"
    );
}
