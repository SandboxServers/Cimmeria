//! The crafting station gate and crafting options through the real
//! dispatcher.
//!
//! - A forged request with no station in reach, no tool and no "craft
//!   anywhere" is refused with a visible line, not passed to the verb.
//! - "Craft anywhere" lets the same request through.
//! - A station report sends `onUpdateCraftingOptions` byte for byte once
//!   the login send has happened, and nothing before it.
//! - `.allcraft` from a caller below GameMaster is refused with a line and
//!   turns nothing on.
//!
//! `db_pool` is `None` throughout, so no tool can be read: a request is
//! allowed only by the station mask or "craft anywhere".

use super::super::*;
use super::one_session;
use crate::cell::messages::PluginMsg;
use crate::cell::messages::{
    CraftRequest, CraftVerb, CraftingStations, GmAllCraft, StationChangeCause,
};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_base_crafting::base::crafting::feedback::feedback_text_args;
use cimmeria_base_crafting::base::crafting::options::CraftingOptionsExt;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_wire::cell::client_methods::player::ON_UPDATE_CRAFTING_OPTIONS;
use cimmeria_wire::crafting::{crafting_options_args, CraftingInfo, CraftingOptions};

const ENTITY: u32 = 4270;
const PLAYER_ID: i32 = 4271;
const ACCOUNT_ID: u32 = 4272;

/// Give the test session the player's account and character.
fn with_identity(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) {
    let mut clients = connected.lock().unwrap();
    let c = clients.get_mut(&addr).unwrap();
    c.account_id = ACCOUNT_ID;
    c.active_player_id = Some(PLAYER_ID);
}

fn craft(allowed: u8) -> CellToBaseMsg {
    CellToBaseMsg::Plugin(PluginMsg::new(CraftRequest {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        verb: CraftVerb::Craft {
            blueprint_id: 412,
            items: vec![20_001],
            quantity: 1,
        },
        allowed,
    }))
}

async fn dispatch(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    route_cell_message(
        msg,
        transport,
        connected,
        entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
        &super::crafting_plugins(),
    )
    .await;
}

fn feedback_packet(text: &str) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        ENTITY,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
        EncryptionVersion::V1,
    )
}

/// The guard: a forged `craft` with an empty station mask is refused with
/// the "no station" line. With the gate removed, the request reaches the
/// verb, which (with no database) answers "Crafting is unavailable right
/// now." instead.
#[tokio::test]
async fn forged_request_without_station_or_tool_is_refused_with_feedback() {
    let capture = LogCapture::install();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    with_identity(&connected, addr);

    dispatch(craft(0), &transport, &connected, &entity_to_addr).await;

    let sent = typed.filter_to(addr);
    assert_eq!(sent.len(), 1, "exactly the rejection line");
    assert_eq!(
        sent[0],
        feedback_packet("No crafting station or tool for crafting nearby.")
    );
    let event = capture
        .find_event(tracing::Level::INFO, "rejected", "no_station_or_tool")
        .expect("the rejection is logged with its reason");
    assert_eq!(event.target, "crafting");
    assert!(
        event.has_field("account_id", &ACCOUNT_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("entity_id", &ENTITY.to_string()),
        "{event:#?}"
    );
    assert!(event.has_field("verb", "craft"), "{event:#?}");
    assert!(event.has_field("station_mask", "0"), "{event:#?}");
    assert!(
        event.has_field("tools", "[]"),
        "no pool, no tools read: {event:#?}"
    );
}

/// A failed crafting-bag read in the gate is a WARN `lookup_failed` with the
/// player's identity, and the request is still refused with the line.
#[tokio::test]
async fn a_failed_tool_lookup_warns_and_refuses() {
    let capture = LogCapture::install();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    with_identity(&connected, addr);
    // Nothing listens on port 1, so every query fails fast.
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let db_pool = Some(Arc::new(unreachable));

    route_cell_message(
        craft(0),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &db_pool,
        &None,
        "127.0.0.1",
        7777,
        &super::crafting_plugins(),
    )
    .await;

    assert_eq!(
        typed.filter_to(addr),
        vec![feedback_packet(
            "No crafting station or tool for crafting nearby."
        )]
    );
    let event = capture
        .find_message(tracing::Level::WARN, "crafting gate lookup failed")
        .expect("the failed lookup is logged");
    assert!(event.has_field("event", "lookup_failed"), "{event:#?}");
    assert!(
        event.has_field("account_id", &ACCOUNT_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("entity_id", &ENTITY.to_string()),
        "{event:#?}"
    );
}

/// A mask for another verb does not open this one.
#[tokio::test]
async fn a_station_for_another_verb_does_not_allow_crafting() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);

    // Research, reverse engineering and alloying stations, no crafting one.
    dispatch(craft(0x0E), &transport, &connected, &entity_to_addr).await;

    assert_eq!(
        typed.filter_to(addr),
        vec![feedback_packet(
            "No crafting station or tool for crafting nearby."
        )]
    );
}

/// "Craft anywhere" passes the gate with an empty mask; the request then
/// reaches the verb, which cannot decide it without a database.
#[tokio::test]
async fn craft_anywhere_passes_the_gate() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .crafting_options_mut()
        .craft_anywhere = true;

    dispatch(craft(0), &transport, &connected, &entity_to_addr).await;

    assert_eq!(
        typed.filter_to(addr),
        vec![feedback_packet(
            "Crafting is unavailable right now. Nothing was changed."
        )]
    );
}

/// Learning a discipline needs no station: with no station, tool or craft
/// anywhere, the request reaches the spend handler (which, with no
/// database, refuses as unavailable) instead of the gate's line.
#[tokio::test]
async fn spend_is_not_gated() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    let spend = CellToBaseMsg::Plugin(PluginMsg::new(CraftRequest {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        verb: CraftVerb::Spend { discipline_id: 21 },
        allowed: 0,
    }));

    dispatch(spend, &transport, &connected, &entity_to_addr).await;

    assert_eq!(
        typed.filter_to(addr),
        vec![feedback_packet(
            "Learning disciplines is unavailable right now. Nothing was changed."
        )]
    );
}

/// Before the login send a station report only updates the session; after
/// it, a changed report sends one 140, byte for byte, and a repeat sends
/// nothing.
#[tokio::test]
async fn station_report_sends_options_only_after_login_and_on_change() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    let report = |stations| {
        CellToBaseMsg::Plugin(PluginMsg::new(CraftingStations {
            entity_id: ENTITY,
            player_id: PLAYER_ID,
            stations,
            cause: StationChangeCause::Moved,
        }))
    };

    dispatch(
        report([Some(900), None, None, None]),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert!(typed.filter_to(addr).is_empty(), "no send before login");

    // The login send has happened (all sections empty).
    {
        let mut clients = connected.lock().unwrap();
        let inputs = clients.get_mut(&addr).unwrap().crafting_options_mut();
        inputs.armed = true;
        inputs.last_sent = Some(CraftingOptions::default());
    }

    dispatch(
        report([Some(900), Some(901), None, Some(902)]),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    let expected_options = CraftingOptions {
        crafting: CraftingInfo {
            items: vec![],
            entities: vec![900],
        },
        research: CraftingInfo {
            items: vec![],
            entities: vec![901],
        },
        reverse_engineering: CraftingInfo::default(),
        alloying: CraftingInfo {
            items: vec![],
            entities: vec![902],
        },
    };
    let expected = build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        ENTITY,
        ON_UPDATE_CRAFTING_OPTIONS,
        &crafting_options_args(&expected_options),
        EncryptionVersion::V1,
    );
    assert_eq!(typed.filter_to(addr), vec![expected]);

    dispatch(
        report([Some(900), Some(901), None, Some(902)]),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert_eq!(
        typed.filter_to(addr).len(),
        1,
        "an unchanged set sends nothing"
    );
}

/// `.allcraft` whose caller is below GameMaster (a forged or mis-routed
/// message; the cell's console gate would not send it) is refused with a
/// line to the caller and leaves "craft anywhere" off.
#[tokio::test]
async fn allcraft_from_a_non_gm_is_refused() {
    let capture = LogCapture::install();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    assert_eq!(connected.lock().unwrap()[&addr].access_level, 0);

    let grant = CellToBaseMsg::Plugin(PluginMsg::new(GmAllCraft {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        gm_entity_id: ENTITY,
    }));
    dispatch(grant, &transport, &connected, &entity_to_addr).await;

    assert_eq!(
        typed.filter_to(addr),
        vec![feedback_packet(
            "allcraft: refused, GameMaster access is required."
        )]
    );
    assert!(
        !connected.lock().unwrap()[&addr]
            .crafting_options()
            .craft_anywhere
    );
    let event = capture
        .find_message(
            tracing::Level::WARN,
            "allcraft from a caller below GameMaster",
        )
        .expect("the refusal is logged");
    assert!(event.has_field("event", "gm_allcraft"), "{event:#?}");
    assert!(event.has_field("outcome", "refused"), "{event:#?}");
}
