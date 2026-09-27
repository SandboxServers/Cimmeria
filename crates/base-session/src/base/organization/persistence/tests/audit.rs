//! The member-delete trigger's audit rows (`sgw_organization_events`) and
//! their export to the `org` log target: logged, then stamped, by whichever
//! path reaches the row first; a stamped row is never logged again.

use cimmeria_entity::organization::{OrgRank, OrgType};
use tracing::Level;

use super::super::super::audit::{export_committed, sweep_unexported, ExportSource};
use super::super::super::character_delete::delete_character;
use super::super::{add_member, remove_member, AfterRemoval};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// `(event, reason, from_player_id, to_player_id, exported)` of every audit
/// row for `org_id`, oldest first.
async fn audit_rows(pool: &PgPool, org_id: i32) -> Vec<(String, String, i32, Option<i32>, bool)> {
    sqlx::query_as(
        "SELECT event, reason, from_player_id, to_player_id, exported_at IS NOT NULL \
         FROM sgw_organization_events WHERE org_id = $1 ORDER BY org_event_id",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn add(pool: &PgPool, org_id: i32, player_id: i32, rank: OrgRank) {
    let mut tx = pool.begin().await.unwrap();
    as_sys!(add_member, tx, org_id, player_id, rank).unwrap();
    tx.commit().await.unwrap();
}

/// The captured export log for `event` in `org_id`, at `level`.
fn export_log(
    capture: &crate::test_support::LogCaptureGuard,
    level: Level,
    org_id: i32,
    event: &str,
) -> Option<crate::test_support::Captured> {
    capture.all().into_iter().find(|c| {
        c.level == level
            && c.target == "org"
            && c.message_contains("member-delete trigger")
            && c.has_field("event", event)
            && c.has_field("org_id", &org_id.to_string())
    })
}

/// The character-delete handler's path: the trigger writes one row per
/// organization it changed, `delete_character` logs each at INFO with both
/// identities right after its commit and stamps it, and a later sweep finds
/// nothing left to log.
#[tokio::test]
async fn character_delete_exports_trigger_events_once() {
    let pool = require_db_or_skip!();
    let names = ["Org02 Audit Cmd", "Org02 Audit Team"];
    let fx = setup(&pool, 20, 2, &names).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let cmd = create(&pool, OrgType::Command, names[0], p0).await.org_id;
    add(&pool, cmd, p1, OrgRank::OFFICER).await;
    let team = create(&pool, OrgType::Team, names[1], p0).await.org_id;

    let capture = LogCapture::install();
    let deletion = delete_character(&pool, p0, fx.account_id)
        .await
        .expect("character delete");
    assert!(deletion.deleted);
    // One row per organization, in the order the cascade deleted the member
    // rows; compare as a set.
    let mut got: Vec<(i32, &str, &str)> = deletion
        .org_events
        .iter()
        .map(|r| (r.org_id, r.event.as_str(), r.reason.as_str()))
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            (cmd, "leader_changed", "character_deleted"),
            (team, "disbanded", "character_deleted"),
        ]
    );

    let acc = fx.account_id.to_string();
    let changed = export_log(&capture, Level::INFO, cmd, "leader_changed")
        .expect("leader_changed exported at INFO");
    for (k, v) in [
        ("reason", "character_deleted"),
        ("source", "character_delete"),
        ("from_player_id", p0.to_string().as_str()),
        ("from_account_id", acc.as_str()),
        ("to_player_id", p1.to_string().as_str()),
        ("to_account_id", acc.as_str()),
    ] {
        assert!(changed.has_field(k, v), "{k} = {v} in {changed:?}");
    }
    let disbanded =
        export_log(&capture, Level::INFO, team, "disbanded").expect("disbanded exported at INFO");
    assert!(disbanded.has_field("from_account_id", &acc));
    assert!(
        !disbanded.fields.contains_key("to_player_id"),
        "a disband names no second player: {disbanded:?}"
    );
    drop(capture);

    assert_eq!(
        audit_rows(&pool, cmd).await,
        vec![(
            "leader_changed".into(),
            "character_deleted".into(),
            p0,
            Some(p1),
            true
        )]
    );
    assert!(audit_rows(&pool, team).await[0].4, "stamped");
    let swept = sweep_unexported(&pool).await.unwrap();
    assert!(
        swept.iter().all(|r| r.org_id != cmd && r.org_id != team),
        "a stamped row must not be logged twice"
    );

    teardown(&pool, &fx).await;
}

/// A bare `DELETE FROM sgw_player` (psql, a GM tool, a test) leaves the
/// trigger's row unstamped; the startup sweep logs it at INFO once and
/// stamps it.
#[tokio::test]
async fn startup_sweep_exports_rows_a_bare_delete_left() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 21, 2, &["Org02 Sweep"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Team, "Org02 Sweep", p0).await.org_id;
    add(&pool, org, p1, OrgRank::MEMBER).await;

    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(p0)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        audit_rows(&pool, org).await,
        vec![(
            "leader_changed".into(),
            "character_deleted".into(),
            p0,
            Some(p1),
            false
        )],
        "the trigger's row waits, unstamped"
    );

    let capture = LogCapture::install();
    let swept = sweep_unexported(&pool).await.unwrap();
    assert_eq!(swept.iter().filter(|r| r.org_id == org).count(), 1);
    let log = export_log(&capture, Level::INFO, org, "leader_changed")
        .expect("the sweep logs the row at INFO");
    assert!(log.has_field("source", "startup_sweep"));
    assert!(log.has_field("from_account_id", &fx.account_id.to_string()));
    drop(capture);

    let id = swept.iter().find(|r| r.org_id == org).unwrap().org_event_id;
    assert!(
        log.has_field("org_event_id", &id.to_string()),
        "dedup key on the event"
    );

    assert!(audit_rows(&pool, org).await[0].4, "stamped");
    let again = sweep_unexported(&pool).await.unwrap();
    assert!(
        again.iter().all(|r| r.org_id != org),
        "a stamped row is not re-sent"
    );

    // At least once: a crash after the log but before the stamp commits
    // leaves the row unstamped, and the next sweep sends it again with the
    // same org_event_id for queries to deduplicate on.
    sqlx::query("UPDATE sgw_organization_events SET exported_at = NULL WHERE org_event_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let resent = sweep_unexported(&pool).await.unwrap();
    assert_eq!(
        resent
            .iter()
            .filter(|r| r.org_id == org)
            .map(|r| r.org_event_id)
            .collect::<Vec<_>>(),
        vec![id]
    );

    teardown(&pool, &fx).await;
}

/// A Rust leave or kick: `remove_member` logs nothing from the audit
/// table itself; it returns its transaction id, and the caller exports
/// after committing, at INFO with `reason = member_removed`. A rollback
/// leaves no row to export.
#[tokio::test]
async fn remove_member_rows_export_after_commit() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 22, 2, &["Org02 InTx"]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    let org = create(&pool, OrgType::Command, "Org02 InTx", p0)
        .await
        .org_id;
    add(&pool, org, p1, OrgRank::INITIATE).await;

    // Rolled back: no row survives.
    let mut tx = pool.begin().await.unwrap();
    let r = as_sys!(remove_member, tx, org, p0).unwrap();
    tx.rollback().await.unwrap();
    assert!(audit_rows(&pool, org).await.is_empty());
    assert!(
        export_committed(&pool, r.tx_id, ExportSource::MemberRemoval)
            .await
            .unwrap()
            .is_empty()
    );

    let capture = LogCapture::install();
    let mut tx = pool.begin().await.unwrap();
    let r = as_sys!(remove_member, tx, org, p0).unwrap();
    assert_eq!(r.after, AfterRemoval::LeaderPromoted { player_id: p1 });
    assert!(
        export_log(&capture, Level::INFO, org, "leader_changed").is_none()
            && export_log(&capture, Level::DEBUG, org, "leader_changed").is_none(),
        "nothing is logged from the audit table before the commit"
    );
    tx.commit().await.unwrap();
    assert!(
        !audit_rows(&pool, org).await[0].4,
        "unstamped until exported"
    );

    let rows = export_committed(&pool, r.tx_id, ExportSource::MemberRemoval)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let log = export_log(&capture, Level::INFO, org, "leader_changed")
        .expect("exported at INFO after the commit");
    assert!(log.has_field("reason", "member_removed"));
    assert!(log.has_field("source", "member_removal"));
    assert!(log.has_field("org_event_id", &rows[0].org_event_id.to_string()));
    drop(capture);

    assert_eq!(
        audit_rows(&pool, org).await,
        vec![(
            "leader_changed".into(),
            "member_removed".into(),
            p0,
            Some(p1),
            true
        )]
    );

    teardown(&pool, &fx).await;
}
