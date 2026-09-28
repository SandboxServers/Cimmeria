//! `organizationInvite` / `organizationInviteByType` for Teams and Commands
//! (ORG-07): CAT-M-01 and CAT-M-02, D-ORG18, the Ignore rule, the rate
//! limit, and the `onOrganizationInvite` [34] the invitee gets.

use std::time::Instant;

use cimmeria_entity::organization::BASE_INVITE_REQUEST_FLAG;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_invite, ON_ORGANIZATION_INVITE,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::answer::{
    IGNORED_TEXT, NOT_MEMBER_TEXT, NO_PERMISSION_TEXT, RATE_LIMITED_TEXT,
};
use crate::base::organization::handlers::{handle_invite, InviteInto, OrgReject};
use crate::test_support::{require_db_or_skip, LogCapture};

/// A successful invite: the invitee holds one base request id (bit 29) and
/// gets [34] byte for byte; the inviter gets a line; one `ok` row with
/// both identities and the inviter's rank; the DEBUG `invite_created`.
#[tokio::test]
async fn live_db_invite_records_pending_and_sends_on_organization_invite() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 0, 2, &["Org07 Invite"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Invite", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let capture = LogCapture::install();
    let request_id = handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::Org(cmd),
        &fx.name(1),
        Instant::now(),
    )
    .await
    .expect("invite");
    assert_ne!(request_id & BASE_INVITE_REQUEST_FLAG, 0);
    assert_eq!(fx.held_invites(1), 1);
    assert_eq!(
        fx.calls_to(1),
        vec![(
            ON_ORGANIZATION_INVITE,
            build_on_organization_invite(
                &fx.name(0),
                OrgType::Command,
                request_id,
                "Org07 Invite",
                false
            )
        )]
    );
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec![format!("You invited {} to join Org07 Invite.", fx.name(1))]
    );
    let row = one_row(&capture, "org.invite", "ok", None);
    assert!(row.has_field("target_player_id", &fx.player_id(1).to_string()));
    assert!(row.has_field("target_account_id", &fx.account_id.to_string()));
    assert!(row.has_field("actor_rank", "8"), "{row:?}");
    assert!(row.has_field("request_id", &request_id.to_string()));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "invite_created") && c.level == tracing::Level::DEBUG));
    fx.teardown().await;
}

/// CAT-M-01: an inviter who is not a member of the organization named
/// (here a real Team of someone else's) is refused under the lock; nothing
/// is recorded or sent to the invitee.
#[tokio::test]
async fn live_db_org_invite_rejects_non_member_inviter() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 1, 3, &["Org07 Foreign"]).await;
    let team = fx.org(OrgType::Team, "Org07 Foreign", 0, &[]).await;
    fx.online(1);
    fx.online(2);
    let capture = LogCapture::install();
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(1),
        InviteInto::Org(team),
        &fx.name(2),
        Instant::now(),
    )
    .await;
    assert_eq!(r, Err(OrgReject::NotMember));
    one_row(&capture, "org.invite", "rejected", Some("not_member"));
    assert_eq!(fx.held_invites(2), 0);
    assert!(fx.calls_to(2).is_empty(), "the invitee hears nothing");
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NOT_MEMBER_TEXT]);
    fx.teardown().await;
}

/// CAT-M-01: a member whose rank lacks `Invite` (a Team Member holds only
/// roster notes and the vault bits, D-ORG08) is refused.
#[tokio::test]
async fn live_db_org_invite_rejects_without_invite_perm() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 2, 3, &["Org07 NoPerm"]).await;
    let team = fx.org(OrgType::Team, "Org07 NoPerm", 0, &[1]).await;
    fx.online(1);
    fx.online(2);
    let capture = LogCapture::install();
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(1),
        InviteInto::Org(team),
        &fx.name(2),
        Instant::now(),
    )
    .await;
    assert_eq!(r, Err(OrgReject::MissingPermission));
    let row = one_row(
        &capture,
        "org.invite",
        "rejected",
        Some("missing_permission"),
    );
    assert!(row.has_field("actor_rank", "2"), "{row:?}");
    assert_eq!(fx.held_invites(2), 0);
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NO_PERMISSION_TEXT]);
    fx.teardown().await;
}

/// CAT-M-02: invite-by-type for a Team or Command only ever finds the
/// inviter's existing organization of that type. An inviter with none is
/// refused, and no organization is created.
#[tokio::test]
async fn live_db_invite_by_type_never_creates_team_or_command() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 3, 2, &[]).await;
    fx.online(0);
    fx.online(1);
    for org_type in [OrgType::Team, OrgType::Command] {
        let capture = LogCapture::install();
        let r = handle_invite(
            &fx.ctx(),
            &fx.player(0),
            InviteInto::ByType(org_type),
            &fx.name(1),
            Instant::now(),
        )
        .await;
        assert_eq!(r, Err(OrgReject::NotMember), "{org_type:?}");
        one_row(&capture, "org.invite", "rejected", Some("not_member"));
    }
    let memberships: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sgw_organization_members WHERE player_id = ANY($1)",
    )
    .bind(&fx.players)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(memberships, 0, "no Team or Command was created");
    assert_eq!(fx.held_invites(1), 0);
    fx.teardown().await;
}

/// Invite-by-type resolves the inviter's own organization of that type.
#[tokio::test]
async fn live_db_invite_by_type_finds_the_inviters_organization() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 4, 2, &["Org07 ByType"]).await;
    let team = fx.org(OrgType::Team, "Org07 ByType", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let capture = LogCapture::install();
    handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::ByType(OrgType::Team),
        &fx.name(1),
        Instant::now(),
    )
    .await
    .expect("invite");
    let row = one_row(&capture, "org.invite", "ok", None);
    assert!(row.has_field("org_id", &team.to_string()), "{row:?}");
    assert!(row.has_field("org_type", "team"), "{row:?}");
    fx.teardown().await;
}

/// D-ORG18: an invitee already in a Team is refused a second Team's
/// invite, and one already in this Team is refused as a member.
#[tokio::test]
async fn live_db_invite_rejects_a_target_already_in_the_type() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 5, 3, &["Org07 TeamA", "Org07 TeamB"]).await;
    let a = fx.org(OrgType::Team, "Org07 TeamA", 0, &[2]).await;
    fx.org(OrgType::Team, "Org07 TeamB", 1, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    // Character 1 leads Team B and invites Team A's member.
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(1),
        InviteInto::ByType(OrgType::Team),
        &fx.name(2),
        Instant::now(),
    )
    .await;
    assert_eq!(r, Err(OrgReject::AlreadyInOrgType));
    one_row(
        &capture,
        "org.invite",
        "rejected",
        Some("already_in_org_type"),
    );
    let capture = LogCapture::install();
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::Org(a),
        &fx.name(2),
        Instant::now(),
    )
    .await;
    assert_eq!(r, Err(OrgReject::AlreadyMember));
    one_row(&capture, "org.invite", "rejected", Some("already_member"));
    assert_eq!(fx.held_invites(2), 0);
    fx.teardown().await;
}

/// The Ignore rule (carried from ORG-03): an invitee who ignores the
/// inviter is never offered the invite; the inviter reads the same line a
/// duel challenge gets.
#[tokio::test]
async fn live_db_invite_rejects_an_invitee_who_ignores_the_inviter() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 6, 2, &["Org07 Ignored"]).await;
    let team = fx.org(OrgType::Team, "Org07 Ignored", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    fx.ignore(1, 0);
    let capture = LogCapture::install();
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::Org(team),
        &fx.name(1),
        Instant::now(),
    )
    .await;
    assert_eq!(r, Err(OrgReject::Ignored));
    let row = one_row(&capture, "org.invite", "rejected", Some("ignored"));
    assert!(row.has_field("target_player_id", &fx.player_id(1).to_string()));
    assert_eq!(fx.held_invites(1), 0);
    assert!(fx.calls_to(1).is_empty());
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![IGNORED_TEXT]);
    fx.teardown().await;
}

/// The inviter's rate limit is checked before any database work: five sent
/// in the window, the sixth is refused.
#[tokio::test]
async fn live_db_invite_is_rate_limited_per_inviter() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 7, 2, &["Org07 Rate"]).await;
    let team = fx.org(OrgType::Team, "Org07 Rate", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let now = Instant::now();
    {
        let mut clients = fx.connected.lock().unwrap();
        let s = clients.get_mut(&fx.addr(0)).unwrap();
        for _ in 0..crate::base::organization::invites::INVITE_RATE_MAX {
            s.org_invites.record_sent(now);
        }
    }
    let capture = LogCapture::install();
    let r = handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::Org(team),
        &fx.name(1),
        now,
    )
    .await;
    assert_eq!(r, Err(OrgReject::RateLimited));
    one_row(&capture, "org.invite", "rejected", Some("rate_limited"));
    assert_eq!(fx.held_invites(1), 0);
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![RATE_LIMITED_TEXT]);
    fx.teardown().await;
}

/// Negative seam: the invitee's session is in the index but its entity is
/// not in `entity_to_addr`, so [34] cannot be sent. WARN `org.send_failed`
/// (`what = organization_invite`) with the reason; the invite is still
/// recorded and the action still ends in its one row.
#[tokio::test]
async fn live_db_an_unsendable_invite_warns_with_reason() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 8, 2, &["Org07 Unsent"]).await;
    let team = fx.org(OrgType::Team, "Org07 Unsent", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    fx.entity_to_addr.lock().unwrap().remove(&fx.entity(1));
    let capture = LogCapture::install();
    handle_invite(
        &fx.ctx(),
        &fx.player(0),
        InviteInto::Org(team),
        &fx.name(1),
        Instant::now(),
    )
    .await
    .expect("recorded");
    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "could not be sent to the invitee",
            "entity_to_addr_miss",
        )
        .expect("org.send_failed WARN");
    assert!(warn.has_field("what", "organization_invite"), "{warn:?}");
    assert!(warn.has_field("target_player_id", &fx.player_id(1).to_string()));
    one_row(&capture, "org.invite", "ok", None);
    fx.teardown().await;
}
