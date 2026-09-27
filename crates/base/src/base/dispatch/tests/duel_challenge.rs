//! SS-D1: `sendDuelChallenge` (0xD9) on the base. The duel bucket
//! (D-SS21), squad refusal, the online lookup (D-SS13), and the forward,
//! whose ids come from the session map and never from the payload.
//!
//! Type 12 (negative log) plus the forward: each refusal asserts its
//! `duel.challenge_refused` / `rate_limit.exceeded` event, that nothing
//! reached the cell, and the one line the challenger sees.

use std::time::{Duration, Instant};

use cimmeria_wire::cell::client_methods::duel::{
    TEXT_CHALLENGER_LOADING, TEXT_CHALLENGE_UNDELIVERED, TEXT_SQUAD_DUEL_UNSUPPORTED,
    TEXT_TARGET_AMBIGUOUS, TEXT_TARGET_LOADING, TEXT_TARGET_NOT_ONLINE,
};
use cimmeria_wire::mercury::types::WorldEntryInfo;

use super::super::duel::send_duel_challenge_at;
use super::super::*;
use crate::base::rate_limit::RateCategory;
use crate::cell::messages::DuelBaseToCell;
use crate::test_support::{
    test_default_connected_client_state, LogCapture, LogCaptureGuard, TestTransport,
};
use tracing::Level;

const CHALLENGER: &str = "127.0.0.1:54800";
const TARGET: &str = "127.0.0.1:54801";
const CHALLENGER_EID: u32 = 4100;
const TARGET_EID: u32 = 4200;

struct Harness {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

fn session(name: &str, player_id: i32, entity_id: u32, account_id: u32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.active_player_id = Some(player_id);
    s.player_entity_id = Some(entity_id);
    s.account_id = account_id;
    s.listed_online = true;
    s
}

impl Harness {
    /// "Lomiada" (player 7, account 70) challenges; "Teal'c" (player 8) is
    /// online.
    fn new() -> Self {
        let addr: SocketAddr = CHALLENGER.parse().unwrap();
        let target: SocketAddr = TARGET.parse().unwrap();
        let transport = Arc::new(TestTransport::default());
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(16);
        Self {
            addr,
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([
                (addr, session("Lomiada", 7, CHALLENGER_EID, 70)),
                (target, session("Teal'c", 8, TARGET_EID, 80)),
            ]))),
            cell_tx: Some(tx),
            cell_rx: rx,
        }
    }

    async fn challenge(&self, name: &str, squad: u8, now: Instant) {
        let mut payload = Vec::new();
        crate::mercury::write_wstring(&mut payload, name);
        payload.push(squad);
        self.raw(&payload, now).await;
    }

    async fn raw(&self, payload: &[u8], now: Instant) {
        send_duel_challenge_at(
            payload,
            self.addr,
            &self.dyn_transport,
            &self.connected,
            &self.cell_tx,
            now,
        )
        .await;
    }

    fn forwarded(&mut self) -> Vec<DuelBaseToCell> {
        let mut out = Vec::new();
        while let Ok(msg) = self.cell_rx.try_recv() {
            match msg {
                BaseToCellMsg::Duel(d) => out.push(d),
                _ => panic!("only Duel is expected on the cell channel"),
            }
        }
        out
    }

    /// Every feedback line sent to the challenger, decoded.
    fn feedback(&self) -> Vec<String> {
        self.transport
            .filter_to(self.addr)
            .iter()
            .map(|p| decode_feedback_text(p))
            .collect()
    }
}

/// Decrypt one `onPlayerCommunication` packet (all-zero test key), check it
/// is addressed to the challenger's entity, and return its text.
fn decode_feedback_text(packet: &[u8]) -> String {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    assert_eq!(
        u32::from_le_bytes(body[3..7].try_into().unwrap()),
        CHALLENGER_EID
    );
    let args = &body[7..];
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let mut offset = 4 + speaker_len * 2 + 2;
    let text_len = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    let units: Vec<u16> = (0..text_len)
        .map(|i| u16::from_le_bytes(args[offset + i * 2..offset + i * 2 + 2].try_into().unwrap()))
        .collect();
    String::from_utf16(&units).unwrap()
}

fn refused(capture: &LogCaptureGuard, reason: &str) -> bool {
    capture.all().iter().any(|c| {
        c.target == "duel"
            && c.has_field("event", "duel.challenge_refused")
            && c.has_field("reason", reason)
            && c.has_field("player_id", "7")
            && c.has_field("account_id", "70")
    })
}

/// A `duel.challenge_refused` row with `reason` that names the target.
fn refused_naming(capture: &LogCaptureGuard, reason: &str, target_player_id: &str) -> bool {
    capture.all().iter().any(|c| {
        c.has_field("event", "duel.challenge_refused")
            && c.has_field("reason", reason)
            && c.has_field("target_player_id", target_player_id)
    })
}

/// The forward carries both players' ids from the session map. The payload
/// holds only a name (typed in the wrong case here, resolved per D-SS13)
/// and the squad byte.
#[tokio::test]
async fn challenge_forwards_session_ids_to_the_cell() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.challenge("teal'c", 0, Instant::now()).await;
    assert_eq!(
        h.forwarded(),
        vec![DuelBaseToCell::Challenge {
            player_id: 7,
            entity_id: CHALLENGER_EID,
            account_id: 70,
            target_player_id: 8,
            target_entity_id: TARGET_EID,
        }]
    );
    assert!(
        h.feedback().is_empty(),
        "the cell answers a forwarded challenge"
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "duel.challenge_forwarded")
            && c.has_field("target_player_id", "8")));
}

/// D-SS21: burst 2, one more every 15 s. The third challenge inside the
/// window is dropped before the lookup and the forward, logs
/// `rate_limit.exceeded category=duel_challenge` at WARN, and sends one line.
#[tokio::test]
async fn challenge_rate_limited() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    let t0 = Instant::now();
    h.challenge("Teal'c", 0, t0).await;
    h.challenge("Teal'c", 0, t0 + Duration::from_secs(1)).await;
    h.challenge("Teal'c", 0, t0 + Duration::from_secs(2)).await;
    assert_eq!(
        h.forwarded().len(),
        2,
        "the third challenge must not reach the cell"
    );
    assert_eq!(
        h.feedback(),
        vec![RateCategory::DuelChallenge.feedback_text().to_string()]
    );
    let ev = capture
        .find_event(Level::WARN, "rate_limit.exceeded", "bucket_empty")
        .expect("rate_limit.exceeded WARN");
    assert!(ev.has_field("category", "duel_challenge"));
    assert!(ev.has_field("player_id", "7"));

    // One token back after 15 s.
    h.challenge("Teal'c", 0, t0 + Duration::from_secs(15)).await;
    assert_eq!(h.forwarded().len(), 1);
}

/// Squad duels are refused at the base with a line, never forwarded.
#[tokio::test]
async fn challenge_rejects_squad_duel() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.challenge("Teal'c", 1, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_SQUAD_DUEL_UNSUPPORTED.to_string()]);
    assert!(refused(&capture, "squad_duel"));
}

/// No online character by that name: a line, no forward.
#[tokio::test]
async fn challenge_rejects_offline_target() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.challenge("Daniel", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_TARGET_NOT_ONLINE.to_string()]);
    assert!(refused(&capture, "target_not_online"));
}

/// Two characters fold to the typed name (D-SS13): refused, not guessed.
#[tokio::test]
async fn challenge_rejects_ambiguous_target() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.connected.lock().unwrap().insert(
        "127.0.0.1:54802".parse().unwrap(),
        session("teal'C", 9, 4300, 90),
    );
    h.challenge("TEAL'C", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_TARGET_AMBIGUOUS.to_string()]);
    assert!(refused(&capture, "target_ambiguous"));
}

/// A world-entry step in flight: what gate travel sets on a listed session.
fn loading_entry(entity_id: u32) -> WorldEntryInfo {
    WorldEntryInfo {
        player_entity_id: entity_id,
        space_id: 1,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "Agnos".into(),
        class_id: 2,
        world_stargates: Vec::new(),
    }
}

/// A challenger still entering the world (before `onClientReady`, or mid
/// gate travel with the listing kept) is refused before the lookup.
#[tokio::test]
async fn challenge_rejects_a_challenger_still_loading() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.connected
        .lock()
        .unwrap()
        .get_mut(&h.addr)
        .unwrap()
        .pending_map_loaded = Some(loading_entry(CHALLENGER_EID));
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_CHALLENGER_LOADING.to_string()]);
    assert!(refused(&capture, "challenger_loading"));

    // Not yet listed (before the first `onClientReady`) is not ready either.
    let mut h = Harness::new();
    {
        let mut clients = h.connected.lock().unwrap();
        let c = clients.get_mut(&h.addr).unwrap();
        c.listed_online = false;
    }
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
}

/// A target mid gate travel stays listed but is not client-ready: refused
/// at the base, never prompted.
#[tokio::test]
async fn challenge_rejects_a_target_still_loading() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    let target: SocketAddr = TARGET.parse().unwrap();
    h.connected
        .lock()
        .unwrap()
        .get_mut(&target)
        .unwrap()
        .pending_world_entry = Some(loading_entry(TARGET_EID));
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_TARGET_LOADING.to_string()]);
    assert!(refused(&capture, "target_loading"));
    assert!(
        refused_naming(&capture, "target_loading", "8"),
        "the refusal names the resolved target"
    );
}

/// The name resolved to a listed session with no player entity: refused as
/// not online, and the row still names the resolved target.
#[tokio::test]
async fn challenge_rejects_a_target_not_in_world_and_names_it() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    let target: SocketAddr = TARGET.parse().unwrap();
    h.connected
        .lock()
        .unwrap()
        .get_mut(&target)
        .unwrap()
        .player_entity_id = None;
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert!(h.forwarded().is_empty());
    assert_eq!(h.feedback(), vec![TEXT_TARGET_NOT_ONLINE.to_string()]);
    assert!(refused_naming(&capture, "target_not_in_world", "8"));
}

/// A session with no player in the world still spends duel tokens: the
/// bucket is taken before the identity check, so a flood of 0xD9 from
/// character select is limited like any other (Copilot on #888). Three
/// calls: two `not_in_world` refusals, then a `rate_limit.exceeded` drop.
#[tokio::test]
async fn out_of_world_flood_is_rate_limited() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    {
        let mut clients = h.connected.lock().unwrap();
        let c = clients.get_mut(&h.addr).unwrap();
        c.player_entity_id = None;
        c.listed_online = false;
    }
    let t0 = Instant::now();
    for i in 0..3u64 {
        h.challenge("Teal'c", 0, t0 + Duration::from_millis(i * 100))
            .await;
    }
    assert!(h.forwarded().is_empty());
    let not_in_world = capture
        .all()
        .iter()
        .filter(|c| {
            c.has_field("event", "duel.challenge_refused") && c.has_field("reason", "not_in_world")
        })
        .count();
    assert_eq!(not_in_world, 2, "the third call is dropped by the bucket");
    let ev = capture
        .find_event(Level::WARN, "rate_limit.exceeded", "bucket_empty")
        .expect("rate_limit.exceeded WARN");
    assert!(ev.has_field("category", "duel_challenge"));
    assert!(ev.has_field("player_id", "7"));
    assert!(ev.has_field("account_id", "70"));
}

/// No path to the cell (the channel closed, or none at all): the challenger
/// still gets a line on the first press, the refusal is logged, and no
/// `duel.challenge_forwarded` row claims the cell has it.
#[tokio::test]
async fn challenge_with_no_cell_channel_tells_the_challenger() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    let (tx, rx) = mpsc::channel::<BaseToCellMsg>(1);
    drop(rx);
    h.cell_tx = Some(tx);
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert_eq!(h.feedback(), vec![TEXT_CHALLENGE_UNDELIVERED.to_string()]);
    assert!(refused_naming(&capture, "cell_channel_closed", "8"));

    let mut h = Harness::new();
    h.cell_tx = None;
    h.challenge("Teal'c", 0, Instant::now()).await;
    assert_eq!(h.feedback(), vec![TEXT_CHALLENGE_UNDELIVERED.to_string()]);
    assert!(refused_naming(&capture, "no_cell_channel", "8"));

    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "duel.challenge_forwarded")),
        "nothing reached the cell, so nothing may log challenge_forwarded"
    );
}

/// A payload that does not decode is logged at WARN and not answered.
#[tokio::test]
async fn challenge_malformed_payload_is_dropped() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    h.raw(&[0xFF, 0xFF, 0xFF, 0xFF, 0x41, 0], Instant::now())
        .await;
    assert!(h.forwarded().is_empty());
    assert!(h.feedback().is_empty());
    assert!(capture
        .find_event(Level::WARN, "did not decode", "truncated")
        .is_some());
}

/// The 0xD9 arm routes to the duel handler: through the real dispatcher,
/// a challenge reaches the cell and does not hit the unhandled-method WARN.
#[tokio::test]
async fn dispatch_routes_0xd9_to_the_duel_handler() {
    let capture = LogCapture::install();
    let mut h = Harness::new();
    let mut payload = Vec::new();
    crate::mercury::write_wstring(&mut payload, "Teal'c");
    payload.push(0);
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    dispatch_sgw_player_base_method(
        0xD9,
        &payload,
        &Some("Lomiada".to_string()),
        h.addr,
        &h.dyn_transport,
        [0u8; 32],
        &h.connected,
        &entity_manager,
        &h.cell_tx,
        &entity_to_addr,
        &None,
    )
    .await
    .unwrap();
    assert_eq!(h.forwarded().len(), 1);
    assert!(capture
        .find_message(Level::WARN, "Unhandled SGWPlayer base method")
        .is_none());
}
