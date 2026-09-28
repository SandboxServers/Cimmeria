//! `broadcast_to_org` (ORG-API, ORG-07): every online member, or only the
//! ranks holding a permission; the WARN seams.

use cimmeria_entity::organization::OrgPermission;
use tracing::Level;

use super::*;
use crate::base::organization::api::broadcast_to_org;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Unfiltered, every online member gets the call byte for byte and an
/// offline one is skipped; filtered by `OfficerNotes`, only the Leader and
/// the Officer (D-ORG08 masks) get it. The DEBUG `org.broadcast` row counts
/// the recipients.
#[tokio::test]
async fn live_db_broadcast_reaches_online_members_filtered_by_permission() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 26, 4, &["Org07 Broadcast"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org07 Broadcast", 0, &[1, 2, 3])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    for i in 0..3 {
        fx.online(i);
    }
    let args = vec![1u8, 2, 3];
    let capture = LogCapture::install();
    assert_eq!(broadcast_to_org(&fx.ctx(), cmd, 45, &args, None).await, 3);
    for i in 0..3 {
        assert_eq!(fx.calls_to(i), vec![(45, args.clone())], "member {i}");
    }
    assert!(fx.calls_to(3).is_empty(), "offline");
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.broadcast"))
        .expect("org.broadcast row");
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("recipients", "3") && row.has_field("roster_size", "4"));

    fx.clear_sent();
    let sent = broadcast_to_org(
        &fx.ctx(),
        cmd,
        47,
        &args,
        Some(OrgPermission::OFFICER_NOTES),
    )
    .await;
    assert_eq!(sent, 2);
    assert_eq!(fx.calls_to(0), vec![(47, args.clone())]);
    assert_eq!(fx.calls_to(1), vec![(47, args.clone())]);
    assert!(
        fx.calls_to(2).is_empty(),
        "an Initiate holds no OfficerNotes"
    );
    fx.teardown().await;
}

/// Negative seams: a member whose session cannot be reached is WARN
/// `org.send_failed` with the reason (the others still get it), and with no
/// database the broadcast is WARN `org.broadcast_failed` (`no_db`) and sends
/// nothing.
#[tokio::test]
async fn live_db_broadcast_failures_warn_with_reason() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 27, 2, &["Org07 Broadcast Fail"]).await;
    let team = fx.org(OrgType::Team, "Org07 Broadcast Fail", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    fx.entity_to_addr.lock().unwrap().remove(&fx.entity(1));
    let capture = LogCapture::install();
    assert_eq!(broadcast_to_org(&fx.ctx(), team, 45, &[9], None).await, 1);
    let warn = capture
        .find_event(
            Level::WARN,
            "could not be sent to a member",
            "entity_to_addr_miss",
        )
        .expect("org.send_failed");
    assert!(warn.has_field("target_player_id", &fx.player_id(1).to_string()));

    let no_db: Option<Arc<PgPool>> = None;
    let ctx = OrgCtx {
        db_pool: &no_db,
        ..fx.ctx()
    };
    let capture = LogCapture::install();
    assert_eq!(broadcast_to_org(&ctx, team, 45, &[9], None).await, 0);
    let warn = capture
        .find_event(Level::WARN, "roster could not be read", "no_db")
        .expect("org.broadcast_failed");
    assert!(warn.has_field("event", "org.broadcast_failed"));
    fx.teardown().await;
}
