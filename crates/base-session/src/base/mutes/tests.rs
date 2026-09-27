//! `MuteTable` on an injected clock, and the `.mute` / `.unmute` handlers
//! (SS-C3, D-SS26).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::transport::Transport;
use tracing::Level;

use super::gm::{apply_gm_mute, apply_gm_unmute, GmActor, GmMuteCtx, MuteOutcome};
use super::*;
use crate::base::feedback::FeedbackCtx;
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

fn entry(until: Instant) -> MuteEntry {
    MuteEntry {
        until,
        by_account_id: Some(1),
    }
}

/// D-SS26 acceptance: a mute holds until its expiry on the injected clock
/// and not a moment longer; the expired entry is removed on that check.
#[test]
fn mute_expires_on_injected_clock() {
    let table = MuteTable::new();
    let t0 = Instant::now();
    assert_eq!(
        table.mute(7, entry(t0 + Duration::from_secs(600)), t0),
        None
    );

    let at_start = table.active(7, t0).expect("muted at t0");
    assert_eq!(at_start.until, t0 + Duration::from_secs(600));
    assert!(table.active(7, t0 + Duration::from_secs(599)).is_some());
    assert!(
        table.active(7, t0 + Duration::from_secs(600)).is_none(),
        "a mute ends exactly at its expiry"
    );
    assert!(
        table.is_empty(),
        "the expired entry is dropped on the check"
    );
    assert!(
        table.active(8, t0).is_none(),
        "another player is never muted"
    );
}

/// Re-muting replaces the expiry and reports what the old mute had left.
#[test]
fn mute_replaces_and_reports_previous_remaining() {
    let table = MuteTable::new();
    let t0 = Instant::now();
    table.mute(7, entry(t0 + Duration::from_secs(600)), t0);
    let previous = table.mute(
        7,
        entry(t0 + Duration::from_secs(60)),
        t0 + Duration::from_secs(100),
    );
    assert_eq!(previous, Some(Duration::from_secs(500)));
    assert!(table.active(7, t0 + Duration::from_secs(61)).is_none());
}

/// Every insert sweeps expired entries, so the table holds live mutes only.
#[test]
fn mute_insert_sweeps_expired_entries() {
    let table = MuteTable::new();
    let t0 = Instant::now();
    for id in 0..10 {
        table.mute(id, entry(t0 + Duration::from_secs(1)), t0);
    }
    table.mute(
        99,
        entry(t0 + Duration::from_secs(100)),
        t0 + Duration::from_secs(2),
    );
    assert_eq!(table.len(), 1);
}

/// Unmute returns the time left, and nothing for an expired or absent mute.
#[test]
fn unmute_reports_remaining_only_for_a_live_mute() {
    let table = MuteTable::new();
    let t0 = Instant::now();
    table.mute(7, entry(t0 + Duration::from_secs(300)), t0);
    assert_eq!(
        table.unmute(7, t0 + Duration::from_secs(100)),
        Some(Duration::from_secs(200))
    );
    assert_eq!(table.unmute(7, t0), None, "already lifted");

    table.mute(8, entry(t0 + Duration::from_secs(1)), t0);
    assert_eq!(
        table.unmute(8, t0 + Duration::from_secs(5)),
        None,
        "expired"
    );
}

#[test]
fn muted_text_rounds_minutes_up() {
    assert_eq!(
        muted_text(Duration::from_secs(1)),
        "You are muted and cannot chat for another 1 minute."
    );
    assert_eq!(
        muted_text(Duration::from_secs(61)),
        "You are muted and cannot chat for another 2 minutes."
    );
}

// ── The GM handlers ────────────────────────────────────────────────────────

const GM_ADDR: &str = "127.0.0.1:55100";
const SUBJECT_ADDR: &str = "127.0.0.1:55101";
const OTHER_GM_ADDR: &str = "127.0.0.1:55102";
const GM_EID: u32 = 900;
const SUBJECT_EID: u32 = 901;
const OTHER_GM_EID: u32 = 902;
const SUBJECT_PLAYER_ID: i32 = 0x7300_0301;

struct Harness {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

fn listed(
    eid: u32,
    player_id: i32,
    account_id: u32,
    name: &str,
    access: u32,
) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_entity_id = Some(eid);
    s.active_player_id = Some(player_id);
    s.account_id = account_id;
    s.player_name = Some(name.to_string());
    s.access_level = access;
    s.listed_online = true;
    s
}

impl Harness {
    fn new() -> Self {
        let gm: SocketAddr = GM_ADDR.parse().unwrap();
        let subject: SocketAddr = SUBJECT_ADDR.parse().unwrap();
        let other: SocketAddr = OTHER_GM_ADDR.parse().unwrap();
        let transport = Arc::new(TestTransport::default());
        Self {
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([
                (gm, listed(GM_EID, 0x7300_0300, 30, "Warden", 2)),
                (
                    subject,
                    listed(SUBJECT_EID, SUBJECT_PLAYER_ID, 31, "Loudmouth", 0),
                ),
                (other, listed(OTHER_GM_EID, 0x7300_0302, 32, "Overseer", 3)),
            ]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([
                (GM_EID, gm),
                (SUBJECT_EID, subject),
                (OTHER_GM_EID, other),
            ]))),
        }
    }

    fn ctx(&self) -> GmMuteCtx<'_> {
        GmMuteCtx {
            feedback: FeedbackCtx {
                transport: &self.dyn_transport,
                connected: &self.connected,
            },
            entity_to_addr: &self.entity_to_addr,
        }
    }

    fn lines_to(&self, addr: &str) -> Vec<String> {
        self.transport
            .filter_to(addr.parse().unwrap())
            .iter()
            .map(|p| decode_text(p))
            .collect()
    }
}

const GM: GmActor = GmActor {
    entity_id: GM_EID,
    player_id: Some(0x7300_0300),
    account_id: Some(30),
};

/// The text of one `onPlayerCommunication` feedback packet (zero test key).
fn decode_text(packet: &[u8]) -> String {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let args = &pt[1..pt.len() - 4][7..];
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let mut offset = 4 + speaker_len * 2 + 2;
    let text_len = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let units: Vec<u16> = (0..text_len)
        .map(|i| u16::from_le_bytes(args[offset + i * 2..offset + i * 2 + 2].try_into().unwrap()))
        .collect();
    String::from_utf16(&units).unwrap()
}

/// `.mute` writes the table under the subject's `player_id`, logs
/// `chat.gm_mute` with the GM's ids and the subject, and tells both sides.
#[tokio::test]
async fn gm_mute_records_logs_and_tells_both_sides() {
    let capture = LogCapture::install();
    let h = Harness::new();
    let table = MuteTable::new();
    let t0 = Instant::now();

    let outcome = apply_gm_mute(&h.ctx(), &table, GM, "loudmouth", 30, "spam", t0).await;

    assert_eq!(
        outcome,
        MuteOutcome::Muted {
            subject_player_id: SUBJECT_PLAYER_ID
        }
    );
    let e = table
        .active(SUBJECT_PLAYER_ID, t0)
        .expect("the subject is muted");
    assert_eq!(e.until, t0 + Duration::from_secs(30 * 60));
    assert_eq!(e.by_account_id, Some(30));

    let event = capture
        .find_message(Level::INFO, "GM muted a player")
        .expect("chat.gm_mute at INFO");
    assert_eq!(event.target, "chat");
    for (k, v) in [
        ("event", "chat.gm_mute"),
        ("entity_id", "900"),
        ("account_id", "30"),
        ("player_id", &0x7300_0300.to_string()),
        ("subject_player_id", &SUBJECT_PLAYER_ID.to_string()),
        ("subject_account_id", "31"),
        ("duration_minutes", "30"),
        ("reason", "spam"),
        ("remaining_secs", "1800"),
    ] {
        assert!(event.has_field(k, v), "{k}={v} missing: {event:#?}");
    }
    assert_eq!(h.lines_to(GM_ADDR), vec!["Muted Loudmouth for 30 minutes."]);
    assert_eq!(
        h.lines_to(SUBJECT_ADDR),
        vec!["A GM has muted you for 30 minutes. You cannot chat or send tells until it ends."]
    );
}

/// Type 12: every `.mute` refusal logs `chat.gm_mute_refused` at WARN with
/// its `reason`, writes nothing, and tells the GM.
#[tokio::test]
async fn gm_mute_refusals_log_reason_and_leave_the_table_alone() {
    let cases: [(&str, u32, &str, &str); 4] = [
        (
            "Loudmouth",
            0,
            "bad_duration",
            ".mute: minutes must be 1 to 10080.",
        ),
        (
            "Nobody",
            5,
            "not_online",
            ".mute: Player Nobody is not online.",
        ),
        (
            "Overseer",
            5,
            "target_is_gm",
            ".mute: Overseer is a GM and cannot be muted.",
        ),
        (
            "bad\u{202e}name",
            5,
            "bad_name",
            ".mute: That is not a valid character name.",
        ),
    ];
    for (name, minutes, reason, line) in cases {
        let capture = LogCapture::install();
        let h = Harness::new();
        let table = MuteTable::new();
        let outcome = apply_gm_mute(&h.ctx(), &table, GM, name, minutes, "", Instant::now()).await;
        assert_eq!(outcome, MuteOutcome::Refused(reason));
        assert!(table.is_empty(), "{reason}: nothing is written");
        let event = capture
            .find_event(Level::WARN, "GM .mute refused", reason)
            .unwrap_or_else(|| panic!("{reason}: chat.gm_mute_refused at WARN"));
        assert!(event.has_field("event", "chat.gm_mute_refused"));
        assert!(event.has_field("account_id", "30"));
        assert!(event.has_field("entity_id", "900"));
        assert_eq!(h.lines_to(GM_ADDR), vec![line], "{reason}");
        assert!(
            h.lines_to(SUBJECT_ADDR).is_empty(),
            "{reason}: subject told nothing"
        );
    }
}

/// `.unmute` lifts a live mute and tells both sides; on a player who is not
/// muted it is refused with `reason = not_muted`.
#[tokio::test]
async fn gm_unmute_lifts_and_refuses_when_not_muted() {
    let capture = LogCapture::install();
    let h = Harness::new();
    let table = MuteTable::new();
    let t0 = Instant::now();
    table.mute(SUBJECT_PLAYER_ID, entry(t0 + Duration::from_secs(600)), t0);

    let outcome = apply_gm_unmute(&h.ctx(), &table, GM, "Loudmouth", t0).await;
    assert_eq!(
        outcome,
        MuteOutcome::Unmuted {
            subject_player_id: SUBJECT_PLAYER_ID
        }
    );
    assert!(table.active(SUBJECT_PLAYER_ID, t0).is_none());
    let event = capture
        .find_message(Level::INFO, "GM lifted a player's mute")
        .expect("chat.gm_unmute at INFO");
    assert!(event.has_field("event", "chat.gm_unmute"));
    assert!(event.has_field("subject_player_id", &SUBJECT_PLAYER_ID.to_string()));
    assert!(event.has_field("previous_remaining_secs", "600"));
    assert_eq!(h.lines_to(GM_ADDR), vec!["Unmuted Loudmouth."]);
    assert_eq!(
        h.lines_to(SUBJECT_ADDR),
        vec!["A GM has lifted your mute. You can chat again."]
    );

    let again = apply_gm_unmute(&h.ctx(), &table, GM, "Loudmouth", t0).await;
    assert_eq!(again, MuteOutcome::Refused("not_muted"));
    assert!(capture
        .find_event(Level::WARN, "GM .unmute refused", "not_muted")
        .is_some());
    assert_eq!(
        h.lines_to(GM_ADDR).last().unwrap(),
        ".unmute: Loudmouth is not muted."
    );
}
