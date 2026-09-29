//! `chatJoin` / `chatLeave` / a `sendPlayerCommunication` on a user channel
//! id (issue #1039), and the disconnect/logoff cleanup that empties a
//! character out of every channel it held.
//!
//! [`user_channel_registry`] is one process-wide table (like the mute
//! table), so every test here uses its own channel names *and* its own
//! entity ids -- never a name or id another test in this file might also
//! use -- so tests running in parallel cannot see each other's state.

use super::super::chat::{handle_chat_join, handle_chat_leave, send_player_communication_at};
use super::super::*;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_base_session::base::user_channels::user_channel_registry;
use cimmeria_wire::cell::chat::CHAN_CHAT;
use cimmeria_wire::cell::client_methods::communicator::{
    ON_CHAT_JOINED, ON_CHAT_LEFT, ON_PLAYER_COMMUNICATION,
};

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn session(name: &str, player_id: i32, entity_id: u32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.active_player_id = Some(player_id);
    s.player_entity_id = Some(entity_id);
    s.account_id = 777;
    s.listed_online = true;
    s
}

/// One decoded entity-method call: `(method_index, args)`. Mirrors
/// `chat_channel_mute.rs::player_comm`'s framing but keeps the method index
/// instead of assuming `ON_PLAYER_COMMUNICATION`.
fn decode_method(packet: &[u8]) -> (u16, Vec<u8>) {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    let idx = u16::from(body[0] & 0x7F);
    (idx, body[7..].to_vec())
}

fn read_wstring_arg(args: &[u8], offset: &mut usize) -> String {
    let n = u32::from_le_bytes(args[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([args[*offset + i * 2], args[*offset + i * 2 + 1]]))
        .collect();
    *offset += n * 2;
    String::from_utf16(&units).unwrap()
}

struct Harness {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
}

impl Harness {
    fn new(sessions: &[(u16, &str, i32, u32)]) -> Self {
        let transport = Arc::new(TestTransport::default());
        let (tx, _rx) = mpsc::channel(16);
        let mut connected = HashMap::new();
        let mut entity_to_addr = HashMap::new();
        for &(port, name, player_id, entity_id) in sessions {
            connected.insert(addr(port), session(name, player_id, entity_id));
            entity_to_addr.insert(entity_id, addr(port));
        }
        Self {
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(connected)),
            entity_to_addr: Arc::new(Mutex::new(entity_to_addr)),
            cell_tx: Some(tx),
        }
    }

    async fn join(&self, port: u16, channel_name: &str) {
        let mut payload = Vec::new();
        crate::mercury::write_wstring(&mut payload, channel_name);
        crate::mercury::write_wstring(&mut payload, "");
        handle_chat_join(&payload, addr(port), &self.dyn_transport, &self.connected).await;
    }

    async fn leave(&self, port: u16, display_id: u8) {
        handle_chat_leave(
            &[display_id],
            addr(port),
            &self.dyn_transport,
            &self.connected,
        )
        .await;
    }

    async fn speak(&self, port: u16, channel: u8, text: &str) {
        let mut payload = vec![channel];
        crate::mercury::write_wstring(&mut payload, "");
        crate::mercury::write_wstring(&mut payload, text);
        let name = self.connected.lock().unwrap()[&addr(port)]
            .player_name
            .clone();
        send_player_communication_at(
            &payload,
            &name,
            addr(port),
            &self.dyn_transport,
            &self.connected,
            super::super::chat::ChatRoutes {
                cell_tx: &self.cell_tx,
                entity_to_addr: &self.entity_to_addr,
                db_pool: &None,
            },
            std::time::Instant::now(),
        )
        .await;
    }

    /// Every packet sent to `port`, decoded to `(method_index, args)`.
    fn methods_to(&self, port: u16) -> Vec<(u16, Vec<u8>)> {
        self.transport
            .filter_to(addr(port))
            .iter()
            .map(|p| decode_method(p))
            .collect()
    }

    /// The `(channel_name, display_id)` of every `onChatJoined` sent to
    /// `port`.
    fn joined_events(&self, port: u16) -> Vec<(String, u8)> {
        self.methods_to(port)
            .into_iter()
            .filter(|(idx, _)| *idx == ON_CHAT_JOINED)
            .map(|(_, args)| {
                let mut o = 0;
                let name = read_wstring_arg(&args, &mut o);
                (name, args[o])
            })
            .collect()
    }

    /// The channel name of every `onChatLeft` sent to `port`.
    fn left_events(&self, port: u16) -> Vec<String> {
        self.methods_to(port)
            .into_iter()
            .filter(|(idx, _)| *idx == ON_CHAT_LEFT)
            .map(|(_, args)| {
                let mut o = 0;
                read_wstring_arg(&args, &mut o)
            })
            .collect()
    }

    /// The `(channel, text)` of every `onPlayerCommunication` sent to
    /// `port`.
    fn communication_to(&self, port: u16) -> Vec<(u8, String)> {
        self.methods_to(port)
            .into_iter()
            .filter(|(idx, _)| *idx == ON_PLAYER_COMMUNICATION)
            .map(|(_, args)| {
                let mut o = 0;
                read_wstring_arg(&args, &mut o); // speaker
                let channel = args[o + 1];
                o += 2;
                (channel, read_wstring_arg(&args, &mut o))
            })
            .collect()
    }

    /// The feedback-channel (9) texts sent to `port`.
    fn feedback_to(&self, port: u16) -> Vec<String> {
        self.communication_to(port)
            .into_iter()
            .filter(|(c, _)| *c == 9)
            .map(|(_, t)| t)
            .collect()
    }
}

/// `chatJoin` on a name nobody holds creates the channel and sends
/// `onChatJoined` with the legacy display-id mapping (`wire_id - 12`, the
/// lowest free id): no other feedback line.
#[tokio::test]
async fn join_creates_a_channel_and_sends_on_chat_joined() {
    let h = Harness::new(&[(55_600, "Alice", 0x7301_0001, 96_000)]);
    h.join(55_600, "ut-join-create").await;

    let joined = h.joined_events(55_600);
    assert_eq!(joined.len(), 1, "exactly one onChatJoined");
    let (name, display_id) = &joined[0];
    assert_eq!(name, "ut-join-create");
    // No other feedback: onChatJoined is itself the "you joined" line.
    assert!(
        h.feedback_to(55_600).is_empty(),
        "a successful join sends no separate feedback line"
    );

    let members = user_channel_registry()
        .members_if_joined(CHAN_CHAT + display_id, 96_000)
        .expect("the joiner is a member");
    assert_eq!(members, vec![96_000]);
}

/// A second `chatJoin` on the same (case-folded) name joins the channel the
/// first player created, at the same wire id -- it does not mint a second
/// channel.
#[tokio::test]
async fn second_join_by_name_reaches_the_same_channel() {
    let h = Harness::new(&[
        (55_601, "Alice", 0x7301_0002, 96_001),
        (55_602, "BOB", 0x7301_0003, 96_002),
    ]);
    h.join(55_601, "ut-join-second").await;
    h.join(55_602, "UT-JOIN-SECOND").await; // case-insensitive join

    let (name_a, id_a) = &h.joined_events(55_601)[0];
    let (name_b, id_b) = &h.joined_events(55_602)[0];
    assert_eq!(id_a, id_b, "both land on the same wire channel");
    // The first creator's spelling wins for both.
    assert_eq!(name_a, "ut-join-second");
    assert_eq!(name_b, "ut-join-second");

    let members = user_channel_registry()
        .members_if_joined(CHAN_CHAT + id_a, 96_001)
        .expect("member");
    let mut members = members;
    members.sort_unstable();
    assert_eq!(members, vec![96_001, 96_002]);
}

/// Joining a channel already joined is refused with one feedback line, and
/// no second `onChatJoined` is sent.
#[tokio::test]
async fn joining_twice_is_refused_with_feedback() {
    let h = Harness::new(&[(55_603, "Alice", 0x7301_0004, 96_003)]);
    h.join(55_603, "ut-join-twice").await;
    h.join(55_603, "ut-join-twice").await;

    assert_eq!(
        h.joined_events(55_603).len(),
        1,
        "only the first join is confirmed"
    );
    let feedback = h.feedback_to(55_603);
    assert_eq!(feedback.len(), 1, "the repeat join gets one feedback line");
    assert!(feedback[0].contains("already in channel"));
}

/// A channel name with a forbidden character is refused before any channel
/// is created -- no `onChatJoined`, one feedback line. Checks the exact
/// (unique to this test) key a validation bypass would have inserted,
/// rather than the total channel count, which other tests mutate
/// concurrently on the shared process-wide registry.
#[tokio::test]
async fn joining_with_a_bad_name_is_refused_and_creates_nothing() {
    let h = Harness::new(&[(55_604, "Alice", 0x7301_0005, 96_004)]);
    let raw_name = "ut-join-badname-guard\u{0007}"; // bell character
    h.join(55_604, raw_name).await;

    assert!(h.joined_events(55_604).is_empty());
    assert_eq!(h.feedback_to(55_604).len(), 1);
    let would_be_key = cimmeria_entity::organization::org_text::name_key(raw_name);
    assert!(
        !user_channel_registry().contains_name(&would_be_key),
        "a validation bypass would have created a channel under the raw, unvalidated name"
    );
}

/// `chatLeave` on a channel the caller is in sends `onChatLeft` and no
/// other feedback; the last member leaving deletes the channel.
#[tokio::test]
async fn leave_sends_on_chat_left_and_deletes_the_empty_channel() {
    let h = Harness::new(&[(55_605, "Alice", 0x7301_0006, 96_005)]);
    h.join(55_605, "ut-leave-solo").await;
    let (_, display_id) = h.joined_events(55_605)[0].clone();

    h.leave(55_605, display_id).await;

    let left = h.left_events(55_605);
    assert_eq!(left, vec!["ut-leave-solo".to_string()]);
    assert!(
        h.feedback_to(55_605).is_empty(),
        "a successful leave sends no separate feedback line"
    );
    assert_eq!(
        user_channel_registry().members_if_joined(CHAN_CHAT + display_id, 96_005),
        None,
        "the channel is gone"
    );
}

/// `chatLeave` naming a channel the caller never joined is refused with
/// feedback and sends no `onChatLeft`.
#[tokio::test]
async fn leave_on_a_channel_not_joined_is_refused() {
    let h = Harness::new(&[(55_606, "Alice", 0x7301_0007, 96_006)]);
    h.leave(55_606, 200).await; // an id nobody has ever joined

    assert!(h.left_events(55_606).is_empty());
    let feedback = h.feedback_to(55_606);
    assert_eq!(feedback.len(), 1);
    assert!(feedback[0].contains("not in that chat channel"));
}

/// A user-channel post reaches every member, the speaker included, and
/// nobody else. This is the "reaches every member" acceptance criterion.
#[tokio::test]
async fn post_reaches_every_member_and_nobody_else() {
    let h = Harness::new(&[
        (55_607, "Alice", 0x7301_0008, 96_007),
        (55_608, "Bob", 0x7301_0009, 96_008),
        (55_609, "Carol", 0x7301_000A, 96_009),
    ]);
    h.join(55_607, "ut-post-fanout").await;
    h.join(55_608, "ut-post-fanout").await;
    // Carol never joins -- she must not receive the line.
    let (_, display_id) = h.joined_events(55_607)[0].clone();
    let channel = CHAN_CHAT + display_id;

    h.speak(55_607, channel, "hello everyone").await;

    let alice_lines = h.communication_to(55_607);
    let bob_lines = h.communication_to(55_608);
    assert_eq!(
        alice_lines,
        vec![(channel, "hello everyone".to_string())],
        "the speaker gets their own line (no separate local echo to skip)"
    );
    assert_eq!(bob_lines, vec![(channel, "hello everyone".to_string())]);
    assert!(
        h.communication_to(55_609).is_empty(),
        "a non-member must not receive the line"
    );
}

/// Server authority: a client cannot post to a channel id it never joined,
/// even one that exists and has members. Regression guard for the
/// membership check in `post_to_user_channel` / `members_if_joined`.
#[tokio::test]
async fn post_to_a_channel_never_joined_is_refused() {
    let h = Harness::new(&[
        (55_610, "Alice", 0x7301_000B, 96_010),
        (55_611, "Mallory", 0x7301_000C, 96_011),
    ]);
    h.join(55_610, "ut-post-guard").await;
    let (_, display_id) = h.joined_events(55_610)[0].clone();
    let channel = CHAN_CHAT + display_id;

    // Mallory never joined "ut-post-guard" but sends on its wire id anyway.
    h.speak(55_611, channel, "let me in").await;

    assert!(
        h.communication_to(55_610)
            .iter()
            .all(|(c, _)| *c != channel),
        "Alice must not receive Mallory's forged post"
    );
    let feedback = h.feedback_to(55_611);
    assert_eq!(feedback.len(), 1);
    assert!(feedback[0].contains("not in that chat channel"));
}

/// A `logOff` (either variant) removes the character from every user
/// channel it held -- the disconnect-cleanup acceptance criterion. This
/// calls the real dispatch entry point, not the registry directly, so it
/// fails if the `leave_all` call in `handle_log_off` is ever removed.
async fn logoff_leaves_every_channel(disconnect: u8) {
    let entity_id = 96_100 + u32::from(disconnect);
    let player_id = 0x7301_0100 + i32::from(disconnect);
    let channel_name = format!("ut-logoff-cleanup-{disconnect}");
    let addr: SocketAddr = format!("127.0.0.1:{}", 55_700 + u16::from(disconnect))
        .parse()
        .unwrap();

    let mut leaving = test_default_connected_client_state();
    leaving.player_entity_id = Some(entity_id);
    leaving.player_name = Some("Leaving".to_string());
    leaving.active_player_id = Some(player_id);
    leaving.account_id = 777;
    leaving.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, leaving)])));

    let key = cimmeria_entity::organization::org_text::name_key(&channel_name);
    let outcome = user_channel_registry().join(&channel_name, &key, entity_id);
    assert!(matches!(
        outcome,
        cimmeria_base_session::base::user_channels::JoinOutcome::Joined { .. }
    ));

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[disconnect],
        &Some("Leaving".to_string()),
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &Some(tx),
        &entity_to_addr,
        &None,
    )
    .await
    .expect("logOff must not fail");

    // The channel had exactly one member; logOff must have emptied and
    // deleted it. Rejoining by the same name mints a fresh channel
    // (created = true) rather than finding the old membership still there.
    let key = cimmeria_entity::organization::org_text::name_key(&channel_name);
    let rejoined = user_channel_registry().join(&channel_name, &key, entity_id + 1_000_000);
    assert!(
        matches!(
            rejoined,
            cimmeria_base_session::base::user_channels::JoinOutcome::Joined { created: true, .. }
        ),
        "logOff (disconnect={disconnect}) must have left every channel; got {rejoined:?}"
    );
}

#[tokio::test]
async fn logoff_full_exit_leaves_every_user_channel() {
    logoff_leaves_every_channel(1).await;
}

#[tokio::test]
async fn logoff_to_character_select_leaves_every_user_channel() {
    logoff_leaves_every_channel(0).await;
}
