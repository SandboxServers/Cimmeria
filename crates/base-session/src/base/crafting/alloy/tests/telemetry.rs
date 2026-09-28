//! The alloy verb's seams: the dispatcher reaches it, and a request it
//! cannot decide is a `lookup_failed` warning plus a visible line.

use std::sync::Arc;

use tracing::Level;

use super::super::{handle_alloy_in, AlloyRequest};
use super::fixture::*;
use crate::base::crafting::request::{handle_craft_request, CraftCtx};
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::session::{CraftingSessions, ManualScheduler};
use crate::base::crafting::test_packets::{decode_all, feedback_text};
use crate::base::crafting::test_players::{OneSession, SESSION_ACCOUNT_ID};
use crate::cell::messages::{CraftRequest, CraftVerb};
use crate::test_support::{require_db_or_skip, LogCapture};
use cimmeria_cell_catalog::crafting::CraftType;

/// With no database the verb cannot decide anything: the player reads the
/// "unavailable" line and the WARN names the phase and the player.
#[tokio::test]
async fn no_database_is_a_lookup_warning_and_a_visible_line() {
    const ENTITY: u32 = 0x7000_C3F0;
    const PLAYER: i32 = 0x7000_C3F1;
    let session = OneSession::new(ENTITY, 56390);
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(ManualScheduler::default()),
        Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
    ));
    let ctx = CraftCtx {
        db_pool: &None,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    let capture = LogCapture::install();
    let lower = [1, 2];
    let request = AlloyRequest {
        blueprint_id: ALLOY,
        current_tier_item_id: 3,
        lower_tier_items: &lower,
    };

    handle_alloy_in(&sessions, ENTITY, PLAYER, request, &ctx).await;

    let warn = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "lookup_failed"))
        .expect("lookup_failed");
    assert_eq!(warn.level, Level::WARN);
    for (key, value) in [
        ("verb", "alloying".to_string()),
        ("phase", "no_pool".to_string()),
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", PLAYER.to_string()),
        ("entity_id", ENTITY.to_string()),
        ("blueprint_id", ALLOY.to_string()),
    ] {
        assert!(warn.has_field(key, &value), "{key}={value}: {warn:#?}");
    }
    assert_eq!(sessions.pending(ENTITY), 0);
    let calls = decode_all(&session.typed.filter_to(session.addr));
    assert_eq!(calls.len(), 1, "one line, no resync without a database");
    assert_eq!(
        feedback_text(&calls[0]),
        "Alloying is unavailable right now. Nothing was changed."
    );
}

/// The base dispatcher routes `alloying` past the station gate to this
/// verb: an unlearned blueprint is refused by the alloy rules, not with
/// "not available yet".
#[tokio::test]
async fn live_db_the_dispatcher_routes_alloying_to_the_alloy_verb() {
    let pool = require_db_or_skip!();
    let f = Fixture::new(&pool, 20).await;
    let capture = LogCapture::install();
    let request = CraftRequest {
        entity_id: f.entity_id,
        player_id: f.player_id,
        verb: CraftVerb::Alloy {
            blueprint_id: OTHER_ALLOY,
            current_tier_item_id: 0,
            lower_tier_items: vec![],
        },
        allowed: CraftType::Alloying.bit(),
    };
    handle_craft_request(request, &f.ctx()).await;
    let rejected = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "rejected"))
        .expect("rejected");
    assert!(
        rejected.has_field("reason", "unknown_blueprint"),
        "{rejected:#?}"
    );
    assert!(rejected.has_field("verb", "alloying"), "{rejected:#?}");
    f.cleanup().await;
}

/// More elementary ids than the page has slots only comes from a forged
/// packet: dropped with a `malformed` WARN, no line, nothing read.
#[tokio::test]
async fn an_overlong_elementary_list_is_dropped_as_malformed() {
    const ENTITY: u32 = 0x7000_C3F4;
    const PLAYER: i32 = 0x7000_C3F5;
    let session = OneSession::new(ENTITY, 56391);
    let sessions = Arc::new(CraftingSessions::new(
        Box::new(ManualScheduler::default()),
        Box::new(|| Box::new(ScriptedRng::new(vec![0.5]))),
    ));
    let ctx = CraftCtx {
        db_pool: &None,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    let capture = LogCapture::install();
    let lower: Vec<i32> = (1..=11).collect();
    let request = AlloyRequest {
        blueprint_id: ALLOY,
        current_tier_item_id: 100,
        lower_tier_items: &lower,
    };

    handle_alloy_in(&sessions, ENTITY, PLAYER, request, &ctx).await;

    let events: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|e| e.target == "crafting" && e.fields.contains_key("event"))
        .collect();
    assert_eq!(events.len(), 1, "{events:#?}");
    let malformed = &events[0];
    assert_eq!(malformed.level, Level::WARN);
    for (key, value) in [
        ("event", "malformed".to_string()),
        ("reason", "too_many_elementary_items".to_string()),
        ("count", "11".to_string()),
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", PLAYER.to_string()),
        ("entity_id", ENTITY.to_string()),
    ] {
        assert!(
            malformed.has_field(key, &value),
            "{key}={value}: {malformed:#?}"
        );
    }
    assert!(session.typed.filter_to(session.addr).is_empty(), "no line");
}
