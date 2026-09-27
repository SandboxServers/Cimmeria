//! The crafting state pushes: the login bundle byte for byte, the relog
//! round trip against the database, and the D-CR03 paradigm defaults.

use super::*;
use crate::base::crafting::persistence::save_crafting_state;
use crate::base::crafting::test_players::{cleanup, insert_player, OneSession};
use crate::mercury::encrypt_packet;
use crate::test_support::require_db_or_skip;
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

/// The packets `send_bundle_to_witness_reliable` emits for `state` on a
/// fresh test session (sequence 0, no acks, the all-zero key).
fn expected_bundle_packets(entity_id: u32, state: &CraftingState, seq: u32) -> Vec<Vec<u8>> {
    let (packets, _) = build_crafting_state_bundle(entity_id, state).finalize(
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

    push_login_bundle(ENTITY, &state, session.client()).await;

    assert_eq!(session.typed.len(), 1);
    assert_eq!(
        session.typed.filter_to(session.addr),
        expected_bundle_packets(ENTITY, &state, 0)
    );
}

/// Without a database the login sync sends nothing: there is no state to
/// back it, and the `mapLoaded` bundle already carried the row's ASP and
/// blueprints.
#[tokio::test]
async fn login_sync_without_a_database_sends_nothing() {
    let session = OneSession::new(ENTITY, 55731);
    push_crafting_on_login(ENTITY, 1, &None, session.client()).await;
    assert!(session.typed.is_empty());
}

/// The single pushes: 136, 138, 139 and the ASP total, each one reliable
/// packet to the owner.
#[tokio::test]
async fn single_pushes_carry_their_method_and_arguments() {
    let session = OneSession::new(ENTITY, 55732);
    push_discipline(ENTITY, 78, 1, session.client()).await;
    push_paradigm(ENTITY, 2, 3, session.client()).await;
    push_known_crafts(ENTITY, &[25, 412], session.client()).await;
    push_asp(ENTITY, 9, session.client()).await;

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

    let session = OneSession::new(ENTITY, 55733);
    let db_pool = Some(Arc::new(pool.clone()));
    push_crafting_on_login(ENTITY, player_id, &db_pool, session.client()).await;

    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(
        sent,
        expected_bundle_packets(ENTITY, &saved, 0),
        "the relog push is the saved state, byte for byte"
    );
}

/// Live DB, the D-CR03 guard for new characters: a player row inserted
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

/// Live DB, the D-CR03 guard for existing characters: a stored empty array
/// (every character created before D-CR03, and every seeded one) loads as
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
}
