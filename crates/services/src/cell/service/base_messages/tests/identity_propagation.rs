//! Regression guards for the stable-identity log convention
//! (`docs/architecture/instrumentation-discipline.md` §Rule 5).
//!
//! # The bug shape these reproduce
//!
//! Before this convention, `account_id` appeared **only** on auth and
//! character-select log lines. The moment a character entered the world every
//! downstream log switched to `entity_id` / `caller_id` — and `entity_id` is
//! not an identity: it is a recycled per-space slot integer that a later
//! connection can be handed after this one releases it. Answering "what did
//! account 6 do" therefore required matching `access_level` values between a
//! login event and a console command and correlating by wall clock, which
//! breaks as soon as two players are online at once.
//!
//! So a happy-path assertion that the event merely *fired* is worthless here —
//! the event always fired. Every guard below asserts on the **presence and
//! value of the identity fields**, and the NPC guard additionally asserts
//! their **absence**, so a "fix" that papers over an unknown id with a `0`
//! sentinel also trips.
//!
//! # What each guard fails on
//!
//! | Revert | Guard that trips |
//! |---|---|
//! | drop `account_id`/`player_id` from the movement reject warn | [`movement_reject_carries_account_and_player_id`] |
//! | drop the `CreateEntity` identity stamp | [`movement_reject_carries_account_and_player_id`] |
//! | `unwrap_or(0)` instead of passing the `Option` through | [`npc_movement_reject_emits_no_identity_fields`] |
//! | drop identity from the disconnect teardown log | [`disconnect_carries_identity_resolved_before_teardown`] |
//! | resolve identity *after* teardown instead of before | [`disconnect_carries_identity_resolved_before_teardown`] |

use super::*;
use crate::test_support::LogCapture;
use tracing::Level;

const WORLD: &str = "Castle_CellBlock";
const SPACES_XML: &str = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;

/// The account/player pair this fixture threads end-to-end. `6` is the
/// account from the SigNoz investigation that motivated the convention.
const ACCOUNT_ID: u32 = 6;
const PLAYER_ID: i32 = 12;

/// Far outside the fallback AABB (`[-10_000, 10_000]` per axis), so the
/// bounds layer rejects and the warn fires.
const OUT_OF_BOUNDS: [f32; 3] = [50_000.0, 5.0, 20.0];
const SPAWN: [f32; 3] = [10.0, 5.0, 20.0];

fn manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(SPACES_XML).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

/// Drive the real `BaseToCellMsg::CreateEntity` arm rather than calling
/// `SpaceManager::create_entity` directly — the identity stamp lives in the
/// message handler, so a direct create would bypass the thing under test.
async fn create_via_base_message(
    mgr: &mut SpaceManager,
    entity_id: u32,
    account_id: Option<u32>,
    player_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    handle_base_message(
        BaseToCellMsg::CreateEntity {
            entity_id,
            world_name: WORLD.to_string(),
            position: SPAWN,
            rotation: [0.0; 3],
            destination_space_id: None,
            account_id,
            player_id,
            reply_tx,
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx.await.expect("cell must ack the create");
}

async fn send_out_of_bounds_move(
    mgr: &mut SpaceManager,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id,
            claimed_space_id: 0,
            position: OUT_OF_BOUNDS,
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
}

/// A movement reject for a player must name the account and character, not
/// just the recycled entity slot.
///
/// This is the end-to-end seam: identity enters the cell on `CreateEntity`,
/// is stamped onto the `CellEntity`, and is re-resolved by the reject branch
/// through `SpaceManager::player_identity`. Breaking **any** link in that
/// chain drops the fields and fails here.
#[tokio::test]
async fn movement_reject_carries_account_and_player_id() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(16);

    create_via_base_message(&mut mgr, 7777, Some(ACCOUNT_ID), Some(PLAYER_ID), &tx).await;
    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("the bounds reject warn must still fire");

    assert!(
        event.has_field("account_id", "6"),
        "movement reject must carry account_id=6 -- without it, a snap-back \
         cannot be attributed to an account and `entity_id` alone is a \
         recycled slot that another player may hold later; got {event:#?}"
    );
    assert!(
        event.has_field("player_id", "12"),
        "movement reject must carry player_id=12 so the specific character is \
         identifiable on a multi-character account; got {event:#?}"
    );
    // The pre-existing correlator must survive alongside the new ones --
    // this is additive, not a replacement.
    assert!(
        event.has_field("entity_id", "7777"),
        "entity_id must still be emitted for per-space debugging; got {event:#?}"
    );
}

/// An NPC has no account. Its reject must emit **neither** identity field
/// rather than a `0`/`"None"` placeholder.
///
/// This is the guard that makes the `Option`-passthrough contract real: a
/// well-meaning `unwrap_or(0)` would satisfy the player guard above while
/// making `account_id = 0` an un-filterable value that collides with nothing
/// and matches every NPC in the log store.
#[tokio::test]
async fn npc_movement_reject_emits_no_identity_fields() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(16);

    // No identity on the create -- exactly what the spawner path does.
    create_via_base_message(&mut mgr, 4242, None, None, &tx).await;
    send_out_of_bounds_move(&mut mgr, 4242, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("the bounds reject warn must fire for NPCs too");

    assert!(
        !event.fields.contains_key("account_id"),
        "an NPC reject must omit account_id entirely -- emitting a sentinel \
         (0, \"None\") pollutes the field's values and makes `account_id = 0` \
         match every NPC in the log store; got {event:#?}"
    );
    assert!(
        !event.fields.contains_key("player_id"),
        "an NPC reject must omit player_id entirely; got {event:#?}"
    );
    assert!(
        event.has_field("entity_id", "4242"),
        "entity_id is still the right correlator for an NPC; got {event:#?}"
    );
}

/// `InitPlayerState` arrives only after `onClientReady`. A reject during the
/// world-entry window (before it) must still be attributable — that is the
/// whole reason identity is threaded through `CreateEntity` rather than
/// waiting for `InitPlayerState`.
#[tokio::test]
async fn identity_is_available_before_init_player_state() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(16);

    create_via_base_message(&mut mgr, 7777, Some(ACCOUNT_ID), Some(PLAYER_ID), &tx).await;
    // Deliberately NO InitPlayerState here.
    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("reject warn must fire");
    assert!(
        event.has_field("account_id", "6") && event.has_field("player_id", "12"),
        "identity must come from the CreateEntity stamp, not InitPlayerState -- \
         otherwise every log in the multi-second world-entry window (and the \
         fresh entity a gate-travel creates) is un-attributable; got {event:#?}"
    );
}

/// The teardown log is the last line of a session that can still name the
/// account, and it runs *after* the entity has been removed from its space.
/// Resolving identity lazily at the log statement would report UNKNOWN.
#[tokio::test]
async fn disconnect_carries_identity_resolved_before_teardown() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(64);

    create_via_base_message(&mut mgr, 7777, Some(ACCOUNT_ID), Some(PLAYER_ID), &tx).await;
    handle_base_message(
        BaseToCellMsg::ConnectEntity { entity_id: 7777 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 7777 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    // Sanity: the entity really is gone, so a lazy lookup at the log
    // statement would have resolved to UNKNOWN.
    assert!(
        mgr.get_entity(7777).is_none(),
        "fixture precondition: disconnect must have destroyed the entity"
    );

    let event = capture
        .find_message(Level::DEBUG, "Entity disconnected and destroyed")
        .expect("the disconnect teardown log must still fire");
    assert!(
        event.has_field("account_id", "6") && event.has_field("player_id", "12"),
        "the session-closing log must be attributable to the account; moving \
         the identity lookup after the teardown silently degrades it to \
         UNKNOWN and reintroduces the gap; got {event:#?}"
    );
}

/// Connect is the counterpart bookend: a session's first in-world log line
/// must already carry identity.
#[tokio::test]
async fn connect_carries_identity() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(64);

    create_via_base_message(&mut mgr, 7777, Some(ACCOUNT_ID), Some(PLAYER_ID), &tx).await;
    handle_base_message(
        BaseToCellMsg::ConnectEntity { entity_id: 7777 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    let event = capture
        .find_message(Level::DEBUG, "Entity connected (player)")
        .expect("the space-manager connect log must still fire");
    assert!(
        event.has_field("account_id", "6") && event.has_field("player_id", "12"),
        "connect must name the account so a session's in-world span is \
         bounded by two attributable log lines; got {event:#?}"
    );
}
