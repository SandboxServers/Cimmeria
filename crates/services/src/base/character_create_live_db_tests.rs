//! Live-DB guard for `handle_create_character`'s `access_level`
//! persistence.
//!
//! Bug shape: the `createCharacter` INSERT stamps the new
//! `sgw_player.access_level` from the session's account access level
//! (`get_access_level(connected, addr)`). The column defaults to
//! `0 NOT NULL`, so if the `access_level` column or its `$N` bind is
//! ever dropped from the INSERT, the row silently lands at
//! `access_level = 0` and a GM account's characters lose their
//! privilege marker on every world-entry load. This test drives the
//! real handler with a session at access_level 2 (GameMaster) and
//! asserts the persisted row reads back 2 — it FAILS the moment the
//! bind/column is reverted.
//!
//! The payload is built to be valid against the same `db/database.sql`
//! the CI live-DB job loads. It uses CharDefId 9 (SGU Asgard Male),
//! which has the fewest `VIS_Optional` visual groups in
//! `resources.char_creation_visgroups` (only 115 Head, 118 SkinPattern,
//! 119 Accessory). The handler requires a client choice for every
//! optional group, so all three are supplied with real choice ids from
//! `resources.char_creation_choices`:
//!   - group 115 -> choice 619 (`BS_Asgard.BS_AS_Head_00`, no item)
//!   - group 118 -> choice 631 (`BS_Asgard.BS_AS_SkinPattern_00`, no item)
//!   - group 119 -> choice 641 (`ACC_HumanMale.ACC_HM_Accessory_00`,
//!     item 4343 — a starter item that the handler may insert into
//!     `sgw_inventory`; cleanup removes any such rows by character_id).
//!
//! The four `VIS_Forced` groups (113 Torso, 114 Legs, 116 Hands,
//! 117 Boots) are auto-resolved by the handler and need no choice.

use super::*;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Sentinel base for the character-create live-DB tests. The crate's
/// `0x7000_xx00` windows are reserved contiguously up through
/// `0x7000_1900` (cover-loader); this steps to the next free slot so
/// concurrent live-DB runs don't collide on account/player ids. Fits
/// in `i32`. See the neighbour map in
/// `crates/services/src/base/character/delete_live_db_tests.rs`.
const TEST_BASE: i32 = 0x7000_1A00;

/// CharDefId with the fewest `VIS_Optional` groups in the seed data
/// (SGU Asgard Male). Keeps the valid payload as small as possible.
const TEST_CHAR_DEF_ID: i32 = 9;

/// `(vis_group_id, choice_id)` pairs satisfying every `VIS_Optional`
/// group of `TEST_CHAR_DEF_ID`. Derived from
/// `db/resources/Archetypes/Seed/char_creation_choices.sql`.
const VISUAL_CHOICES: &[(i32, i32)] = &[(115, 619), (118, 631), (119, 641)];

/// Skin tint id inside the handler's accepted `0..=15` range.
const TEST_SKIN_TINT: i32 = 0;

/// The session's account access level we expect to be persisted.
/// 2 == GameMaster — a non-default value, so a row that reads back 0
/// means the access_level bind/column was lost.
const SESSION_ACCESS_LEVEL: u32 = 2;

async fn cleanup(pool: &PgPool, account_id: i32) {
    // sgw_inventory FKs to sgw_player(player_id); delete starter items
    // (if any) by joining back through the account before the player row.
    let _ = sqlx::query(
        "DELETE FROM sgw_inventory WHERE character_id IN \
         (SELECT player_id FROM sgw_player WHERE account_id = $1)",
    )
    .bind(account_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert the test account with `accesslevel = 2` to mirror a real GM
/// account. The handler reads the level from session state, not this
/// row, but seeding it keeps the fixture self-consistent.
async fn insert_account(pool: &PgPool, account_id: i32, access_level: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, accesslevel) \
         VALUES ($1, $2, '', $3)",
    )
    .bind(account_id)
    .bind(format!("ccreate-{account_id}"))
    .bind(access_level)
    .execute(pool)
    .await
    .expect("insert account");
}

fn make_connected(
    addr: SocketAddr,
    account_id: u32,
    access_level: u32,
) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    let mut state = test_default_connected_client_state();
    state.account_id = account_id;
    state.access_level = access_level;
    // A non-zero account entity id so the post-INSERT onCharacterList
    // build path has something to address; value is otherwise irrelevant
    // to the access_level assertion.
    state.account_entity_id = 0xAAAA_0001;
    let mut m = HashMap::new();
    m.insert(addr, state);
    Arc::new(Mutex::new(m))
}

/// Build a valid `createCharacter` payload, mirroring the handler's own
/// wire parsing:
/// `[WSTRING Name][WSTRING ExtraName][INT32 CharDefId]
///  [u32 VisualChoiceCount][count × {INT32 VisGroupId, INT32 ChoiceId}]
///  [INT32 SkinTintColorID]`.
fn build_create_character_payload(
    name: &str,
    extra_name: &str,
    char_def_id: i32,
    visual_choices: &[(i32, i32)],
    skin_tint_color_id: i32,
) -> Vec<u8> {
    let mut buf = Vec::new();
    crate::mercury::write_wstring(&mut buf, name);
    crate::mercury::write_wstring(&mut buf, extra_name);
    buf.extend_from_slice(&char_def_id.to_le_bytes());
    buf.extend_from_slice(&(visual_choices.len() as u32).to_le_bytes());
    for &(vis_group_id, choice_id) in visual_choices {
        buf.extend_from_slice(&vis_group_id.to_le_bytes());
        buf.extend_from_slice(&choice_id.to_le_bytes());
    }
    buf.extend_from_slice(&skin_tint_color_id.to_le_bytes());
    buf
}

/// The new character must persist the session's account access level,
/// not the column default. A session at access_level 2 (GameMaster)
/// must produce an `sgw_player` row whose `access_level` reads back 2.
///
/// Reverting the `access_level` column or its `$16` bind in
/// `handle_create_character`'s INSERT makes the row default to 0 and
/// trips the final assertion.
#[tokio::test]
async fn create_character_persists_session_access_level() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;

    cleanup(&pool, account_id).await;
    insert_account(&pool, account_id, SESSION_ACCESS_LEVEL as i32).await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55810".parse().unwrap();
    let connected = make_connected(addr, account_id as u32, SESSION_ACCESS_LEVEL);
    let key = [0u8; 32];
    let db_pool = Some(Arc::new(pool.clone()));

    let payload = build_create_character_payload(
        "Asgard Test",
        "",
        TEST_CHAR_DEF_ID,
        VISUAL_CHOICES,
        TEST_SKIN_TINT,
    );

    let result = handle_create_character(
        &dyn_transport,
        addr,
        key,
        account_id as u32,
        &payload,
        &connected,
        &db_pool,
    )
    .await;
    assert!(
        result.is_ok(),
        "createCharacter handler must return Ok for a valid payload \
         (CharDefId {TEST_CHAR_DEF_ID} with all optional visual groups \
         satisfied); got {result:?}",
    );

    // The handler succeeds by sending the updated character list back;
    // a failure path would have sent onCharacterCreateFailed instead.
    // The authoritative check is the persisted row below — read it back
    // by the account we created.
    let persisted: Option<i32> =
        sqlx::query_scalar("SELECT access_level FROM sgw_player WHERE account_id = $1")
            .bind(account_id)
            .fetch_optional(&pool)
            .await
            .expect("query persisted access_level");

    assert_eq!(
        persisted,
        Some(SESSION_ACCESS_LEVEL as i32),
        "createCharacter must persist sgw_player.access_level from the \
         session's account access level ({SESSION_ACCESS_LEVEL}). A value \
         of Some(0) means the access_level column/bind was dropped from \
         the INSERT — a GM account's characters would silently load at \
         access_level 0. None means the character row was never inserted.",
    );

    cleanup(&pool, account_id).await;
}
