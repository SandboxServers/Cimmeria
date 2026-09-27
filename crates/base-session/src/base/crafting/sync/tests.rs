//! The crafting state pushes: the login bundle byte for byte, the relog
//! round trip against the database, and the starting paradigm levels.

use super::*;
use crate::base::crafting::persistence::{load_crafting_state, save_crafting_state};
use crate::base::crafting::test_players::{cleanup, insert_player, OneSession, SESSION_ACCOUNT_ID};
use crate::mercury::encrypt_packet;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};
use cimmeria_entity::crafting::DEFAULT_RACIAL_PARADIGM_LEVELS;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::packet::{FLAG_ON_CHANNEL, FLAG_RELIABLE};

const ENTITY: u32 = 4270;

/// Sentinels in the crafting `0x7000_Cxxx` block, past `handlers.rs`
/// (`0x7000_CC00..`): `0x7000_CD00..0x7000_CD1F`.
const TEST_BASE: i32 = 0x7000_CD00;

/// Known 78 (expertise 12) and 21 (no expertise row), the default
/// paradigms, blueprint 25, 3 ASP.
fn fixture_state() -> CraftingState {
    let mut state = CraftingState::new();
    state.discipline_ids = vec![78, 21];
    state.expertise.insert(78, 12);
    state.apply_default_paradigm_levels();
    state.blueprint_ids = vec![25];
    state.applied_science_points = 3;
    state
}

/// The packets `send_bundle_to_witness_reliable` emits for the login
/// bundle of `state` on a fresh test session (no acks, the all-zero key).
/// A test session has no stations, tools or craft anywhere, so its
/// options are all empty.
fn expected_bundle_packets(entity_id: u32, state: &CraftingState, seq: u32) -> Vec<Vec<u8>> {
    expected_login_packets(entity_id, Some(state), seq)
}

/// [`expected_bundle_packets`] for a bundle that may carry no state.
fn expected_login_packets(entity_id: u32, state: Option<&CraftingState>, seq: u32) -> Vec<Vec<u8>> {
    let options = CraftingOptions::default();
    let (packets, _) = build_login_bundle(entity_id, state, Some(&options)).finalize(
        FLAG_RELIABLE | FLAG_ON_CHANNEL,
        seq,
        |plaintext| encrypt_packet(plaintext, &[0u8; 32], EncryptionVersion::V1),
    );
    packets
}

/// The login messages for the fixture, byte for byte and in order: 136 per
/// known discipline in `discipline_ids` order (a known discipline with no
/// expertise row reads 0), 138 per paradigm by id with Common at 5, 139,
/// then the ASP property carrying the total.
#[test]
fn login_messages_are_byte_exact_for_a_fixture_state() {
    let messages = crafting_state_messages(&fixture_state());
    let expected: Vec<(u16, Vec<u8>)> = vec![
        (136, vec![78, 0, 0, 0, 12, 0, 0, 0]),
        (136, vec![21, 0, 0, 0, 0, 0, 0, 0]),
        (138, vec![1, 0, 0, 0, 5]),
        (138, vec![2, 0, 0, 0, 1]),
        (138, vec![3, 0, 0, 0, 1]),
        (138, vec![4, 0, 0, 0, 1]),
        (138, vec![5, 0, 0, 0, 1]),
        (139, vec![1, 0, 0, 0, 25, 0, 0, 0]),
        (7, vec![2, 0, 0, 0, 3, 0, 0, 0]),
    ];
    assert_eq!(messages, expected);
}

/// The login bundle's messages are the state's, then
/// `onUpdateCraftingOptions` (140) with the options' bytes, last. Either
/// half may be absent. Dropping the 140 from the bundle (a second, separate
/// login push) fails the first assertion.
#[test]
fn login_messages_end_with_the_crafting_options() {
    let state = fixture_state();
    let options = CraftingOptions {
        crafting: cimmeria_wire::crafting::CraftingInfo {
            items: vec![20_002],
            entities: vec![900],
        },
        ..CraftingOptions::default()
    };
    let mut expected = crafting_state_messages(&state);
    expected.push((140, crafting_options_args(&options)));
    assert_eq!(login_messages(Some(&state), Some(&options)), expected);
    assert_eq!(
        login_messages(None, Some(&options)),
        vec![(140, crafting_options_args(&options))]
    );
    assert_eq!(
        login_messages(Some(&state), None),
        crafting_state_messages(&state)
    );
    // 140 = 0x8C, framed like 136-139 (sub-index 140 - 61 = 79).
    let bundle = build_login_bundle(ENTITY, Some(&state), Some(&options));
    assert_eq!(bundle.num_messages(), 10);
}

/// The bundle body is those messages, each framed as an SGWPlayer entity
/// method (the extended 0xBD encoding for 136-139, direct for property 7).
/// The frame is rebuilt here with the wire crate's own framer, so a
/// bundle that dropped, reordered or re-framed a message fails.
#[test]
fn login_bundle_frames_every_message_in_order() {
    let state = fixture_state();
    let mut body = Vec::new();
    for (method_index, args) in crafting_state_messages(&state) {
        crate::mercury::append_entity_method(
            &mut body,
            method_index,
            IDBASE_SGW_PLAYER,
            ENTITY,
            &args,
        );
    }
    // 136 for discipline 78: marker, u16 length 13, entity id, sub-index
    // 136 - 61 = 75, then the eight argument bytes.
    let entity = ENTITY.to_le_bytes();
    let first = [
        &[0xBD, 13, 0][..],
        &entity[..],
        &[75, 78, 0, 0, 0, 12, 0, 0, 0][..],
    ]
    .concat();
    assert_eq!(&body[..first.len()], &first[..]);

    let bundle = build_crafting_state_bundle(ENTITY, &state);
    assert_eq!(bundle.num_messages(), 9);
    assert_eq!(bundle.body_len(), body.len());
    let (plain, _) = bundle.finalize(FLAG_RELIABLE | FLAG_ON_CHANNEL, 0, |p| p.to_vec());
    assert_eq!(plain.len(), 1, "one packet");
    assert!(
        plain[0].windows(body.len()).any(|w| w == body.as_slice()),
        "the bundle carries exactly the framed messages, in order"
    );
}

/// `push_login_bundle` sends the bundle to the player's own client and
/// nowhere else.
#[tokio::test]
async fn push_login_bundle_sends_one_packet_to_the_owner() {
    let session = OneSession::new(ENTITY, 55730);
    let state = fixture_state();

    assert_eq!(
        push_login_bundle(
            ENTITY,
            Some(&state),
            Some(&CraftingOptions::default()),
            session.client()
        )
        .await,
        Ok(())
    );

    assert_eq!(session.typed.len(), 1);
    assert_eq!(
        session.typed.filter_to(session.addr),
        expected_bundle_packets(ENTITY, &state, 0)
    );
}

/// A single push that cannot reach the client (no address for the
/// entity) is a `push_failed` WARN with the reason and the identity, and
/// `push_login_bundle` reports the reason to its caller; a silent drop
/// would leave stale crafting state on screen.
#[tokio::test]
async fn unsent_push_is_a_warn() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55735);
    let empty = Arc::new(Mutex::new(HashMap::new()));
    let client = CraftClient {
        entity_to_addr: &empty,
        ..session.client()
    };

    assert_eq!(
        push_login_bundle(ENTITY, Some(&fixture_state()), None, client).await,
        Err("entity_to_addr_miss")
    );
    push_asp(ENTITY, 17, 3, client).await;

    let warns: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "crafting" && c.has_field("event", "push_failed"))
        .collect();
    assert_eq!(warns.len(), 1, "{warns:#?}");
    let warn = &warns[0];
    assert_eq!(warn.level, tracing::Level::WARN);
    assert!(warn.has_field("what", "asp"));
    assert!(warn.has_field("reason", "entity_to_addr_miss"));
    assert!(warn.has_field("player_id", "17"));
    assert!(warn.has_field("entity_id", &ENTITY.to_string()));
}

/// Live DB: a login bundle that cannot be sent is `login_sync_failed` with
/// `reason = send` and the send failure as the error class, not a
/// `login_sync`.
#[tokio::test]
async fn unsent_login_bundle_is_login_sync_failed() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 6, TEST_BASE + 7);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55737);
    let empty = Arc::new(Mutex::new(HashMap::new()));
    let client = CraftClient {
        entity_to_addr: &empty,
        ..session.client()
    };
    let db_pool = Some(Arc::new(pool.clone()));

    push_crafting_on_login(ENTITY, player_id, &db_pool, client).await;
    cleanup(&pool, account_id, player_id).await;

    let warn = capture
        .find_event(tracing::Level::WARN, "bundle not sent", "send")
        .expect("login_sync_failed WARN");
    for (k, v) in [
        ("event", "login_sync_failed"),
        ("error_class", "entity_to_addr_miss"),
        ("player_id", &player_id.to_string()),
        ("entity_id", &ENTITY.to_string()),
    ] {
        assert!(warn.has_field(k, v), "{k}={v}: {warn:#?}");
    }
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "login_sync")),
        "no login_sync for a bundle that was not sent"
    );
}

/// Without a database the login sync sends only the crafting options:
/// there is no state to back the rest, and the `mapLoaded` bundle already
/// carried the row's ASP and blueprints.
#[tokio::test]
async fn login_sync_without_a_database_sends_only_the_options() {
    let session = OneSession::new(ENTITY, 55731);
    push_crafting_on_login(ENTITY, 1, &None, session.client()).await;
    assert_eq!(
        session.typed.filter_to(session.addr),
        expected_login_packets(ENTITY, None, 0)
    );
}

/// The single pushes: 136, 138, 139 and the ASP total, each one reliable
/// packet to the owner.
#[tokio::test]
async fn single_pushes_carry_their_method_and_arguments() {
    let session = OneSession::new(ENTITY, 55732);
    push_discipline(ENTITY, 1, 78, 1, session.client()).await;
    push_paradigm(ENTITY, 1, 2, 3, session.client()).await;
    push_known_crafts(ENTITY, 1, &[25, 412], session.client()).await;
    push_asp(ENTITY, 1, 9, session.client()).await;

    let packet = |seq, method, args: &[u8]| {
        build_player_entity_method_packet(
            &[0u8; 32],
            seq,
            &[],
            ENTITY,
            method,
            args,
            EncryptionVersion::V1,
        )
    };
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![
            packet(0, 136, &[78, 0, 0, 0, 1, 0, 0, 0]),
            packet(1, 138, &[2, 0, 0, 0, 3]),
            packet(2, 139, &[2, 0, 0, 0, 25, 0, 0, 0, 0x9C, 0x01, 0, 0]),
            packet(3, 7, &[2, 0, 0, 0, 9, 0, 0, 0]),
        ]
    );
}

/// Live DB, the relog guard: state saved in one session is what the next
/// world entry pushes. Disciplines, expertise, paradigm levels, blueprints
/// and ASP all come back from the database and reach the client in one
/// bundle. Dropping any field from the load (or the push) fails the
/// packet comparison against the independently built expected state.
#[tokio::test]
async fn relog_restores_and_pushes_the_whole_crafting_state() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE, TEST_BASE + 1);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;

    let mut saved = CraftingState::new();
    saved.discipline_ids = vec![78, 21];
    saved.expertise.insert(78, 60);
    saved.expertise.insert(21, 4);
    saved.racial_paradigm_levels = HashMap::from([(1, 6), (2, 1), (3, 2), (4, 1), (5, 1)]);
    saved.blueprint_ids = vec![25, 412];
    saved.applied_science_points = 3;
    save_crafting_state(&pool, player_id, &saved)
        .await
        .expect("save");

    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55733);
    let db_pool = Some(Arc::new(pool.clone()));
    push_crafting_on_login(ENTITY, player_id, &db_pool, session.client()).await;

    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    let event = login_sync_event(&capture);
    for (k, v) in [
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", ENTITY.to_string()),
        ("disciplines", "2".to_string()),
        ("paradigms", "5".to_string()),
        ("blueprints", "2".to_string()),
        ("asp", "3".to_string()),
        ("defaults_applied", "false".to_string()),
        ("crafting_options", "true".to_string()),
    ] {
        assert!(event.has_field(k, &v), "{k}={v}: {event:#?}");
    }
    assert_eq!(
        sent,
        expected_bundle_packets(ENTITY, &saved, 0),
        "the relog push is the saved state, byte for byte"
    );
}

/// Live DB, the starting-levels guard for new characters: a player row inserted
/// without naming `racial_paradigm_levels` gets the column default, which
/// must equal `DEFAULT_RACIAL_PARADIGM_LEVELS` (index i = paradigm i+1).
/// Reverting the column default in `sgw_player.sql` fails the first
/// assertion.
#[tokio::test]
async fn new_character_column_default_is_the_starting_paradigm_levels() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 2, TEST_BASE + 3);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;

    let stored: Vec<i32> =
        sqlx::query_scalar("SELECT racial_paradigm_levels FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("select levels");
    cleanup(&pool, account_id, player_id).await;

    let expected: Vec<i32> = DEFAULT_RACIAL_PARADIGM_LEVELS
        .iter()
        .enumerate()
        .map(|(i, &(id, level))| {
            assert_eq!(id, i as i32 + 1, "the constant is in id order");
            i32::from(level)
        })
        .collect();
    assert_eq!(stored, expected);
}

/// Live DB, the starting-levels guard for existing characters: a stored
/// empty array (every older and every seeded character) loads as
/// the starting levels, and the login push tells the client Common is 5.
/// Removing the default from the load fails both assertions.
#[tokio::test]
async fn existing_character_with_no_levels_loads_and_pushes_the_defaults() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 4, TEST_BASE + 5);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;
    sqlx::query("UPDATE sgw_player SET racial_paradigm_levels = '{}' WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("clear levels");

    let loaded = load_crafting_state(&pool, player_id).await.expect("load");
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55734);
    let db_pool = Some(Arc::new(pool.clone()));
    push_crafting_on_login(ENTITY, player_id, &db_pool, session.client()).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;

    assert_eq!(
        loaded.racial_paradigm_levels,
        HashMap::from(DEFAULT_RACIAL_PARADIGM_LEVELS),
        "an empty stored array loads as the starting levels"
    );
    let mut expected_state = CraftingState::new();
    expected_state.racial_paradigm_levels = HashMap::from(DEFAULT_RACIAL_PARADIGM_LEVELS);
    assert_eq!(
        sent,
        expected_bundle_packets(ENTITY, &expected_state, 0),
        "the login push carries 138 for all five paradigms, Common at 5"
    );
    assert!(
        login_sync_event(&capture).has_field("defaults_applied", "true"),
        "the login_sync event says the defaults were applied"
    );
}

/// Live DB, the partial-array guard: a stored array shorter than five
/// (here Common 7 and Human 2 only) keeps its stored levels and loads the
/// rest at their starting levels, so the login push carries a 138 for all
/// five paradigms. A push of only the stored two would leave the client's
/// Goa'uld, Asgard and Ancient levels at whatever they were before the relog.
/// Filling in only an empty map fails every assertion.
#[tokio::test]
async fn partial_stored_levels_load_and_push_all_five_paradigms() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 8, TEST_BASE + 9);
    cleanup(&pool, account_id, player_id).await;
    insert_player(&pool, account_id, player_id).await;
    sqlx::query("UPDATE sgw_player SET racial_paradigm_levels = '{7,2}' WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("store partial levels");

    let loaded = load_crafting_state(&pool, player_id).await.expect("load");
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 55738);
    let db_pool = Some(Arc::new(pool.clone()));
    push_crafting_on_login(ENTITY, player_id, &db_pool, session.client()).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;

    let levels = HashMap::from([(1, 7), (2, 2), (3, 1), (4, 1), (5, 1)]);
    assert_eq!(
        loaded.racial_paradigm_levels, levels,
        "stored levels kept, the missing three at their starting levels"
    );
    let mut expected_state = CraftingState::new();
    expected_state.racial_paradigm_levels = levels;
    let paradigm_pushes = crafting_state_messages(&expected_state)
        .into_iter()
        .filter(|(method, _)| *method == ON_UPDATE_RACIAL_PARADIGM_LEVEL)
        .count();
    assert_eq!(
        paradigm_pushes, 5,
        "the expected bundle names every paradigm"
    );
    assert_eq!(
        sent,
        expected_bundle_packets(ENTITY, &expected_state, 0),
        "the login push carries 138 for all five paradigms"
    );
    let event = login_sync_event(&capture);
    assert!(event.has_field("paradigms", "5"), "{event:#?}");
    assert!(event.has_field("defaults_applied", "true"), "{event:#?}");
}

/// A login whose crafting load fails (a pool that cannot connect stands in
/// for a database outage) is a WARN naming the phase, and the bundle
/// carries only the crafting options.
#[tokio::test]
async fn failed_login_load_is_a_warn_and_sends_only_the_options() {
    let capture = LogCapture::install();
    // A pool whose connections can never be made: port 1 refuses.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let session = OneSession::new(ENTITY, 55736);
    let db_pool = Some(Arc::new(pool));

    push_crafting_on_login(ENTITY, 23, &db_pool, session.client()).await;

    assert_eq!(
        session.typed.filter_to(session.addr),
        expected_login_packets(ENTITY, None, 0),
        "only the options on a failed load"
    );
    let warn = capture
        .find_event(tracing::Level::WARN, "crafting login sync", "load")
        .expect("login_sync_failed WARN");
    for (k, v) in [
        ("event", "login_sync_failed"),
        ("player_id", "23"),
        ("entity_id", &ENTITY.to_string()),
        ("account_id", &SESSION_ACCOUNT_ID.to_string()),
    ] {
        assert!(warn.has_field(k, v), "{k}={v}: {warn:#?}");
    }
}

fn login_sync_event(capture: &LogCaptureGuard) -> Captured {
    capture
        .all()
        .into_iter()
        .find(|c| c.target == "crafting" && c.has_field("event", "login_sync"))
        .expect("login_sync event")
}
