//! Negative-log and transition-log guards (TESTING.md type 12): a typed
//! miss logs exactly one WARN with its `reason`, and a change logs a DEBUG
//! `event` with its before/after values and `rows_affected`.

use cimmeria_entity::organization::{OrgPermission, OrgRank, OrgType};
use tracing::Level;

use super::super::super::api::{lock_org, member_access_locked, OrgAccess};
use super::super::{add_member, set_rank, set_rank_permissions, set_text, OrgTextTarget};
use super::*;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};

fn org_events(capture: &LogCaptureGuard, level: Level, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.level == level && c.target == "org" && c.has_field("event", event))
        .collect()
}

/// Each typed miss is one WARN on `org` with the function as `event` and a
/// closed `reason`, and the ORG-API lookups warn on their own misses.
#[tokio::test]
async fn typed_misses_log_one_warn_with_reason() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 23, 2, &["Org02 Warn"]).await;
    let (p0, outsider) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Team, "Org02 Warn", p0).await.org_id;
    let gone = org + 1_000_000;

    let capture = LogCapture::install();
    let mut tx = pool.begin().await.unwrap();
    as_sys!(set_rank, tx, org, outsider, OrgRank::SENIOR_MEMBER).unwrap_err();
    assert!(OrgAccess::system(&mut tx, gone, TEST_ACTOR)
        .await
        .unwrap()
        .is_none());
    as_sys!(set_rank, tx, org, p0, OrgRank::MEMBER).unwrap_err();
    as_sys!(set_text, tx, org, OrgTextTarget::Motd, "a\u{202E}b").unwrap_err();
    assert!(lock_org(&mut tx, gone).await.unwrap().is_none());
    assert!(member_access_locked(&mut tx, org, outsider)
        .await
        .unwrap()
        .is_none());
    tx.rollback().await.unwrap();

    for (event, reason, org_id, player_id) in [
        ("set_rank", "not_a_member", org, Some(outsider)),
        ("system_access", "no_such_org", gone, None),
        ("set_rank", "leader_pinned", org, Some(p0)),
        ("set_text", "bidi_control", org, None),
        ("lock_org", "no_such_org", gone, None),
        ("member_access_locked", "not_a_member", org, Some(outsider)),
    ] {
        let hits: Vec<Captured> = org_events(&capture, Level::WARN, event)
            .into_iter()
            .filter(|c| c.has_field("reason", reason))
            .collect();
        assert_eq!(hits.len(), 1, "{event}/{reason}: {:#?}", capture.all());
        assert!(hits[0].has_field("org_id", &org_id.to_string()));
        if let Some(p) = player_id {
            assert!(hits[0].has_field("player_id", &p.to_string()));
        }
    }
    // The persistence layer's own lock miss is not a second WARN.
    assert_eq!(
        org_events(&capture, Level::WARN, "lock_org").len(),
        1,
        "system_access's miss must not also warn as lock_org"
    );

    teardown(&pool, &fx).await;
}

/// Changes log DEBUG with before/after values; text is logged as lengths.
#[tokio::test]
async fn changes_log_debug_with_before_and_after() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 24, 2, &["Org02 Debug"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 Debug", p0)
        .await
        .org_id;

    let capture = LogCapture::install();
    let mut tx = pool.begin().await.unwrap();
    as_sys!(add_member, tx, org, p1, OrgRank::INITIATE).unwrap();
    as_sys!(set_rank, tx, org, p1, OrgRank::OFFICER).unwrap();
    let old = as_sys!(
        set_rank_permissions,
        tx,
        org,
        OrgRank::OFFICER,
        OrgPermission::MOTD
    )
    .unwrap();
    as_sys!(set_text, tx, org, OrgTextTarget::Motd, "hello").unwrap();
    as_sys!(set_text, tx, org, OrgTextTarget::Motd, "hi").unwrap();
    tx.commit().await.unwrap();

    let added = &org_events(&capture, Level::DEBUG, "add_member")[0];
    assert!(added.has_field("player_id", &p1.to_string()));
    assert!(added.has_field("account_id", &fx.account_id.to_string()));
    assert!(added.has_field("rows_affected", "1"));

    let rank = &org_events(&capture, Level::DEBUG, "set_rank")[0];
    assert!(rank.has_field("from_rank", "1") && rank.has_field("to_rank", "6"));

    let perms = &org_events(&capture, Level::DEBUG, "set_rank_permissions")[0];
    assert!(perms.has_field("from_mask", &old.bits().to_string()));
    assert!(perms.has_field("to_mask", &OrgPermission::MOTD.bits().to_string()));

    let texts = org_events(&capture, Level::DEBUG, "set_text");
    assert_eq!(texts.len(), 2);
    assert!(texts[1].has_field("from_units", "5") && texts[1].has_field("to_units", "2"));
    assert!(
        capture
            .all()
            .iter()
            .all(|c| !c.fields.values().any(|v| v.contains("hello"))),
        "text is logged as lengths, never the text"
    );

    teardown(&pool, &fx).await;
}
