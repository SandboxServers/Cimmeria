//! SS-C3: the channel allowlist (CAT-L-03) and the GM mute (D-SS26) on
//! `sendPlayerCommunication`, both at the base, before the cell forward and
//! before a tell is delivered.
//!
//! Type 12 for every refusal (the `chat` event, its `reason` and the ids),
//! plus the forward itself and the player's feedback line. The mute table is
//! the process-wide one, so every test here uses its own `0x7300_03xx`
//! player id and lifts its mute before it returns.

use std::time::{Duration, Instant};

use super::super::chat::send_player_communication_at;
use super::super::chat_gates::{
    check_channel, SYSTEM_CHANNEL_TEXT, UNKNOWN_CHANNEL_TEXT, USER_CHANNEL_TEXT,
};
use super::super::*;
use crate::base::mutes::{mute_table, muted_text, MuteEntry};
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_base_session::base::organization::handlers::chat::ORG_CHAT_UNAVAILABLE_TEXT;
use cimmeria_wire::cell::chat::{
    CHAN_CHAT, CHAN_COMMAND, CHAN_EMOTE, CHAN_FEEDBACK, CHAN_OFFICER, CHAN_SAY, CHAN_SERVER,
    CHAN_SPLASH, CHAN_SQUAD, CHAN_TEAM, CHAN_TELL, CHAN_YELL,
};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use tracing::Level;

const SPEAKER_PORT: u16 = 54900;
const OTHER_PORT: u16 = 54901;
const SPEAKER_EID: u32 = 9100;
const OTHER_EID: u32 = 9101;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

struct Harness {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

fn session(name: &str, player_id: i32, eid: u32, access_level: u32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.active_player_id = Some(player_id);
    s.player_entity_id = Some(eid);
    s.account_id = 555;
    s.access_level = access_level;
    s.listed_online = true;
    s
}

impl Harness {
    /// The speaker, with `player_id` and `access_level`, and one bystander
    /// ("Bob") who can receive a tell.
    fn new(player_id: i32, access_level: u32) -> Self {
        let transport = Arc::new(TestTransport::default());
        let (tx, rx) = mpsc::channel(16);
        Self {
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([
                (
                    addr(SPEAKER_PORT),
                    session("Speaker", player_id, SPEAKER_EID, access_level),
                ),
                (
                    addr(OTHER_PORT),
                    session("Bob", player_id + 0x40, OTHER_EID, 0),
                ),
            ]))),
            cell_tx: Some(tx),
            cell_rx: rx,
        }
    }

    async fn speak(&self, channel: u8, target: &str, text: &str, now: Instant) {
        let mut payload = vec![channel];
        crate::mercury::write_wstring(&mut payload, target);
        crate::mercury::write_wstring(&mut payload, text);
        send_player_communication_at(
            &payload,
            &Some("Speaker".to_string()),
            addr(SPEAKER_PORT),
            &self.dyn_transport,
            &self.connected,
            super::super::chat::ChatRoutes {
                cell_tx: &self.cell_tx,
                entity_to_addr: &Arc::new(Mutex::new(HashMap::new())),
                db_pool: &None,
            },
            now,
        )
        .await;
    }

    /// Every `(channel, text)` forwarded to the cell so far.
    fn forwarded(&mut self) -> Vec<(u8, String)> {
        let mut out = Vec::new();
        while let Ok(msg) = self.cell_rx.try_recv() {
            match msg {
                BaseToCellMsg::ChatMessage { channel, text, .. } => out.push((channel, text)),
                _ => panic!("only ChatMessage is expected on the cell channel"),
            }
        }
        out
    }

    /// Every `onPlayerCommunication` to `port`, as `(channel, text)`.
    fn lines_to(&self, port: u16) -> Vec<(u8, String)> {
        self.transport
            .filter_to(addr(port))
            .iter()
            .map(|p| player_comm(p))
            .collect()
    }

    /// The feedback-channel texts to the speaker.
    fn feedback(&self) -> Vec<String> {
        self.lines_to(SPEAKER_PORT)
            .into_iter()
            .filter(|(c, _)| *c == CHAN_FEEDBACK)
            .map(|(_, t)| t)
            .collect()
    }
}

/// Decrypt one packet (zero test key); `(channel, text)` of its
/// `onPlayerCommunication`. Other methods decode as channel 255.
fn player_comm(packet: &[u8]) -> (u8, String) {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    if u16::from(body[0] & 0x7F) != ON_PLAYER_COMMUNICATION {
        return (255, String::new());
    }
    let args = &body[7..];
    let read = |o: &mut usize| {
        let n = u32::from_le_bytes(args[*o..*o + 4].try_into().unwrap()) as usize;
        *o += 4;
        let units: Vec<u16> = (0..n)
            .map(|i| u16::from_le_bytes([args[*o + i * 2], args[*o + i * 2 + 1]]))
            .collect();
        *o += n * 2;
        String::from_utf16(&units).unwrap()
    };
    let mut o = 0;
    read(&mut o);
    let channel = args[o + 1];
    o += 2;
    (channel, read(&mut o))
}

fn mute(player_id: i32, until: Instant, now: Instant) {
    mute_table().mute(
        player_id,
        MuteEntry {
            until,
            by_account_id: Some(42),
        },
        now,
    );
}

fn assert_ids(event: &crate::test_support::Captured, player_id: i32) {
    assert!(
        event.has_field("player_id", &player_id.to_string()),
        "{event:#?}"
    );
    assert!(event.has_field("account_id", "555"), "{event:#?}");
    assert!(
        event.has_field("entity_id", &SPEAKER_EID.to_string()),
        "{event:#?}"
    );
}

/// CAT-L-03: a line on server (8), feedback (9) or splash (11) is refused
/// at the base with `chat.channel_rejected reason=system_channel`, never
/// reaches the cell, and the player reads why.
#[tokio::test]
async fn chat_rejects_system_channel_at_base() {
    for channel in [CHAN_SERVER, CHAN_FEEDBACK, CHAN_SPLASH] {
        let capture = LogCapture::install();
        let mut h = Harness::new(0x7300_0310, 0);
        h.speak(channel, "", "hello", Instant::now()).await;

        assert!(
            h.forwarded().is_empty(),
            "channel {channel} reached the cell"
        );
        let event = capture
            .find_event(Level::WARN, "refused at the base", "system_channel")
            .unwrap_or_else(|| panic!("channel {channel}: chat.channel_rejected"));
        assert_eq!(event.target, "chat");
        assert!(event.has_field("event", "chat.channel_rejected"));
        assert!(event.has_field("channel", &channel.to_string()));
        assert_ids(&event, 0x7300_0310);
        assert_eq!(h.feedback(), vec![SYSTEM_CHANNEL_TEXT], "channel {channel}");
    }
}

/// CAT-L-03: an id `EChannel` does not name (7, the old Rust "server" id)
/// and every user channel (12 and up; this server registers none) is
/// refused at the base with feedback and never reaches the cell.
#[tokio::test]
async fn chat_rejects_unknown_channel_at_base() {
    for (channel, reason, text) in [
        (7u8, "unknown_channel", UNKNOWN_CHANNEL_TEXT),
        (CHAN_CHAT, "user_channel", USER_CHANNEL_TEXT),
        (255, "user_channel", USER_CHANNEL_TEXT),
    ] {
        let capture = LogCapture::install();
        let mut h = Harness::new(0x7300_0311, 0);
        h.speak(channel, "", "hello", Instant::now()).await;

        assert!(
            h.forwarded().is_empty(),
            "channel {channel} reached the cell"
        );
        let event = capture
            .find_event(Level::WARN, "refused at the base", reason)
            .unwrap_or_else(|| panic!("channel {channel}: chat.channel_rejected {reason}"));
        assert!(event.has_field("channel", &channel.to_string()));
        assert_ids(&event, 0x7300_0311);
        assert_eq!(h.feedback(), vec![text], "channel {channel}");
    }
}

/// The allowlist does not over-block: say, emote, yell and squad still
/// reach the cell, and team, command and officer pass the allowlist to the
/// base's organization chat (ORG-09), which never forwards them. With no
/// database here, each of those three is answered with the "unavailable"
/// line instead of the old cell "not supported yet".
#[tokio::test]
async fn chat_forwards_allowlisted_channels_to_the_cell() {
    let mut h = Harness::new(0x7300_0312, 0);
    let t0 = Instant::now();
    let channels = [
        CHAN_SAY,
        CHAN_EMOTE,
        CHAN_YELL,
        CHAN_TEAM,
        CHAN_SQUAD,
        CHAN_COMMAND,
        CHAN_OFFICER,
    ];
    for (i, channel) in channels.iter().enumerate() {
        // One second apart, so the chat bucket never limits.
        h.speak(*channel, "", "hi", t0 + Duration::from_secs(i as u64))
            .await;
    }
    let got: Vec<u8> = h.forwarded().into_iter().map(|(c, _)| c).collect();
    assert_eq!(got, vec![CHAN_SAY, CHAN_EMOTE, CHAN_YELL, CHAN_SQUAD]);
    assert_eq!(h.feedback(), vec![ORG_CHAT_UNAVAILABLE_TEXT; 3]);
}

/// ORG-09 runs after SS-C3's mute gate: a muted player's team, command or
/// officer line gets the mute line, and the organization chat never sees
/// it (no `org.chat` row, no "unavailable" line).
#[tokio::test]
async fn muted_player_org_line_refused_before_org_chat() {
    const PID: i32 = 0x7300_0318;
    let capture = LogCapture::install();
    let mut h = Harness::new(PID, 0);
    let t0 = Instant::now();
    mute(PID, t0 + Duration::from_secs(60), t0);
    for (i, channel) in [CHAN_TEAM, CHAN_COMMAND, CHAN_OFFICER].iter().enumerate() {
        h.speak(*channel, "", "hi", t0 + Duration::from_secs(i as u64))
            .await;
    }
    mute_table().unmute(PID, t0);
    assert!(h.forwarded().is_empty());
    let feedback = h.feedback();
    assert_eq!(feedback.len(), 3, "{feedback:?}");
    assert!(
        !feedback.iter().any(|t| t == ORG_CHAT_UNAVAILABLE_TEXT),
        "the org chat ran before the mute gate: {feedback:?}"
    );
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "org.chat")),
        "org.chat row for a muted line"
    );
    assert_eq!(
        capture
            .all()
            .iter()
            .filter(|c| c.has_field("event", "chat.muted_refused"))
            .count(),
        3
    );
}

/// The allowlist accepts exactly the channels a player may use, by byte:
/// the tell route shares `CHAN_TELL` with it, so tell 10 is here and the
/// old tell 9 (now feedback) is not. The `EChannel` pin itself is
/// `cimmeria_wire::cell::chat::tests::chan_constants_match_enumerations_xml`.
#[test]
fn allowlist_matches_the_tell_route() {
    let allowed: Vec<u8> = (0..=255u8).filter(|c| check_channel(*c).is_ok()).collect();
    assert_eq!(allowed, vec![0, 1, 2, 3, 4, 5, 6, 10]);
}

/// D-SS26 on the chat path, on the injected clock: a muted player's say
/// line is refused with `chat.muted_refused reason=muted` and the time left,
/// and is forwarded again once the mute has expired.
#[tokio::test]
async fn muted_player_spatial_chat_refused_until_expiry() {
    const PID: i32 = 0x7300_0313;
    let capture = LogCapture::install();
    let mut h = Harness::new(PID, 0);
    let t0 = Instant::now();
    mute(PID, t0 + Duration::from_secs(300), t0);

    h.speak(CHAN_SAY, "", "let me talk", t0).await;
    assert!(
        h.forwarded().is_empty(),
        "a muted line must not reach the cell"
    );
    let event = capture
        .find_event(Level::DEBUG, "the speaker is muted", "muted")
        .expect("chat.muted_refused at DEBUG");
    assert_eq!(event.target, "chat");
    assert!(event.has_field("event", "chat.muted_refused"));
    assert!(event.has_field("remaining_secs", "300"));
    assert!(event.has_field("muted_by_account_id", "42"));
    assert_ids(&event, PID);
    assert_eq!(h.feedback(), vec![muted_text(Duration::from_secs(300))]);

    h.speak(CHAN_SAY, "", "free", t0 + Duration::from_secs(300))
        .await;
    assert_eq!(
        h.forwarded(),
        vec![(CHAN_SAY, "free".to_string())],
        "forwarded again at the expiry"
    );
    mute_table().unmute(PID, t0);
}

/// D-SS26: a muted sender's tell is refused before the recipient lookup:
/// the recipient gets nothing, the sender gets no `onTellSent`, only the
/// mute line.
#[tokio::test]
async fn muted_player_tell_not_delivered() {
    const PID: i32 = 0x7300_0314;
    let capture = LogCapture::install();
    let mut h = Harness::new(PID, 0);
    let t0 = Instant::now();
    mute(PID, t0 + Duration::from_secs(120), t0);

    h.speak(CHAN_TELL, "Bob", "psst", t0).await;

    assert!(
        h.transport.filter_to(addr(OTHER_PORT)).is_empty(),
        "Bob gets nothing"
    );
    assert_eq!(
        h.transport.filter_to(addr(SPEAKER_PORT)).len(),
        1,
        "only the mute line reaches the sender, no onTellSent"
    );
    assert_eq!(h.feedback(), vec![muted_text(Duration::from_secs(120))]);
    assert!(h.forwarded().is_empty());
    let event = capture
        .find_event(Level::DEBUG, "the speaker is muted", "muted")
        .expect("chat.muted_refused");
    assert!(event.has_field("tell", "true"));
    mute_table().unmute(PID, t0);
}

/// The mute is keyed by `player_id`, not the session: the same character on
/// a new session (a relog) is still muted.
#[tokio::test]
async fn mute_holds_across_relog() {
    const PID: i32 = 0x7300_0315;
    let t0 = Instant::now();
    mute(PID, t0 + Duration::from_secs(600), t0);
    {
        let mut first = Harness::new(PID, 0);
        first.speak(CHAN_SAY, "", "one", t0).await;
        assert!(first.forwarded().is_empty());
    }
    // A new session map, a new address: nothing carries over but the id.
    let mut relogged = Harness::new(PID, 0);
    relogged
        .speak(CHAN_SAY, "", "two", t0 + Duration::from_secs(60))
        .await;
    assert!(relogged.forwarded().is_empty(), "still muted after relog");
    assert_eq!(
        relogged.feedback(),
        vec![muted_text(Duration::from_secs(540))]
    );
    mute_table().unmute(PID, t0);
}

/// GameMaster and above are never refused by the mute gate: they run the
/// `.` console over say, and `.mute` refuses a GM target anyway.
#[tokio::test]
async fn mute_gate_skips_gm_speakers() {
    const PID: i32 = 0x7300_0316;
    let mut h = Harness::new(PID, 2);
    let t0 = Instant::now();
    mute(PID, t0 + Duration::from_secs(600), t0);

    h.speak(CHAN_SAY, "", ".help", t0).await;
    assert_eq!(h.forwarded(), vec![(CHAN_SAY, ".help".to_string())]);
    mute_table().unmute(PID, t0);
}

/// A muted recipient still receives a tell, but their AFK / DND text is not
/// sent back to the teller while the mute lasts: it is their words too
/// (SS-C3 review). The teller gets `onTellSent` and nothing else.
#[tokio::test]
async fn muted_recipient_away_reply_withheld() {
    const PID: i32 = 0x7300_0317;
    let bob_pid = PID + 0x40;
    let capture = LogCapture::install();
    let h = Harness::new(PID, 0);
    h.connected
        .lock()
        .unwrap()
        .get_mut(&addr(OTHER_PORT))
        .unwrap()
        .dnd_message = Some("buy gold at example dot com".to_string());
    let t0 = Instant::now();
    mute(bob_pid, t0 + Duration::from_secs(600), t0);

    h.speak(CHAN_TELL, "Bob", "hi", t0).await;

    assert_eq!(
        h.lines_to(OTHER_PORT),
        vec![(CHAN_TELL, "hi".to_string())],
        "Bob still gets the tell"
    );
    let to_speaker = h.lines_to(SPEAKER_PORT);
    assert_eq!(
        to_speaker.len(),
        1,
        "only onTellSent, no away reply: {to_speaker:?}"
    );
    assert_eq!(to_speaker[0].0, 255, "the one packet is onTellSent");
    let event = capture
        .find_message(Level::INFO, "tell delivered")
        .expect("chat.tell_delivered");
    assert!(event.has_field("away_reply", "false"));
    assert!(event.has_field("away_reply_withheld_muted", "true"));
    mute_table().unmute(bob_pid, t0);
}
