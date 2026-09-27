//! `organizationLeave` for Teams and Commands: the CAT-M-04 non-member
//! refusal, D-ORG12's leader rule, the last-member disband and D-ORG20's
//! vault refusal.

use cimmeria_entity::organization::OrgLeaveReason;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_left_organization, build_on_organization_left,
};
use tracing::Level;

use super::*;
use crate::base::organization::api::VAULT_EMPTY_OVERRIDE;
use crate::base::organization::handlers::leave::{
    LEADER_CANNOT_LEAVE_TEXT, NOT_MEMBER_TEXT, VAULT_NOT_EMPTY_TEXT,
};
use crate::base::organization::handlers::{handle_leave, LeaveOutcome, OrgReject};
use crate::test_support::{require_db_or_skip, LogCapture};

/// The one `org.leave` row, asserted to be `outcome`/`reason`.
fn assert_row(capture: &crate::test_support::LogCaptureGuard, outcome: &str, reason: Option<&str>) {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", "org.leave"))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one outcome row: {rows:#?}");
    assert!(rows[0].has_field("outcome", outcome), "{:?}", rows[0]);
    if let Some(r) = reason {
        assert!(rows[0].has_field("reason", r), "{:?}", rows[0]);
    }
    assert_eq!(rows[0].level, Level::INFO);
}

/// CAT-M-04: a leave naming an organization the caller is not in (here, a
/// real Team of someone else's) is refused under the lock, changes nothing,
/// and the caller gets a feedback line.
#[tokio::test]
async fn org_leave_rejects_non_member() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 4, 3, &["Org06 Foreign"]).await;
    let team = fx.org(OrgType::Team, "Org06 Foreign", 0, &[1]).await;
    fx.online(2);
    let capture = LogCapture::install();
    let r = handle_leave(&fx.ctx(), &fx.player(2), team).await;
    assert_eq!(r, Err(OrgReject::NotMember));
    assert_row(&capture, "rejected", Some("not_member"));
    assert_eq!(
        fx.member_ids(team).await,
        vec![fx.player_id(0), fx.player_id(1)]
    );
    assert_eq!(
        feedback_lines(&fx.calls_to(2)),
        vec![NOT_MEMBER_TEXT.to_string()]
    );
    fx.teardown().await;
}

/// D-ORG12: a Team leader cannot leave while others remain. The leave is
/// refused, the membership is unchanged, and the leader's client gets the
/// true state again (a [35] push) and the reason.
#[tokio::test]
async fn leader_leave_is_rejected_while_members_remain() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 5, 2, &["Org06 Leader"]).await;
    let team = fx.org(OrgType::Team, "Org06 Leader", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    let capture = LogCapture::install();
    let r = handle_leave(&fx.ctx(), &fx.player(0), team).await;
    assert_eq!(r, Err(OrgReject::LeaderCannotLeave));
    assert_row(&capture, "rejected", Some("leader_cannot_leave"));
    assert_eq!(
        fx.member_ids(team).await,
        vec![fx.player_id(0), fx.player_id(1)]
    );
    let calls = fx.calls_to(0);
    assert_eq!(calls.first().map(|c| c.0), Some(35), "state re-sent first");
    assert_eq!(
        feedback_lines(&calls),
        vec![LEADER_CANNOT_LEAVE_TEXT.to_string()]
    );
    assert!(fx.calls_to(1).is_empty(), "the others hear nothing");
    fx.teardown().await;
}

/// An ordinary member leaves: the row goes, the leaver gets
/// `onOrganizationLeft(Requested)` and the online leader
/// `onMemberLeftOrganization` with the leaver's entity id and name.
#[tokio::test]
async fn member_leave_removes_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 6, 2, &["Org06 Leave"]).await;
    let team = fx.org(OrgType::Team, "Org06 Leave", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    let capture = LogCapture::install();
    let r = handle_leave(&fx.ctx(), &fx.player(1), team).await;
    assert_eq!(r, Ok(LeaveOutcome::Left));
    assert_row(&capture, "ok", None);
    assert_eq!(fx.member_ids(team).await, vec![fx.player_id(0)]);
    assert_eq!(
        fx.calls_to(1),
        vec![(
            36,
            build_on_organization_left(OrgLeaveReason::Requested, team)
        )]
    );
    assert_eq!(
        fx.calls_to(0),
        vec![(
            39,
            build_on_member_left_organization(
                fx.entity(1) as i32,
                OrgLeaveReason::Requested,
                team,
                &fx.name(1)
            )
        )]
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.level == Level::DEBUG && c.has_field("event", "member_left")));
    fx.teardown().await;
}

/// The last member leaving disbands the organization: its rows are gone
/// and the leaver is told.
#[tokio::test]
async fn last_member_leave_disbands() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 7, 1, &["Org06 Solo"]).await;
    let team = fx.org(OrgType::Team, "Org06 Solo", 0, &[]).await;
    fx.online(0);
    let capture = LogCapture::install();
    let r = handle_leave(&fx.ctx(), &fx.player(0), team).await;
    assert_eq!(r, Ok(LeaveOutcome::Disbanded));
    assert_row(&capture, "ok", None);
    assert!(!fx.org_exists(team).await);
    let calls = fx.calls_to(0);
    assert_eq!(
        calls[0],
        (
            36,
            build_on_organization_left(OrgLeaveReason::Requested, team)
        )
    );
    assert_eq!(feedback_lines(&calls).len(), 1);
    fx.teardown().await;
}

/// D-ORG20: the last member cannot disband an organization whose vault
/// holds anything. The stub's test seam reports a non-empty vault; the
/// leave is refused, the organization and its member stay, and the member
/// gets the state and the reason.
#[tokio::test]
async fn last_member_leave_refused_while_vault_not_empty() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 8, 1, &["Org06 Vault"]).await;
    let team = fx.org(OrgType::Team, "Org06 Vault", 0, &[]).await;
    fx.online(0);
    let capture = LogCapture::install();
    let r = VAULT_EMPTY_OVERRIDE
        .scope(false, handle_leave(&fx.ctx(), &fx.player(0), team))
        .await;
    assert_eq!(r, Err(OrgReject::VaultNotEmpty));
    assert_row(&capture, "rejected", Some("vault_not_empty"));
    assert!(fx.org_exists(team).await);
    assert_eq!(fx.member_ids(team).await, vec![fx.player_id(0)]);
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec![VAULT_NOT_EMPTY_TEXT.to_string()]
    );
    fx.teardown().await;
}
