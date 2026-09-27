//! Live-DB regression guards for moves into and out of the personal vault
//! (bank-vault BV-03; D-BV05, D-BV08), plus the `LogCapture` guards for
//! `move_accepted` and every BV-03 `move_rejected` reason.
//!
//! The cell's verdict ([`VaultAccess`]) is passed in directly: its
//! computation (fresh proximity per move) is pinned in
//! `cimmeria-cell-methods` `inventory/tests/vault_verdict.rs` and
//! `cimmeria-cell-interactions` `bank/tests.rs`. These tests pin what the
//! base does with it.
//!
//! Sentinels: accounts and players `0x7000_B500..=0x7000_B5B1`, entities
//! `0x7000_B5E0..=0x7000_B5EF`, item types `0x7000_B5F0` (bankable,
//! `{1,17}`, max stack 20) and `0x7000_B5F1` (a mission item that also
//! lists the vault, `{1,2,17}`, so only the mission rule can refuse it).
//! Ports 40830-40849. Skip when `DATABASE_URL` is unset.

use tracing::Level;

use super::allowlist_tests::assert_fields;
use super::tests::{cleanup, insert_account_and_player, insert_item};
use super::*;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, Captured, LogCapture, TestTransport,
};

pub(super) const BASE: i32 = 0x7000_B500;
pub(super) const BANKABLE: i32 = 0x7000_B5F0;
pub(super) const MISSION: i32 = 0x7000_B5F1;
pub(super) const BANKER: u32 = 0x7000_B5D0;

/// Next to the Banker, 2.5 units away.
pub(super) const AT_BANKER: VaultAccess = VaultAccess::Open {
    banker_id: Some(BANKER),
    distance: Some(2.5),
};

pub(super) async fn insert_types(pool: &PgPool) {
    for (id, sets) in [(BANKABLE, "{1,17}"), (MISSION, "{1,2,17}")] {
        sqlx::query(
            "INSERT INTO resources.items (\
                item_id, description, name, quality_id, tech_comp, tier, \
                max_stack_size, container_sets \
             ) VALUES ($1, '', 'bv03-vault', 'ITEM_QUALITY_Normal', 0, 1, 20, $2::integer[]) \
             ON CONFLICT (item_id) DO UPDATE SET container_sets = EXCLUDED.container_sets, \
                 max_stack_size = EXCLUDED.max_stack_size",
        )
        .bind(id)
        .bind(sets)
        .execute(pool)
        .await
        .expect("insert synthetic item type");
    }
}

pub(super) async fn setup(pool: &PgPool, account_id: i32, player_id: i32) {
    teardown(pool, account_id, player_id).await;
    insert_account_and_player(pool, account_id, player_id).await;
    insert_types(pool).await;
}

pub(super) async fn teardown(pool: &PgPool, account_id: i32, player_id: i32) {
    cleanup(pool, account_id, player_id).await;
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = ANY($1)")
        .bind(vec![BANKABLE, MISSION])
        .execute(pool)
        .await;
}

/// Every row of `player_id`: `(item_id, container, slot, stack)`.
pub(super) async fn rows(pool: &PgPool, player_id: i32) -> Vec<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT item_id, container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .fetch_all(pool)
    .await
    .expect("rows query")
}

pub(super) struct Client {
    pub(super) transport: Arc<TestTransport>,
    pub(super) dyn_transport: Arc<dyn Transport>,
    pub(super) addr: SocketAddr,
    pub(super) e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(super) conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

/// A client in world as `entity_id`, so feedback lines reach it.
pub(super) fn in_world(entity_id: u32, port: u16) -> Client {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    Client {
        transport,
        dyn_transport,
        addr,
        e2a: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
    }
}

impl Client {
    /// Whether any packet to the client carries `needle` as UTF-16LE, the
    /// encoding of `onPlayerCommunication`'s text.
    pub(super) fn saw_text(&self, needle: &str) -> bool {
        let want: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
        // The test session's key is all zeros (`test_default_connected_client_state`).
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        self.transport
            .filter_to(self.addr)
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .any(|p| p.windows(want.len()).any(|w| w == want.as_slice()))
    }
}

pub(super) async fn mv(
    pool: &PgPool,
    client: &Client,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container: i32,
    slot: i32,
    quantity: i32,
    vault: VaultAccess,
) {
    handle_move_inventory_item_with_vault(
        entity_id,
        player_id,
        item_id,
        container,
        slot,
        quantity,
        vault,
        &Some(Arc::new(pool.clone())),
        &None,
        &client.dyn_transport,
        &client.conn,
        &client.e2a,
    )
    .await;
}

fn bank_events(capture: &crate::test_support::LogCaptureGuard, name: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", name))
        .collect()
}

/// The one `move_rejected` with `reason`, checked for the fields every
/// BV-03 refusal carries.
fn rejected(
    capture: &crate::test_support::LogCaptureGuard,
    reason: &str,
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    item_id: i32,
) -> Captured {
    let event = capture
        .find_event(Level::WARN, "move_rejected", reason)
        .unwrap_or_else(|| panic!("move_rejected reason={reason}: {:#?}", capture.all()));
    assert_eq!(event.target, "bank");
    assert_fields(
        &event,
        &[
            ("event", "move_rejected".into()),
            ("account_id", account_id.to_string()),
            ("player_id", player_id.to_string()),
            ("entity_id", entity_id.to_string()),
            ("item_id", item_id.to_string()),
        ],
        &[],
    );
    event
}

/// Deposit then withdraw: the same `item_id` goes into the vault and comes
/// back, with never a second row, and each leg logs `move_accepted` with
/// its kind, both ends, the stacks and `bank_slots`. Fails with the session
/// wiring reverted (the vault refuses every move, as in BV-01).
#[tokio::test]
async fn deposit_then_withdraw_round_trips_the_same_item() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE, BASE + 1, 0x7000_B5E0);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 5).await;
    let client = in_world(entity_id, 40830);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 3, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(item, 17, 3, 5)],
        "deposited"
    );

    mv(
        &pool, &client, entity_id, player_id, item, 1, 4, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(item, 1, 4, 5)],
        "withdrawn"
    );

    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 2, "{:#?}", capture.all());
    for (event, kind, from, to) in [
        (&accepted[0], "deposit", ("1", "0"), ("17", "3")),
        (&accepted[1], "withdraw", ("17", "3"), ("1", "4")),
    ] {
        assert_eq!(event.level, Level::DEBUG);
        assert_fields(
            event,
            &[
                ("account_id", account_id.to_string()),
                ("player_id", player_id.to_string()),
                ("entity_id", entity_id.to_string()),
                ("item_id", item.to_string()),
                ("type_id", BANKABLE.to_string()),
                ("quantity", "5".into()),
                ("kind", kind.into()),
                ("source_container_id", from.0.into()),
                ("source_slot_id", from.1.into()),
                ("target_container_id", to.0.into()),
                ("target_slot_id", to.1.into()),
                ("source_stack_before", "5".into()),
                ("source_stack_after", "0".into()),
                ("target_stack_before", "0".into()),
                ("target_stack_after", "5".into()),
                ("bank_slots", "40".into()),
                ("banker_id", BANKER.to_string()),
                ("gm_override", "false".into()),
            ],
            &[],
        );
        assert!(event.fields.contains_key("distance"), "{event:#?}");
    }

    teardown(&pool, account_id, player_id).await;
}

/// Walked away after opening: the cell's verdict is `banker_out_of_range`,
/// so the deposit is refused, nothing moves, the player is told why and the
/// item snaps back. Fails if the base honoured any session regardless of
/// the verdict (the loot pin of CAT-D-02).
#[tokio::test]
async fn walking_away_after_opening_then_moving_is_rejected() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x10, BASE + 0x11, 0x7000_B5E1);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 5).await;
    let client = in_world(entity_id, 40831);
    let capture = LogCapture::install();
    let walked_away = VaultAccess::Closed {
        reason: "banker_out_of_range",
        banker_id: Some(BANKER),
        distance: Some(11.0),
    };

    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        item,
        17,
        0,
        -1,
        walked_away,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 0, 5)]);
    let event = rejected(
        &capture,
        "banker_out_of_range",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert_fields(
        &event,
        &[
            ("vault_end", "target".into()),
            ("banker_id", BANKER.to_string()),
            ("distance", "11.0".into()),
            ("target_container_id", "17".into()),
            ("source_container_id", "1".into()),
            ("stack_size", "5".into()),
        ],
        &[],
    );
    assert!(client.saw_text("too far from the Banker"), "feedback line");
    assert_eq!(
        client.transport.send_count_to(client.addr),
        2,
        "the feedback line, then the one-item snap-back"
    );

    // Out of the vault too: a banked item cannot be withdrawn from afar.
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 9, 2).await;
    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        banked,
        1,
        1,
        -1,
        walked_away,
    )
    .await;
    assert!(rows(&pool, player_id).await.contains(&(banked, 17, 9, 2)));
    let events = bank_events(&capture, "move_rejected");
    assert!(
        events
            .iter()
            .any(|e| e.has_field("item_id", &banked.to_string())
                && e.has_field("vault_end", "source")),
        "{events:#?}"
    );

    teardown(&pool, account_id, player_id).await;
}

/// The Banker despawned: `reason=banker_gone`, with its own line.
#[tokio::test]
async fn a_move_after_the_banker_left_is_rejected() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x20, BASE + 0x21, 0x7000_B5E2);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40832);
    let capture = LogCapture::install();
    let gone = VaultAccess::Closed {
        reason: "banker_gone",
        banker_id: Some(BANKER),
        distance: None,
    };

    mv(&pool, &client, entity_id, player_id, item, 17, 0, -1, gone).await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 0, 1)]);
    let event = rejected(
        &capture,
        "banker_gone",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert_fields(&event, &[("banker_id", BANKER.to_string())], &["distance"]);
    assert!(client.saw_text("The Banker has left"));

    teardown(&pool, account_id, player_id).await;
}

/// No session: the no-session guard with its feedback line (the log shape
/// is pinned in `allowlist_tests`).
#[tokio::test]
async fn a_move_with_no_session_tells_the_player() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x30, BASE + 0x31, 0x7000_B5E3);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40833);
    let capture = LogCapture::install();

    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        item,
        17,
        0,
        -1,
        VaultAccess::NO_SESSION,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 0, 1)]);
    rejected(
        &capture,
        "no_vault_session",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert!(client.saw_text("Your vault is closed"));

    teardown(&pool, account_id, player_id).await;
}

/// The slot bound is the player's `bank_slots`, read in the move
/// transaction, not the ceiling of 100: slot 40 of a 40-slot vault is
/// refused (`target_slot_beyond_bank_slots`, `bank_slots=40`), slot 39 is
/// accepted, and after the row grows to 50, slot 40 is accepted. Fails
/// with the bound removed (slot 40 lands) or read as a constant.
#[tokio::test]
async fn a_slot_at_or_above_bank_slots_is_rejected() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x40, BASE + 0x41, 0x7000_B5E4);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40834);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 40, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(item, 1, 0, 1)],
        "slot 40 of 40"
    );
    let event = rejected(
        &capture,
        "target_slot_beyond_bank_slots",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert_fields(
        &event,
        &[
            ("bank_slots", "40".into()),
            ("target_slot_id", "40".into()),
            ("vault_end", "target".into()),
        ],
        &[],
    );
    assert!(client.saw_text("That vault slot is locked. Your vault has 40 slots."));

    mv(
        &pool, &client, entity_id, player_id, item, 17, 39, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(item, 17, 39, 1)],
        "slot 39 of 40"
    );

    sqlx::query("UPDATE sgw_player SET bank_slots = 50 WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("expand");
    mv(
        &pool, &client, entity_id, player_id, item, 17, 40, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(item, 17, 40, 1)],
        "slot 40 of 50"
    );

    teardown(&pool, account_id, player_id).await;
}

/// Mission items cannot enter the vault (D-BV08): a mission-bag item whose
/// type also lists 17, from the main bag and from the mission bag, is
/// refused with `mission_item_not_bankable`. Fails with the rule removed:
/// `container_sets` alone admits it.
#[tokio::test]
async fn a_mission_item_is_rejected() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x50, BASE + 0x51, 0x7000_B5E5);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, MISSION, 1, 0, 1).await;
    let in_mission_bag = insert_item(&pool, player_id, MISSION, 2, 0, 1).await;
    let client = in_world(entity_id, 40835);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 0, -1, AT_BANKER,
    )
    .await;
    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        in_mission_bag,
        17,
        1,
        -1,
        AT_BANKER,
    )
    .await;

    assert_eq!(
        rows(&pool, player_id).await,
        vec![(carried, 1, 0, 1), (in_mission_bag, 2, 0, 1)]
    );
    let event = rejected(
        &capture,
        "mission_item_not_bankable",
        account_id,
        player_id,
        entity_id,
        carried,
    );
    assert_fields(&event, &[("type_id", MISSION.to_string())], &[]);
    assert_eq!(bank_events(&capture, "move_rejected").len(), 2);
    assert!(client.saw_text("Mission items cannot be stored in the vault."));

    teardown(&pool, account_id, player_id).await;
}

/// A swap out of the vault puts the occupant into it, so a mission-item
/// occupant refuses the swap: nothing moves.
#[tokio::test]
async fn a_swap_cannot_bring_a_mission_item_into_the_vault() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x60, BASE + 0x61, 0x7000_B5E6);
    setup(&pool, account_id, player_id).await;
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 0, 1).await;
    let quest = insert_item(&pool, player_id, MISSION, 1, 1, 1).await;
    let client = in_world(entity_id, 40836);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, banked, 1, 1, -1, AT_BANKER,
    )
    .await;

    assert_eq!(
        rows(&pool, player_id).await,
        vec![(quest, 1, 1, 1), (banked, 17, 0, 1)]
    );
    rejected(
        &capture,
        "mission_item_not_bankable",
        account_id,
        player_id,
        entity_id,
        banked,
    );

    teardown(&pool, account_id, player_id).await;
}

/// A split into the vault conserves the count: 10 becomes 6 carried and a
/// new vault row of 4, and `move_accepted` records both stacks.
#[tokio::test]
async fn a_split_into_the_vault_conserves_the_count() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x70, BASE + 0x71, 0x7000_B5E7);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 10).await;
    let client = in_world(entity_id, 40837);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 2, 4, AT_BANKER,
    )
    .await;

    let after = rows(&pool, player_id).await;
    assert_eq!(after.len(), 2, "{after:?}");
    assert_eq!(after[0], (item, 1, 0, 6));
    assert_eq!((after[1].1, after[1].2, after[1].3), (17, 2, 4));
    assert_eq!(
        after.iter().map(|r| r.3).sum::<i32>(),
        10,
        "count conserved"
    );
    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 1);
    assert_fields(
        &accepted[0],
        &[
            ("kind", "split".into()),
            ("quantity", "4".into()),
            ("source_stack_before", "10".into()),
            ("source_stack_after", "6".into()),
            ("target_stack_before", "0".into()),
            ("target_stack_after", "4".into()),
        ],
        &[],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A deposit onto a same-type vault stack merges (legacy
/// `Inventory.py:391-395`): a partial merge takes from the source, a whole
/// one deletes it and sends `onRemoveItem`. The count never changes.
#[tokio::test]
async fn a_deposit_merges_into_a_same_type_vault_stack() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x80, BASE + 0x81, 0x7000_B5E8);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, BANKABLE, 1, 0, 6).await;
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 5, 4).await;
    let client = in_world(entity_id, 40838);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, 2, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(carried, 1, 0, 4), (banked, 17, 5, 6)],
        "partial merge"
    );

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(banked, 17, 5, 10)],
        "whole merge deletes the source row"
    );

    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 2);
    assert_fields(
        &accepted[1],
        &[
            ("kind", "merge".into()),
            ("source_stack_before", "4".into()),
            ("source_stack_after", "0".into()),
            ("target_stack_before", "6".into()),
            ("target_stack_after", "10".into()),
        ],
        &[],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A merge that would overflow the stack (20) swaps instead, as before.
#[tokio::test]
async fn a_full_same_type_stack_swaps_instead_of_merging() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x90, BASE + 0x91, 0x7000_B5E9);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, BANKABLE, 1, 0, 6).await;
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 5, 18).await;
    let client = in_world(entity_id, 40839);

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, -1, AT_BANKER,
    )
    .await;

    assert_eq!(
        rows(&pool, player_id).await,
        vec![(banked, 1, 0, 18), (carried, 17, 5, 6)]
    );

    teardown(&pool, account_id, player_id).await;
}

/// A GM `.bank` session has no Banker and skips proximity: the deposit is
/// accepted and `move_accepted` says `gm_override=true`.
#[tokio::test]
async fn a_gm_session_deposits_without_a_banker() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0xA0, BASE + 0xA1, 0x7000_B5EA);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40840);
    let capture = LogCapture::install();
    let gm = VaultAccess::Open {
        banker_id: None,
        distance: None,
    };

    mv(&pool, &client, entity_id, player_id, item, 17, 0, -1, gm).await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 17, 0, 1)]);
    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 1);
    assert_fields(
        &accepted[0],
        &[("gm_override", "true".into())],
        &["banker_id", "distance"],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A move that never touches the vault logs no `move_accepted`: the event
/// is the bank's, not every move's.
#[tokio::test]
async fn a_carried_move_logs_no_bank_event() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0xB0, BASE + 0xB1, 0x7000_B5EB);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40841);
    let capture = LogCapture::install();

    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        item,
        1,
        7,
        -1,
        VaultAccess::NO_SESSION,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 7, 1)]);
    assert!(
        capture.all().iter().all(|c| c.target != "bank"),
        "{:#?}",
        capture.all()
    );

    teardown(&pool, account_id, player_id).await;
}
