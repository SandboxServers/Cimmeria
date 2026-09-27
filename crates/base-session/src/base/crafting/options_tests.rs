//! Tests for `onUpdateCraftingOptions` (140): the payload built from a
//! session's stations and tools, and when it is sent.

use super::*;
use crate::base::crafting::sync::{build_login_bundle, push_crafting_on_login, CraftClient};
use crate::base::crafting::tools::ToolSpec;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_mercury::encryption::EncryptionVersion;

const ENTITY: u32 = 4280;
const ACCOUNT_ID: u32 = 4281;
const PLAYER_ID: i32 = 4282;

fn tool(instance_id: i32, tech_comp: i32) -> HeldTool {
    HeldTool {
        instance_id,
        spec: ToolSpec {
            applied_science_id: 1,
            tech_comp,
        },
    }
}

fn station_and_tool_fixture() -> CraftingSessionOptions {
    CraftingSessionOptions {
        stations: [Some(900), None, Some(901), Some(902)],
        tools: vec![tool(20_001, 10), tool(20_002, 35)],
        craft_anywhere: false,
        armed: false,
        last_sent: None,
    }
}

/// The byte-exact 140 for a station + tool fixture, written out by hand
/// from `alias.xml`: four `CraftingInfo`s in the order crafting, research,
/// reverseEngineering, alloying, each `items` then `entities`, every array
/// a `u32` count then `i32`s. The best tool (tech_comp 35, instance 20002 =
/// `0x4E22`) fills every section but alloying; the stations 900-902
/// (`0x384`-`0x386`) fill their own sections.
#[test]
fn station_and_tool_fixture_is_byte_exact() {
    let args = crafting_options_args(&build_options(ENTITY, &station_and_tool_fixture()));
    #[rustfmt::skip]
    let expected: [u8; 56] = [
        // crafting: items [20002], entities [900]
        0x01, 0x00, 0x00, 0x00, 0x22, 0x4E, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x84, 0x03, 0x00, 0x00,
        // research: items [20002], entities []
        0x01, 0x00, 0x00, 0x00, 0x22, 0x4E, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        // reverseEngineering: items [20002], entities [901]
        0x01, 0x00, 0x00, 0x00, 0x22, 0x4E, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x85, 0x03, 0x00, 0x00,
        // alloying: items [] (tools never alloy), entities [902]
        0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x86, 0x03, 0x00, 0x00,
    ];
    assert_eq!(args, expected);
}

/// "Craft anywhere" names the player's own entity as the machine of every
/// section, whatever the stations (the legacy `.allcraft` shape).
#[test]
fn craft_anywhere_names_the_player_as_every_machine() {
    let mut inputs = station_and_tool_fixture();
    inputs.craft_anywhere = true;
    let options = build_options(ENTITY, &inputs);
    for section in [
        &options.crafting,
        &options.research,
        &options.reverse_engineering,
        &options.alloying,
    ] {
        assert_eq!(section.entities, vec![ENTITY as i32]);
    }
    assert!(options.alloying.items.is_empty());
}

#[test]
fn nothing_in_reach_is_all_empty() {
    assert_eq!(
        build_options(ENTITY, &CraftingSessionOptions::default()),
        CraftingOptions::default()
    );
}

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

fn session() -> (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    SocketAddr,
    Connected,
    EntityToAddr,
) {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let addr: SocketAddr = "127.0.0.1:55730".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    state.active_player_id = Some(PLAYER_ID);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    (typed, transport, addr, connected, entity_to_addr)
}

/// The 140 packet as the test transport records it: the `seq`-th reliable
/// send on the session.
fn packet(options: &CraftingOptions, seq: u32) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        seq,
        &[],
        ENTITY,
        ON_UPDATE_CRAFTING_OPTIONS,
        &crafting_options_args(options),
        EncryptionVersion::V1,
    )
}

/// The login crafting bundle as the test transport records it when it
/// carries only the options (no database, so no state): the `seq`-th
/// reliable send on the session.
fn login_packet(options: &CraftingOptions, seq: u32) -> Vec<u8> {
    let (packets, _) = build_login_bundle(ENTITY, None, Some(options)).finalize(
        cimmeria_mercury::packet::FLAG_RELIABLE | cimmeria_mercury::packet::FLAG_ON_CHANNEL,
        seq,
        |p| crate::mercury::encrypt_packet(p, &[0u8; 32], EncryptionVersion::V1),
    );
    assert_eq!(packets.len(), 1);
    packets.into_iter().next().unwrap()
}

fn client<'a>(
    transport: &'a Arc<dyn Transport>,
    connected: &'a Connected,
    entity_to_addr: &'a EntityToAddr,
) -> CraftClient<'a> {
    CraftClient {
        transport,
        connected,
        entity_to_addr,
    }
}

fn report(stations: StationSet) -> CraftingStations {
    CraftingStations {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        stations,
        cause: StationChangeCause::Moved,
    }
}

/// The login send goes out in the login crafting bundle, and it arms the
/// change-only sends after it. Without a database the bundle carries only
/// the options, so they still reach the client.
#[tokio::test]
async fn login_send_is_unconditional_and_arms_change_sends() {
    let (typed, transport, addr, connected, entity_to_addr) = session();

    handle_station_report(
        report([Some(900), None, None, None]),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert!(typed.filter_to(addr).is_empty(), "nothing before login");

    push_crafting_on_login(
        ENTITY,
        1,
        &None,
        client(&transport, &connected, &entity_to_addr),
    )
    .await;
    let login = CraftingOptions {
        crafting: CraftingInfo {
            items: vec![],
            entities: vec![900],
        },
        ..CraftingOptions::default()
    };
    assert_eq!(typed.filter_to(addr), vec![login_packet(&login, 0)]);

    handle_station_report(report([None; 4]), &transport, &connected, &entity_to_addr).await;
    assert_eq!(
        typed.filter_to(addr),
        vec![
            login_packet(&login, 0),
            packet(&CraftingOptions::default(), 1)
        ]
    );
}

/// `enable_craft_anywhere` sends at once, and the flag sticks for the gate.
#[tokio::test]
async fn enable_craft_anywhere_sends_and_sets_the_flag() {
    let (typed, transport, addr, connected, entity_to_addr) = session();
    assert!(!craft_anywhere(ENTITY, &connected, &entity_to_addr));

    assert!(enable_craft_anywhere(ENTITY, &transport, &connected, &entity_to_addr).await);

    let mine = CraftingInfo {
        items: vec![],
        entities: vec![ENTITY as i32],
    };
    let expected = CraftingOptions {
        crafting: mine.clone(),
        research: mine.clone(),
        reverse_engineering: mine.clone(),
        alloying: mine,
    };
    assert_eq!(typed.filter_to(addr), vec![packet(&expected, 0)]);
    assert!(craft_anywhere(ENTITY, &connected, &entity_to_addr));
}

/// Every send logs `options_changed` with the player's identity, the cause,
/// and the machine and tool id per section (0 for none).
#[tokio::test]
async fn options_changed_carries_identity_cause_and_ids() {
    let capture = LogCapture::install();
    let (_typed, transport, addr, connected, entity_to_addr) = session();
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .crafting_options
        .tools = vec![tool(20_002, 35)];

    assert!(
        login_options(ENTITY, PLAYER_ID, &None, &connected, &entity_to_addr)
            .await
            .is_some(),
        "the login always yields options"
    );
    handle_station_report(
        CraftingStations {
            entity_id: ENTITY,
            player_id: PLAYER_ID,
            stations: [Some(900), None, None, Some(902)],
            cause: StationChangeCause::StationDespawned,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let events: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|e| e.has_field("event", "options_changed"))
        .collect();
    assert_eq!(events.len(), 2, "{events:#?}");
    for e in &events {
        assert_eq!(e.target, "crafting");
        assert!(e.has_field("account_id", &ACCOUNT_ID.to_string()), "{e:#?}");
        assert!(e.has_field("player_id", &PLAYER_ID.to_string()), "{e:#?}");
        assert!(e.has_field("entity_id", &ENTITY.to_string()), "{e:#?}");
    }
    assert!(events[0].has_field("cause", "login"), "{:#?}", events[0]);
    assert!(
        events[0].has_field("tools", "[20002, 20002, 20002, 0]"),
        "{:#?}",
        events[0]
    );
    assert!(
        events[1].has_field("cause", "station_despawned"),
        "{:#?}",
        events[1]
    );
    assert!(
        events[1].has_field("stations", "[900, 0, 0, 902]"),
        "{:#?}",
        events[1]
    );
}

/// An update for an entity with no session is a WARN `lookup_failed`, not a
/// silent drop.
#[tokio::test]
async fn an_update_without_a_session_warns() {
    let capture = LogCapture::install();
    let (typed, transport, _addr, connected, _) = session();
    let no_mapping = Arc::new(Mutex::new(HashMap::new()));

    handle_station_report(report([Some(900); 4]), &transport, &connected, &no_mapping).await;

    assert!(typed.is_empty());
    let event = capture
        .find_message(
            tracing::Level::WARN,
            "crafting options update for an entity with no session",
        )
        .expect("the miss is logged");
    assert!(event.has_field("event", "lookup_failed"), "{event:#?}");
    assert!(event.has_field("phase", "session"), "{event:#?}");
    assert!(
        event.has_field("entity_id", &ENTITY.to_string()),
        "{event:#?}"
    );
}

/// A transport that refuses every send.
struct FailingTransport;

impl Transport for FailingTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { Err(std::io::Error::other("link down")) })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

/// A change send that did not go out is not recorded as received: the next
/// identical report sends it again. Recording before the send would
/// suppress every identical report after a transient failure, leaving the
/// window on stale options until something else changed.
#[tokio::test]
async fn a_failed_change_send_is_retried_by_the_next_identical_report() {
    let (typed, transport, addr, connected, entity_to_addr) = session();
    let failing: Arc<dyn Transport> = Arc::new(FailingTransport);
    login_options(ENTITY, PLAYER_ID, &None, &connected, &entity_to_addr).await;
    let near = [Some(900), None, None, None];

    handle_station_report(report(near), &failing, &connected, &entity_to_addr).await;
    handle_station_report(report(near), &transport, &connected, &entity_to_addr).await;

    let expected = CraftingOptions {
        crafting: CraftingInfo {
            items: vec![],
            entities: vec![900],
        },
        ..CraftingOptions::default()
    };
    // The failed attempt consumed sequence 0.
    assert_eq!(typed.filter_to(addr), vec![packet(&expected, 1)]);
}

/// A login bundle that did not go out leaves nothing recorded, so the next
/// report sends 140 even when it matches the login options (here: nothing
/// in reach either time).
#[tokio::test]
async fn a_failed_login_bundle_is_retried_by_the_next_report() {
    let (typed, transport, addr, connected, entity_to_addr) = session();
    let failing: Arc<dyn Transport> = Arc::new(FailingTransport);
    push_crafting_on_login(
        ENTITY,
        PLAYER_ID,
        &None,
        client(&failing, &connected, &entity_to_addr),
    )
    .await;

    handle_station_report(report([None; 4]), &transport, &connected, &entity_to_addr).await;

    assert_eq!(
        typed.filter_to(addr),
        vec![packet(&CraftingOptions::default(), 1)],
        "the failed bundle consumed sequence 0"
    );
}

/// A crafting-bag read that fails at login clears the session's tools, so
/// the login options name no tool, as the `lookup_failed` WARN says. Keeping
/// the old tools would show a tool the player may no longer carry.
#[tokio::test]
async fn a_failed_login_tool_read_sends_no_tool() {
    let (_typed, _transport, addr, connected, entity_to_addr) = session();
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .crafting_options
        .tools = vec![tool(20_002, 35)];
    // Nothing listens on port 1, so the bag read fails fast.
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");

    let options = login_options(
        ENTITY,
        PLAYER_ID,
        &Some(Arc::new(unreachable)),
        &connected,
        &entity_to_addr,
    )
    .await
    .expect("the login always yields options");

    assert_eq!(options, CraftingOptions::default(), "no tool named");
    assert!(connected.lock().unwrap()[&addr]
        .crafting_options
        .tools
        .is_empty());
}

/// A world entry forgets the stations of the world being left and holds
/// change sends until the next login send. The tools and craft anywhere
/// stay.
#[test]
fn begin_world_entry_forgets_stations_and_disarms() {
    let mut inputs = station_and_tool_fixture();
    inputs.craft_anywhere = true;
    inputs.armed = true;
    inputs.last_sent = Some(CraftingOptions::default());

    inputs.begin_world_entry();

    assert_eq!(inputs.stations, [None; 4]);
    assert!(!inputs.armed);
    assert_eq!(inputs.last_sent, None);
    assert_eq!(inputs.tools, station_and_tool_fixture().tools);
    assert!(inputs.craft_anywhere);
}

/// A 140 change that fails to send is a WARN `push_failed` with the
/// player's identity.
#[tokio::test]
async fn a_failed_options_send_warns_with_identity() {
    let capture = LogCapture::install();
    let (_typed, _transport, _addr, connected, entity_to_addr) = session();
    let failing: Arc<dyn Transport> = Arc::new(FailingTransport);

    login_options(ENTITY, PLAYER_ID, &None, &connected, &entity_to_addr).await;
    handle_station_report(
        report([Some(900), None, None, None]),
        &failing,
        &connected,
        &entity_to_addr,
    )
    .await;

    let event = capture
        .find_event(
            tracing::Level::WARN,
            "onUpdateCraftingOptions did not reach the client",
            "send_error",
        )
        .expect("the failed send is logged");
    assert!(event.has_field("event", "push_failed"), "{event:#?}");
    assert!(event.has_field("what", "crafting_options"), "{event:#?}");
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("account_id", &ACCOUNT_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("entity_id", &ENTITY.to_string()),
        "{event:#?}"
    );
}
