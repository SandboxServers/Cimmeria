//! Tests for `onUpdateCraftingOptions` (140): the payload built from a
//! session's stations and tools, and when it is sent.

use super::*;
use crate::base::crafting::tools::ToolSpec;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_mercury::encryption::EncryptionVersion;

const ENTITY: u32 = 4280;

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
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
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

/// The login send goes out even when every section is empty, and it arms
/// the change-only sends after it.
#[tokio::test]
async fn login_send_is_unconditional_and_arms_change_sends() {
    let (typed, transport, addr, connected, entity_to_addr) = session();

    handle_station_report(
        ENTITY,
        [Some(900), None, None, None],
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert!(typed.filter_to(addr).is_empty(), "nothing before login");

    send_login_options(ENTITY, 1, &None, &transport, &connected, &entity_to_addr).await;
    let login = CraftingOptions {
        crafting: CraftingInfo {
            items: vec![],
            entities: vec![900],
        },
        ..CraftingOptions::default()
    };
    assert_eq!(typed.filter_to(addr), vec![packet(&login, 0)]);

    handle_station_report(ENTITY, [None; 4], &transport, &connected, &entity_to_addr).await;
    assert_eq!(
        typed.filter_to(addr),
        vec![packet(&login, 0), packet(&CraftingOptions::default(), 1)]
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
