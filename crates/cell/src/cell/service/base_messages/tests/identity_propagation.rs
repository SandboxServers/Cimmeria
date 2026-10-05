//! Regression guards for the stable-identity log convention
//! (`docs/architecture/instrumentation-discipline.md` §Rule 5), and for the
//! names that pair with it (§Rule 6: `player_name`, `account_name`, and
//! `entity_name` next to `entity_id`).
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
//! | drop the names from `PlayerIdentity` or the `CreateEntity` name stamp | every player guard (each asserts the names via [`assert_identity`]) |
//! | emit `""` / `"unknown"` for an NPC's missing name | [`npc_movement_reject_emits_no_identity_fields`] |
//! | `unwrap_or(0)` instead of passing the `Option` through | [`npc_movement_reject_emits_no_identity_fields`] |
//! | drop identity from the disconnect teardown log | [`disconnect_carries_identity_resolved_before_teardown`] |
//! | resolve identity *after* teardown instead of before | [`disconnect_carries_identity_resolved_before_teardown`] |
//! | drop identity from the `Recovered` warn | [`movement_recovery_carries_account_and_player_id`] |
//! | drop identity from the `CorrectionSuppressed` error | [`correction_suppressed_carries_account_and_player_id`] |
//! | stop threading identity into `send_snap_back` | [`snap_back_send_failure_carries_account_and_player_id`] |
//! | drop a name from the `session.start` line (NT-24) | [`session_start_names_the_account_the_character_and_the_archetype`] |
//!
//! Each outcome branch resolves identity for itself, so a guard on one says
//! nothing about the others — hence one test per branch rather than one for
//! the handler.

use super::*;
use crate::test_support::LogCapture;
use cimmeria_entity::movement_validation::MovementValidator;
use tracing::Level;

const WORLD: &str = "Castle_CellBlock";
const SPACES_XML: &str = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;

/// The account/player pair this fixture threads end-to-end. `6` is the
/// account from the SigNoz investigation that motivated the convention.
const ACCOUNT_ID: u32 = 6;
const PLAYER_ID: i32 = 12;
/// The names that pair with the two IDs (Rule 6).
const ACCOUNT_NAME: &str = "sgc_login";
const PLAYER_NAME: &str = "Teal'c";

/// Assert the full Rule 5 + Rule 6 identity on one event: both IDs, both
/// names, and the entity's name next to `entity_id`.
fn assert_identity(event: &crate::test_support::Captured, why: &str) {
    for (key, want) in [
        ("account_id", "6"),
        ("player_id", "12"),
        ("account_name", ACCOUNT_NAME),
        ("player_name", PLAYER_NAME),
        ("entity_name", PLAYER_NAME),
    ] {
        assert!(
            event.has_field(key, want),
            "{why}: expected {key}={want}; got {event:#?}"
        );
    }
}

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
///
/// `player` stamps the fixture's IDs and names, as the login path does; an
/// NPC create carries none, as the spawner does.
async fn create_via_base_message(
    mgr: &mut SpaceManager,
    entity_id: u32,
    player: bool,
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
            account_id: player.then_some(ACCOUNT_ID),
            player_id: player.then_some(PLAYER_ID),
            account_name: player.then(|| ACCOUNT_NAME.to_string()),
            player_name: player.then(|| PLAYER_NAME.to_string()),
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

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("the bounds reject warn must still fire");

    // Without the IDs a snap-back cannot be attributed to an account, and
    // `entity_id` alone is a recycled slot another player may hold later;
    // without the names nobody reading the line knows who it was.
    assert_identity(
        &event,
        "movement reject must name the account and character",
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
    create_via_base_message(&mut mgr, 4242, false, &tx).await;
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
    for key in ["account_name", "player_name", "entity_name"] {
        assert!(
            !event.fields.contains_key(key),
            "an NPC with no name must omit {key} entirely -- a missing name is \
             left out, never written as \"\" or \"unknown\"; got {event:#?}"
        );
    }
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

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    // Deliberately NO InitPlayerState here.
    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_reject", "bounds")
        .expect("reject warn must fire");
    assert_identity(
        &event,
        "identity must come from the CreateEntity stamp, not InitPlayerState -- \
         otherwise every log in the multi-second world-entry window (and the \
         fresh entity a gate-travel creates) is un-attributable",
    );
}

/// The names `CreateEntity` carries are for logs only. Game lookups by
/// name (`.goto`, `.summon`, tells) must see a loading player exactly as
/// before NT-02: not found until `InitPlayerState` sets `character_name`.
#[tokio::test]
async fn birth_names_do_not_change_name_lookups_while_loading() {
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(16);

    create_via_base_message(&mut mgr, 7777, true, &tx).await;

    assert_eq!(
        mgr.find_online_player_by_name(PLAYER_NAME),
        cimmeria_cell_world::cell::space_manager::PlayerNameLookup::NotFound,
        "a player still loading was NotFound before NT-02 and must stay so"
    );
    assert_eq!(
        mgr.get_entity(7777).unwrap().character_name,
        None,
        "the game's name is InitPlayerState's to set"
    );
    assert_eq!(
        mgr.player_identity(7777).player_name,
        Some(PLAYER_NAME),
        "while the log identity is named from birth"
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

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    handle_base_message(
        BaseToCellMsg::ConnectEntity { entity_id: 7777 },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    handle_base_message(
        disconnect_entity_msg(7777),
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
    assert_identity(
        &event,
        "the session-closing log must be attributable to the account; moving \
         the identity lookup after the teardown silently degrades it to \
         UNKNOWN and reintroduces the gap",
    );
}

/// Connect is the counterpart bookend: a session's first in-world log line
/// must already carry identity.
#[tokio::test]
async fn connect_carries_identity() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(64);

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
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
    assert_identity(
        &event,
        "connect must name the account so a session's in-world span is \
         bounded by two attributable log lines",
    );
}

/// A far-outside-the-AABB position for the *entity*, so its own authoritative
/// position stops being a usable snap-back target and the reject resolves to
/// `Recovered` instead of `Rejected`.
const STRANDED: [f32; 3] = [-50_000.0, 5.0, 20.0];

/// The recovery warn is the line that says "the server moved a player it did
/// not intend to move". It must name who.
///
/// `Recovered` is a distinct branch from the ordinary reject, so it does not
/// inherit that branch's identity lookup — dropping the fields here is
/// invisible to [`movement_reject_carries_account_and_player_id`].
#[tokio::test]
async fn movement_recovery_carries_account_and_player_id() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(16);

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    // Server-authoritative write that strands the entity outside the AABB —
    // the `.gotoxyz` / stale-persisted-position shape.
    mgr.update_entity_position(7777, STRANDED, [0, 0, 0], [0.0; 3]);
    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(Level::WARN, "movement.validation_recovered", "bounds")
        .expect("the recovery warn must fire when the snap target is unusable");
    assert_identity(
        &event,
        "a server-initiated relocation must be attributable to the account \
         whose avatar was moved",
    );
}

/// The suppression log is the *last* thing emitted for a client that is stuck,
/// and it is an `error` precisely because an operator is meant to act on it.
/// Acting on it starts with knowing whose session it is.
#[tokio::test]
async fn correction_suppressed_carries_account_and_player_id() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(64);

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    // The entity's own position stays sound (spawn), so every reject is an
    // ordinary correction until the budget runs out and suppression kicks in.
    for _ in 0..=MovementValidator::MAX_SNAP_BACK_CORRECTIONS {
        send_out_of_bounds_move(&mut mgr, 7777, &tx).await;
    }

    let event = capture
        .find_event(Level::ERROR, "movement.correction_suppressed", "bounds")
        .expect("the suppression error must fire once the budget is spent");
    assert_identity(
        &event,
        "the stuck-client error must name the session an operator has to go \
         look at",
    );
}

/// The snap-back delivery failure is what a player experiencing a stuck or
/// looping correction actually surfaces as, so it is the single most
/// operationally important line in this handler — and the identity has to be
/// *threaded into* `send_snap_back`, because the entity may already be gone
/// by the time the send fails.
#[tokio::test]
async fn snap_back_send_failure_carries_account_and_player_id() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, rx) = mpsc::channel(16);
    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    // Base side is gone: the correction cannot be delivered.
    drop(rx);

    send_out_of_bounds_move(&mut mgr, 7777, &tx).await;

    let event = capture
        .find_event(
            Level::WARN,
            "movement.snap_back_send_failed",
            "snap_back_send_failed",
        )
        .expect("the undelivered-correction warn must fire on a closed channel");
    assert_identity(
        &event,
        "the player left desynced must be identifiable from this line alone — \
         it is the one an operator reaches for when a player reports being \
         stuck",
    );
    // Rule 6 (NT-23): the space the player was snapped in is named too.
    assert!(
        event.has_field("world", WORLD),
        "the snap-back failure must name the world; got {event:#?}"
    );
}

/// `session.start` (`player entered world`) is the login half of the
/// session timeline that `session.end` closes on the base. It names the
/// account, the character and the archetype next to their IDs (Rule 6,
/// NT-24); before, it carried only `character_name` and the raw IDs.
#[tokio::test]
async fn session_start_names_the_account_the_character_and_the_archetype() {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let (tx, _rx) = mpsc::channel(256);

    create_via_base_message(&mut mgr, 7777, true, &tx).await;
    handle_base_message(
        BaseToCellMsg::InitPlayerState {
            entity_id: 7777,
            player_id: PLAYER_ID,
            account_id: ACCOUNT_ID,
            world_name: WORLD.into(),
            archetype_id: 1,
            saved_missions: vec![],
            abilities: vec![],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            access_level: 0,
            known_stargates: vec![],
            tree_progress: Default::default(),
            level: 1,
            character_name: Some(PLAYER_NAME.into()),
            body_set: None,
            looted_containers: Vec::new(),
        },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    let event = capture
        .find_message(Level::INFO, "player entered world")
        .unwrap_or_else(|| panic!("no session.start line: {:#?}", capture.all()));
    assert_identity(&event, "session.start must name who entered the world");
    assert!(
        event.has_field("archetype_name", "Soldier"),
        "the archetype is named next to its ordinal: {event:#?}"
    );
}
