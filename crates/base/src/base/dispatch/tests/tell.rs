//! SS-C1: tells on the base (type 8 fan-out against `TestTransport`, type 12
//! for the refusals).
//!
//! Three sessions: Alice (sender), Bob (recipient), Carol (bystander). Each
//! test sends one `sendPlayerCommunication` on the tell channel from Alice
//! and asserts, per session, exactly which client methods arrived, and that
//! nothing reached the cell.

use std::time::Instant;

use super::super::chat::send_player_communication_at;
use super::super::tell::{
    after_resolve_hook, ambiguous_text, not_online_text, TELL_CHANNEL, TELL_NO_TARGET_TEXT,
    TELL_SELF_TEXT,
};
use super::super::*;
use crate::base::contact_list::ignore::{not_accepting_text, IgnoreCache};
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_wire::cell::client_methods::communicator::{ON_PLAYER_COMMUNICATION, ON_TELL_SENT};

const ALICE: (u16, i32, u32) = (54800, 801, 8001);
const BOB: (u16, i32, u32) = (54801, 802, 8002);
const CAROL: (u16, i32, u32) = (54802, 803, 8003);

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// One decoded client method: `(entity_id, method_index, args)`.
#[derive(Debug)]
struct Sent {
    entity_id: u32,
    method: u16,
    args: Vec<u8>,
}

fn decode(packet: &[u8]) -> Sent {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    assert!(body[0] & 0x80 != 0, "direct-encoded entity method expected");
    Sent {
        entity_id: u32::from_le_bytes(body[3..7].try_into().unwrap()),
        method: u16::from(body[0] & 0x7F),
        args: body[7..].to_vec(),
    }
}

fn read_wstr(args: &[u8], offset: &mut usize) -> String {
    let n = u32::from_le_bytes(args[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([args[*offset + i * 2], args[*offset + i * 2 + 1]]))
        .collect();
    *offset += n * 2;
    String::from_utf16(&units).unwrap()
}

/// `onPlayerCommunication` args as `(speaker, flags, channel, text)`.
fn player_comm(args: &[u8]) -> (String, u8, u8, String) {
    let mut o = 0;
    let speaker = read_wstr(args, &mut o);
    let (flags, channel) = (args[o], args[o + 1]);
    o += 2;
    (speaker, flags, channel, read_wstr(args, &mut o))
}

/// `onTellSent` args as `(target, text)`.
fn tell_sent(args: &[u8]) -> (String, String) {
    let mut o = 0;
    let target = read_wstr(args, &mut o);
    (target, read_wstr(args, &mut o))
}

struct Harness {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Harness {
    fn new(sessions: &[(&str, (u16, i32, u32))]) -> Self {
        let mut map = HashMap::new();
        for (name, (port, player_id, eid)) in sessions {
            let mut s = test_default_connected_client_state();
            s.player_name = Some(name.to_string());
            s.active_player_id = Some(*player_id);
            s.player_entity_id = Some(*eid);
            s.listed_online = true;
            map.insert(addr(*port), s);
        }
        let transport = Arc::new(TestTransport::default());
        let (tx, rx) = mpsc::channel(8);
        Self {
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(map)),
            cell_tx: Some(tx),
            cell_rx: rx,
        }
    }

    fn three() -> Self {
        Self::new(&[("Alice", ALICE), ("Bob", BOB), ("Carol", CAROL)])
    }

    /// Edit one session's state in place.
    fn edit(&self, who: (u16, i32, u32), f: impl FnOnce(&mut ConnectedClientState)) {
        f(self
            .connected
            .lock()
            .unwrap()
            .get_mut(&addr(who.0))
            .unwrap());
    }

    async fn alice_tells(&self, target: &str, text: &str) {
        let mut payload = vec![TELL_CHANNEL];
        crate::mercury::write_wstring(&mut payload, target);
        crate::mercury::write_wstring(&mut payload, text);
        send_player_communication_at(
            &payload,
            &Some("Alice".to_string()),
            addr(ALICE.0),
            &self.dyn_transport,
            &self.connected,
            &self.cell_tx,
            Instant::now(),
        )
        .await;
    }

    fn to(&self, who: (u16, i32, u32)) -> Vec<Sent> {
        self.transport
            .filter_to(addr(who.0))
            .iter()
            .map(|p| decode(p))
            .collect()
    }

    /// The single feedback line Alice got, asserting it is the only packet.
    fn alice_feedback(&self) -> String {
        let sent = self.to(ALICE);
        assert_eq!(sent.len(), 1, "exactly one feedback line: {sent:?}");
        assert_eq!(sent[0].method, ON_PLAYER_COMMUNICATION);
        let (speaker, _, channel, text) = player_comm(&sent[0].args);
        assert_eq!((speaker.as_str(), channel), ("SYSTEM", 9));
        text
    }

    fn assert_never_forwarded(&mut self) {
        assert!(
            self.cell_rx.try_recv().is_err(),
            "a tell is handled on the base and never reaches the cell"
        );
    }
}

/// Type 8: the tell reaches exactly Bob (on the tell channel, spoken by
/// Alice), Alice gets `onTellSent(Bob, text)`, Carol gets nothing.
#[tokio::test]
async fn tell_reaches_exactly_one_recipient() {
    let capture = LogCapture::install();
    let mut h = Harness::three();
    h.alice_tells("Bob", "psst").await;

    let bob = h.to(BOB);
    assert_eq!(bob.len(), 1, "Bob gets exactly one packet: {bob:?}");
    assert_eq!(bob[0].entity_id, BOB.2);
    assert_eq!(bob[0].method, ON_PLAYER_COMMUNICATION);
    assert_eq!(
        player_comm(&bob[0].args),
        ("Alice".to_string(), 0, TELL_CHANNEL, "psst".to_string())
    );

    let alice = h.to(ALICE);
    assert_eq!(alice.len(), 1, "Alice gets exactly onTellSent: {alice:?}");
    assert_eq!(alice[0].entity_id, ALICE.2);
    assert_eq!(alice[0].method, ON_TELL_SENT);
    assert_eq!(
        tell_sent(&alice[0].args),
        ("Bob".to_string(), "psst".to_string())
    );

    assert!(h.to(CAROL).is_empty(), "the bystander gets nothing");
    h.assert_never_forwarded();

    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.tell_delivered"))
        .expect("chat.tell_delivered logged");
    assert!(ev.has_field("player_id", "801") && ev.has_field("target_player_id", "802"));
}

/// D-SS13: a unique case-insensitive match resolves, and `onTellSent`
/// carries the canonical name.
#[tokio::test]
async fn tell_resolves_a_case_folded_name() {
    let h = Harness::three();
    h.alice_tells("bOB", "hi").await;
    assert_eq!(h.to(BOB).len(), 1);
    assert_eq!(tell_sent(&h.to(ALICE)[0].args).0, "Bob");
}

/// CAT-L-01 / D-SS15: Bob ignores Alice. Nothing reaches Bob; Alice is told
/// Bob is not accepting her messages. Fails when the Ignore check in
/// `tell::handle_tell` is removed (Bob then receives the line).
#[tokio::test]
async fn tell_to_ignoring_player_not_delivered() {
    let capture = LogCapture::install();
    let mut h = Harness::three();
    h.edit(BOB, |c| {
        c.ignore = IgnoreCache::new(["Alice".to_string()].into())
    });
    h.alice_tells("Bob", "let me in").await;

    assert!(
        h.to(BOB).is_empty(),
        "an ignoring recipient receives nothing"
    );
    assert_eq!(h.alice_feedback(), not_accepting_text("Bob"));
    assert!(h.to(CAROL).is_empty());
    h.assert_never_forwarded();
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.tell_refused"))
        .expect("chat.tell_refused logged");
    assert!(ev.has_field("reason", "recipient_ignores_sender"));
    assert!(ev.has_field("target_player_id", "802"));
}

/// D-SS15 is one-directional: Alice ignoring Bob does not stop her telling
/// him.
#[tokio::test]
async fn tell_from_player_who_ignores_recipient_is_delivered() {
    let h = Harness::three();
    h.edit(ALICE, |c| {
        c.ignore = IgnoreCache::new(["Bob".to_string()].into())
    });
    h.alice_tells("Bob", "one-way").await;
    assert_eq!(h.to(BOB).len(), 1);
}

/// Each refusal: one feedback line to Alice, nothing to anyone else, a
/// `chat.tell_refused` row with its reason.
#[tokio::test]
async fn tell_refusals_feed_back_with_reason() {
    let cases: [(&str, String, &str); 3] = [
        ("", TELL_NO_TARGET_TEXT.to_string(), "no_target"),
        ("Alice", TELL_SELF_TEXT.to_string(), "self"),
        ("Dave", not_online_text("Dave"), "not_online"),
    ];
    for (target, expected, reason) in cases {
        let capture = LogCapture::install();
        let mut h = Harness::three();
        h.alice_tells(target, "hello?").await;
        assert_eq!(h.alice_feedback(), expected, "target {target:?}");
        assert!(h.to(BOB).is_empty() && h.to(CAROL).is_empty());
        h.assert_never_forwarded();
        assert!(
            capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "chat.tell_refused") && c.has_field("reason", reason)),
            "reason {reason} logged"
        );
    }
}

/// D-SS13: two online characters that match only after case folding are
/// refused, not guessed.
#[tokio::test]
async fn tell_to_ambiguous_name_is_refused() {
    let h = Harness::new(&[("Alice", ALICE), ("Bob", BOB), ("bob", CAROL)]);
    h.alice_tells("BOB", "which one").await;
    assert_eq!(h.alice_feedback(), ambiguous_text("BOB"));
    assert!(h.to(BOB).is_empty() && h.to(CAROL).is_empty());
}

/// A recipient who is away still gets the tell; Alice gets `onTellSent`
/// and then Bob's away message on the tell channel, spoken by Bob.
#[tokio::test]
async fn tell_to_away_player_replies_with_away_message() {
    let h = Harness::three();
    h.edit(BOB, |c| c.afk_message = Some("gone fishing".to_string()));
    h.alice_tells("Bob", "you there?").await;

    assert_eq!(h.to(BOB).len(), 1, "the tell is still delivered");
    let alice = h.to(ALICE);
    assert_eq!(alice.len(), 2, "onTellSent then the away reply: {alice:?}");
    assert_eq!(alice[0].method, ON_TELL_SENT);
    assert_eq!(alice[1].method, ON_PLAYER_COMMUNICATION);
    assert_eq!(
        player_comm(&alice[1].args),
        (
            "Bob".to_string(),
            0,
            TELL_CHANNEL,
            "gone fishing".to_string()
        )
    );
}

/// A recipient in DND is flagged `SPEAKER_DND` on the auto-reply, and the
/// DND text wins over an AFK one.
#[tokio::test]
async fn tell_to_dnd_player_replies_with_dnd_message() {
    let h = Harness::three();
    h.edit(BOB, |c| {
        c.afk_message = Some("afk".to_string());
        c.dnd_message = Some("in a raid".to_string());
    });
    h.alice_tells("Bob", "hey").await;
    let alice = h.to(ALICE);
    assert_eq!(
        player_comm(&alice[1].args),
        (
            "Bob".to_string(),
            speaker_flags::DND,
            TELL_CHANNEL,
            "in a raid".to_string()
        )
    );
}

/// D-SS13 fold: Bob's Ignore entry stored as "aLICE" still blocks Alice.
/// Fails when `IgnoreCache::ignores` compares names exactly.
#[tokio::test]
async fn tell_ignore_matches_case_insensitively() {
    let h = Harness::three();
    h.edit(BOB, |c| {
        c.ignore = IgnoreCache::new(["aLICE".to_string()].into())
    });
    h.alice_tells("Bob", "hello").await;
    assert!(h.to(BOB).is_empty());
    assert_eq!(h.alice_feedback(), not_accepting_text("Bob"));
}

/// PR #893 review: gate travel can give Bob a new entity between the name
/// lookup and the send. The tell must go to the entity Bob has at send
/// time. Fails when the send uses the entity id snapshotted at lookup.
#[tokio::test]
async fn tell_is_addressed_to_the_recipients_entity_at_send_time() {
    let h = Harness::three();
    let bob = addr(BOB.0);
    after_resolve_hook::set(move |ctx| {
        ctx.connected
            .lock()
            .unwrap()
            .get_mut(&bob)
            .unwrap()
            .player_entity_id = Some(9_999);
    });
    h.alice_tells("Bob", "after the gate").await;
    after_resolve_hook::clear();

    let got = h.to(BOB);
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0].entity_id, 9_999,
        "the tell must address Bob's current entity, not the one seen at lookup"
    );
    assert_eq!(h.to(ALICE)[0].method, ON_TELL_SENT);
}

/// The same window, but Bob is mid-world-change (no player entity) when the
/// send happens: nothing reaches Bob, Alice hears he is not online, and the
/// refusal says why. Fails when the snapshotted entity id is used.
#[tokio::test]
async fn tell_to_recipient_who_left_the_world_before_the_send_is_refused() {
    let capture = LogCapture::install();
    let h = Harness::three();
    let bob = addr(BOB.0);
    after_resolve_hook::set(move |ctx| {
        ctx.connected
            .lock()
            .unwrap()
            .get_mut(&bob)
            .unwrap()
            .player_entity_id = None;
    });
    h.alice_tells("Bob", "you there?").await;
    after_resolve_hook::clear();

    assert!(h.to(BOB).is_empty(), "nothing is sent to a stale entity");
    assert_eq!(h.alice_feedback(), not_online_text("Bob"));
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.tell_refused"))
        .expect("chat.tell_refused logged");
    assert!(ev.has_field("reason", "recipient_not_in_world"));
    assert!(ev.has_field("target_player_id", "802"));
}
