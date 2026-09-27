//! Regression guard: the GM `.`-console audit log must name the account, not
//! just the entity slot.
//!
//! # The bug shape
//!
//! The audit line used to carry `entity_id` + `access_level` only. Attributing
//! a GM command to a person meant matching the `access_level` value against a
//! login event and correlating by wall clock — which silently picks the wrong
//! GM the moment two privileged accounts are online, because `access_level` is
//! not unique and `entity_id` is a recycled per-space slot.
//!
//! A happy-path assertion that the audit line fired would not catch that; the
//! line always fired. These guards assert on the identity fields themselves.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 5.

use super::*;
use crate::test_support::LogCapture;
use tracing::Level;

const AUDIT_MSG: &str = "GM .-console command accepted";

/// Account 6 / character 12 — the session from the SigNoz investigation that
/// motivated the convention.
const ACCOUNT_ID: u32 = 6;
const PLAYER_ID: i32 = 12;

#[tokio::test]
async fn gm_console_audit_log_carries_account_and_player_id() {
    let capture = LogCapture::install();
    let (mut mgr, gm, _npc) = setup();
    // Stamp the identity the way `BaseToCellMsg::CreateEntity` /
    // `InitPlayerState` do at world entry — `setup()` builds the entity
    // directly and so skips that path.
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.account_id = Some(ACCOUNT_ID);
        e.player_id = Some(PLAYER_ID);
    }

    let (tx, _rx) = mpsc::channel(32);
    // `.speed` is a Target::None command, so it dispatches off the GM's own
    // entity without needing a specific target shape.
    super::super::dispatch::handle_console_command(
        gm,
        ".speed 5",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let event = capture
        .find_message(Level::INFO, AUDIT_MSG)
        .expect("the GM audit line must still fire on an accepted command");

    assert!(
        event.has_field("account_id", "6"),
        "the GM audit trail must name the account; without it, attributing a \
         command means guessing from access_level + wall clock, which picks \
         the wrong GM when two are online; got {event:#?}"
    );
    assert!(
        event.has_field("player_id", "12"),
        "the GM audit trail must name the character so a multi-character \
         account is still pinned to one avatar; got {event:#?}"
    );
    // Additive: the pre-existing audit fields must survive.
    assert!(
        event.has_field("entity_id", &gm.to_string()) && event.has_field("access_level", "2"),
        "entity_id and access_level must still be emitted alongside the new \
         identity fields; got {event:#?}"
    );
}

/// A caller with no stamped identity (an un-hydrated entity, or a future
/// non-player caller) must emit no identity fields rather than a `0`
/// sentinel that would match every such caller in the log store.
#[tokio::test]
async fn gm_console_audit_omits_identity_when_unknown() {
    let capture = LogCapture::install();
    let (mut mgr, gm, _npc) = setup();
    // Deliberately leave account_id/player_id unset.

    let (tx, _rx) = mpsc::channel(32);
    super::super::dispatch::handle_console_command(
        gm,
        ".speed 5",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let event = capture
        .find_message(Level::INFO, AUDIT_MSG)
        .expect("the GM audit line must still fire");

    assert!(
        !event.fields.contains_key("account_id") && !event.fields.contains_key("player_id"),
        "an unstamped caller must omit both identity fields entirely -- a 0 \
         sentinel is indistinguishable from a real id in a log query; got {event:#?}"
    );
}
