use super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

/// Domain A (fan-out byte test): one AoI event (entity 200 leaves) routed
/// to two witnesses lands as exactly two packets — one per witness — each
/// addressed to that witness's `entity_to_addr` mapping, with byte-exact
/// (and, since the witnesses share default key+seq state, identical)
/// payloads.
///
/// This is the witness_id → addr → bytes mapping that was uninspectable
/// before the `Transport` trait: it catches witness-list amplification
/// (N±1 sends) via the `len`/`send_count_to` assertions, and wrong-
/// recipient routes via the per-index addr assertions.
#[tokio::test]
async fn left_aoi_fans_out_one_packet_per_witness_to_each_addr() {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();

    let entity_id = 200u32; // the entity leaving AoI
    let witness_a = 100u32;
    let witness_b = 101u32;
    let addr_a: SocketAddr = "127.0.0.1:50100".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:50101".parse().unwrap();

    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
        Arc::new(Mutex::new(HashMap::from([
            (witness_a, addr_a),
            (witness_b, addr_b),
        ])));
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([
            (addr_a, test_default_connected_client_state()),
            (addr_b, test_default_connected_client_state()),
        ])));

    // Cell fans out one LeftAoI per witness; the base routes each.
    left_aoi(
        witness_a,
        entity_id,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    left_aoi(
        witness_b,
        entity_id,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let sent = transport.drain();
    assert_eq!(
        sent.len(),
        2,
        "exactly one send per witness, no amplification"
    );
    assert_eq!(
        transport.send_count_to(addr_a),
        0,
        "drain consumed the records"
    );
    assert_eq!(sent[0].0, addr_a, "witness A's packet routed to A's addr");
    assert_eq!(sent[1].0, addr_b, "witness B's packet routed to B's addr");

    // Both witnesses start at next_seq=0 with the all-zero key, so the
    // leave packet is byte-identical for each.
    let expected = build_entity_leave(
        &[0u8; 32],
        0,
        &[],
        entity_id,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(sent[0].1, expected, "witness A leave bytes (seq 0)");
    assert_eq!(sent[1].1, expected, "witness B leave bytes (seq 0)");
}

/// Failure side of a leave: a witness with no `entity_to_addr` mapping
/// gets no packet, so `left_aoi` must WARN `phase=leave` naming the
/// leaving `entity_id`. Reverting the outcome plumbing or the WARN
/// trips this guard.
#[tokio::test]
async fn left_aoi_unmapped_witness_warns_leave_send_failure() {
    let capture = LogCapture::install();
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));

    left_aoi(999, 200, &dyn_transport, &connected, &entity_to_addr).await;

    assert_eq!(
        transport.len(),
        0,
        "no leave packet leaves the transport when the witness addr is unresolved"
    );
    let failed = capture
        .find_message(Level::WARN, "NOT delivered to witness")
        .expect("leave send failure must WARN");
    assert!(
        failed.has_field("phase", "leave"),
        "failure WARN names the leave phase: {failed:#?}"
    );
    assert!(
        failed.has_field("entity_id", "200"),
        "failure WARN carries the leaving entity_id: {failed:#?}"
    );
    assert!(
        failed.has_field("reason", "entity_to_addr_miss"),
        "failure WARN carries the miss reason: {failed:#?}"
    );
}

/// Build the single-witness IO triple used by the create-emit seam
/// tests: one witness wired into both maps with a fresh client state.
fn single_witness_io(
    witness_id: u32,
    addr: SocketAddr,
) -> (
    Arc<TestTransport>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let transport = Arc::new(TestTransport::new());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(witness_id, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    (transport, connected, entity_to_addr)
}

/// Success-side seam: `entered_aoi` emits the `aoi.create_emit` DEBUG
/// event for BOTH the `create_base` and `cascade` phases when the
/// witness addr resolves. This is the success-path visibility the
/// invisible-static-NPC (Castle_CellBlock GuardBody corpse, class_id=0)
/// bug needs — the two reliable sends previously discarded their
/// outcomes, so a delivered CREATE/CASCADE was unobservable. Reverting
/// the `log_create_emit` calls (or downgrading them below DEBUG) trips
/// this guard.
#[tokio::test]
async fn entered_aoi_emits_create_and_cascade_success_seams() {
    let capture = LogCapture::install();

    let witness_id = 100u32;
    let addr: SocketAddr = "127.0.0.1:50200".parse().unwrap();
    let (transport, connected, entity_to_addr) = single_witness_io(witness_id, addr);
    let dyn_transport: Arc<dyn Transport> = transport.clone();

    // class_id = 0 is the GuardBody corpse class from the colo repro.
    entered_aoi(
        witness_id,
        777, // entity_id
        0,   // class_id (corpse)
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0],
        1, // level
        None,
        None,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    // Both packets actually hit the wire (the bug is about delivery).
    assert_eq!(
        transport.send_count_to(addr),
        2,
        "create_base + cascade both sent to the witness"
    );

    let base = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("phase", "create_base") && c.level == Level::DEBUG)
        .expect("create_base DEBUG seam must fire");
    assert!(
        base.has_field("phase", "create_base"),
        "create_base seam carries phase=create_base: {base:#?}"
    );
    assert!(
        base.has_field("addr_resolved", "true"),
        "create_base seam carries addr_resolved=true: {base:#?}"
    );
    assert!(
        base.has_field("class_id", "0"),
        "create_base seam carries the corpse class_id: {base:#?}"
    );

    let cascade = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("phase", "cascade") && c.level == Level::DEBUG)
        .expect("cascade DEBUG seam must fire");
    assert!(
        cascade.has_field("addr_resolved", "true"),
        "cascade seam carries addr_resolved=true: {cascade:#?}"
    );
    assert!(
        cascade.has_field("entity_id", "777"),
        "cascade seam carries the entity_id: {cascade:#?}"
    );
}

/// Failure-side seam: when the witness addr is missing from
/// `entity_to_addr`, `entered_aoi` emits the `aoi.create_send_failed`
/// WARN with `reason=entity_to_addr_miss` for the (failed) create_base
/// packet — the negative-logging seam that names the invisible-corpse
/// drop. Reverting the helper-return plumbing or the WARN seam trips
/// this guard.
#[tokio::test]
async fn entered_aoi_emits_failure_seam_on_missing_witness_addr() {
    let capture = LogCapture::install();

    // entity_to_addr is EMPTY — the witness has no resolvable address.
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> = Arc::new(Mutex::new(HashMap::new()));

    entered_aoi(
        999, // witness_id with no addr mapping
        777, // entity_id
        0,   // class_id (corpse)
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        1,
        None,
        None,
        &dyn_transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(
        transport.len(),
        0,
        "no packet leaves the transport when the witness addr is unresolved"
    );

    let failed = capture
        .find_event(Level::WARN, "AoI create emit", "entity_to_addr_miss")
        .expect("create_send_failed WARN seam must fire with reason=entity_to_addr_miss");
    assert!(
        failed.has_field("addr_resolved", "false"),
        "failure seam reports addr_resolved=false: {failed:#?}"
    );
    assert!(
        failed.has_field("phase", "create_base"),
        "failure seam names the failing phase: {failed:#?}"
    );
}
