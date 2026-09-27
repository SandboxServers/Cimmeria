//! The server-failure seams of an induction verb: no database at the
//! request or at completion is a `lookup_failed` WARN and a visible line.

use tracing::Level;

use super::*;
use crate::base::crafting::test_packets::{decode_all, feedback_text};
use crate::base::crafting::test_players::{OneSession, SESSION_ACCOUNT_ID};
use crate::test_support::LogCapture;

const ENTITY: u32 = 0x7000_C2F0;
const PLAYER: i32 = 0x7000_C2F1;

#[tokio::test]
async fn a_request_with_no_database_warns_and_tells_the_player() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 57290);
    let ctx = CraftCtx {
        db_pool: &None,
        cell_tx: &None,
        transport: &session.transport,
        connected: &session.connected,
        entity_to_addr: &session.entity_to_addr,
    };
    let inputs = load_request_inputs("research", "Research", ENTITY, PLAYER, &[1], &ctx).await;
    assert!(inputs.is_none());

    let warn = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", "lookup_failed"))
        .expect("lookup_failed WARN");
    for (k, v) in [
        ("phase", "no_pool".to_string()),
        ("verb", "research".to_string()),
        ("account_id", SESSION_ACCOUNT_ID.to_string()),
        ("player_id", PLAYER.to_string()),
        ("entity_id", ENTITY.to_string()),
    ] {
        assert!(warn.has_field(k, &v), "{k}={v}: {warn:#?}");
    }
    let lines: Vec<String> = decode_all(&session.typed.filter_to(session.addr))
        .iter()
        .map(feedback_text)
        .collect();
    assert_eq!(
        lines,
        vec!["Research is unavailable right now. Nothing was changed.".to_string()]
    );
}

#[tokio::test]
async fn a_completion_with_no_database_warns_and_refuses_the_job() {
    let capture = LogCapture::install();
    let session = OneSession::new(ENTITY, 57291);
    let env = InductionEnv {
        db_pool: None,
        cell_tx: None,
        transport: session.transport.clone(),
        connected: session.connected.clone(),
        entity_to_addr: session.entity_to_addr.clone(),
    };
    let ids = JobIds {
        job_id: 77,
        verb: "reverseEngineer",
        account_id: 55,
        player_id: PLAYER,
        entity_id: ENTITY,
    };
    assert!(state_at_completion(&env, &ids).await.is_none());

    let warn = capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", "lookup_failed"))
        .expect("lookup_failed WARN");
    for (k, v) in [
        ("phase", "completion_state"),
        ("job_id", "77"),
        ("account_id", "55"),
        ("verb", "reverseEngineer"),
    ] {
        assert!(warn.has_field(k, v), "{k}={v}: {warn:#?}");
    }
    let rejected = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "rejected"))
        .expect("rejected");
    assert!(rejected.has_field("reason", "induction_failed"));
    let lines: Vec<String> = decode_all(&session.typed.filter_to(session.addr))
        .iter()
        .map(feedback_text)
        .collect();
    assert_eq!(
        lines,
        vec!["Crafting failed. Nothing was used.".to_string()]
    );
}
