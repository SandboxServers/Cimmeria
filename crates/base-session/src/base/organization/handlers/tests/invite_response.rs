//! `organizationInviteResponse` (CM 8) for a base request id (ORG-07):
//! CAT-M-18 (composite key, single use, re-validated under ORG-LOCK), the
//! join and its fanout, and the decline.

use std::time::Instant;

use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, build_on_organization_joined,
    ON_MEMBER_JOINED_ORGANIZATION, ON_ORGANIZATION_JOINED,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::api::VAULT_EMPTY_OVERRIDE;
use crate::base::organization::handlers::answer::{INVITE_INVALID_TEXT, ORG_GONE_TEXT};
use crate::base::organization::handlers::{
    handle_invite, handle_invite_response, InviteAnswer, InviteInto, OrgReject,
};
use crate::base::organization::persistence::{disband, remove_member};
use crate::test_support::{require_db_or_skip, LogCapture};

/// Character `from` invites character `to` into `org_id`; the request id.
async fn invite(fx: &Fixture, org_id: i32, from: usize, to: usize) -> i32 {
    handle_invite(
        &fx.ctx(),
        &fx.player(from),
        InviteInto::Org(org_id),
        &fx.name(to),
        Instant::now(),
    )
    .await
    .expect("invite")
}

async fn answer(
    fx: &Fixture,
    who: usize,
    request_id: i32,
    accept: bool,
) -> Result<InviteAnswer, OrgReject> {
    handle_invite_response(
        &fx.ctx(),
        &fx.player(who),
        request_id,
        accept,
        Instant::now(),
    )
    .await
}

/// An accept joins at the type's entry rank (D-ORG07: Command 1), sends
/// the joiner the full state ([35] with `aNewMember = 1` first) and a line,
/// tells the other online member with [37] (`aNewMember = 1`, the joiner's
/// entity id), and ends in one `ok` row naming the inviter as the target.
#[tokio::test]
async fn org_accept_joins_at_entry_rank_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 9, 2, &["Org07 Accept"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Accept", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let request_id = invite(&fx, cmd, 0, 1).await;
    fx.clear_sent();
    let capture = LogCapture::install();
    let r = answer(&fx, 1, request_id, true).await;
    assert_eq!(
        r,
        Ok(InviteAnswer::Joined {
            org_id: cmd,
            rank: OrgRank::INITIATE
        })
    );
    assert_eq!(fx.rank_of(cmd, 1).await, Some(1));
    let joiner = fx.calls_to(1);
    assert_eq!(
        joiner[0],
        (
            ON_ORGANIZATION_JOINED,
            build_on_organization_joined(cmd, OrgType::Command, OrgRank::INITIATE, true)
        ),
        "the state push opens with [35] as a new member"
    );
    assert_eq!(
        feedback_lines(&joiner),
        vec!["You joined Org07 Accept.".to_string()]
    );
    assert_eq!(
        fx.calls_to(0),
        vec![(
            ON_MEMBER_JOINED_ORGANIZATION,
            build_on_member_joined_organization(
                &fx.name(1),
                fx.entity(1) as i32,
                cmd,
                OrgRank::INITIATE,
                true
            )
        )]
    );
    let row = one_row(&capture, "org.invite_response", "ok", None);
    assert!(row.has_field("after", "joined"));
    assert!(row.has_field("target_player_id", &fx.player_id(0).to_string()));
    assert!(row.has_field("to_rank", "1"));
    for event in ["invite_consumed", "member_joined"] {
        assert!(
            capture.all().iter().any(|c| c.has_field("event", event)),
            "{event} transition"
        );
    }
    fx.teardown().await;
}

/// CAT-M-18: answering another player's request id finds nothing (the
/// entry is keyed by the invitee's character too), is reported as foreign
/// in the log but reads like any stale invite, and leaves the real
/// invitee's entry usable.
#[tokio::test]
async fn invite_response_rejects_foreign_request_id() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 10, 3, &["Org07 Foreign Id"]).await;
    let team = fx.org(OrgType::Team, "Org07 Foreign Id", 0, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let request_id = invite(&fx, team, 0, 1).await;
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 2, request_id, true).await,
        Err(OrgReject::InviteForeign)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("invite_foreign"),
    );
    assert_eq!(feedback_lines(&fx.calls_to(2)), vec![INVITE_INVALID_TEXT]);
    assert_eq!(fx.rank_of(team, 2).await, None);
    assert!(answer(&fx, 1, request_id, true).await.is_ok());
    assert_eq!(fx.rank_of(team, 1).await, Some(2));
    fx.teardown().await;
}

/// CAT-M-18: an invite is consumed by its first response, so a replayed
/// accept (or an accept after a decline) finds nothing.
#[tokio::test]
async fn invite_response_rejects_replay() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 11, 3, &["Org07 Replay"]).await;
    let team = fx.org(OrgType::Team, "Org07 Replay", 0, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let first = invite(&fx, team, 0, 1).await;
    assert!(answer(&fx, 1, first, true).await.is_ok());
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 1, first, true).await,
        Err(OrgReject::InviteUnknown)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("invite_unknown"),
    );

    let declined = invite(&fx, team, 0, 2).await;
    assert_eq!(
        answer(&fx, 2, declined, false).await,
        Ok(InviteAnswer::Declined)
    );
    assert_eq!(
        answer(&fx, 2, declined, true).await,
        Err(OrgReject::InviteUnknown)
    );
    assert_eq!(
        fx.rank_of(team, 2).await,
        None,
        "a replayed accept joins nobody"
    );
    fx.teardown().await;
}

/// CAT-M-18: the accept re-validates under ORG-LOCK. An inviter kicked
/// after sending the invite no longer vouches for it.
#[tokio::test]
async fn org_accept_rejects_after_inviter_kicked() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 12, 3, &["Org07 Kicked Inviter"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org07 Kicked Inviter", 0, &[1])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    for i in 0..3 {
        fx.online(i);
    }
    let request_id = invite(&fx, cmd, 1, 2).await;
    {
        let mut tx = pool.begin().await.unwrap();
        let actor =
            crate::base::organization::api::member_access_locked(&mut tx, cmd, fx.player_id(0))
                .await
                .unwrap()
                .unwrap();
        remove_member(&mut tx, &actor, cmd, fx.player_id(1))
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 2, request_id, true).await,
        Err(OrgReject::InviterLeft)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("inviter_left"),
    );
    assert_eq!(fx.rank_of(cmd, 2).await, None);
    assert_eq!(feedback_lines(&fx.calls_to(2)), vec![INVITE_INVALID_TEXT]);
    fx.teardown().await;
}

/// An inviter demoted to a rank without `Invite` no longer vouches for it
/// either.
#[tokio::test]
async fn org_accept_rejects_after_inviter_loses_invite() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 13, 3, &["Org07 Demoted Inviter"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org07 Demoted Inviter", 0, &[1])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    for i in 0..3 {
        fx.online(i);
    }
    let request_id = invite(&fx, cmd, 1, 2).await;
    fx.set_rank_of(cmd, 1, OrgRank::VETERAN).await;
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 2, request_id, true).await,
        Err(OrgReject::InviterMissingPermission)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("inviter_missing_permission"),
    );
    assert_eq!(fx.rank_of(cmd, 2).await, None);
    fx.teardown().await;
}

/// An organization disbanded after the invite is gone; the accept joins
/// nothing.
#[tokio::test]
async fn org_accept_rejects_after_disband() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 14, 2, &["Org07 Gone"]).await;
    let team = fx.org(OrgType::Team, "Org07 Gone", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let request_id = invite(&fx, team, 0, 1).await;
    {
        let mut tx = pool.begin().await.unwrap();
        let actor = OrgAccess::system(
            &mut tx,
            team,
            SystemActor::Server {
                source: "org07_test",
            },
        )
        .await
        .unwrap()
        .unwrap();
        VAULT_EMPTY_OVERRIDE
            .scope(true, disband(&mut tx, &actor, team))
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 1, request_id, true).await,
        Err(OrgReject::OrgGone)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("org_gone"),
    );
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![ORG_GONE_TEXT]);
    fx.teardown().await;
}

/// D-ORG18 on accept: two Teams invite the same player; once they have
/// joined one, the other accept is refused and joins nothing.
#[tokio::test]
async fn org_accept_rejects_a_second_team() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 15, 3, &["Org07 First", "Org07 Second"]).await;
    let a = fx.org(OrgType::Team, "Org07 First", 0, &[]).await;
    let b = fx.org(OrgType::Team, "Org07 Second", 1, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let from_a = invite(&fx, a, 0, 2).await;
    let from_b = invite(&fx, b, 1, 2).await;
    assert!(answer(&fx, 2, from_a, true).await.is_ok());
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 2, from_b, true).await,
        Err(OrgReject::AlreadyInOrgType)
    );
    one_row(
        &capture,
        "org.invite_response",
        "rejected",
        Some("already_in_org_type"),
    );
    assert_eq!(fx.rank_of(b, 2).await, None);
    let lines = feedback_lines(&fx.calls_to(2));
    assert_eq!(lines.last().unwrap(), "You are already in a Team.");
    fx.teardown().await;
}

/// A decline tells an online inviter and ends in one `ok` row (`after =
/// declined`); nobody joins and nothing is sent to the cell.
#[tokio::test]
async fn decline_tells_the_inviter() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 16, 2, &["Org07 Decline"]).await;
    let team = fx.org(OrgType::Team, "Org07 Decline", 0, &[]).await;
    fx.online(0);
    fx.online(1);
    let request_id = invite(&fx, team, 0, 1).await;
    fx.clear_sent();
    let capture = LogCapture::install();
    assert_eq!(
        answer(&fx, 1, request_id, false).await,
        Ok(InviteAnswer::Declined)
    );
    let row = one_row(&capture, "org.invite_response", "ok", None);
    assert!(row.has_field("after", "declined"));
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec![format!("{} declined your invitation.", fx.name(1))]
    );
    assert_eq!(fx.rank_of(team, 1).await, None);
    assert!(fx.memberships_ended().is_empty());
    fx.teardown().await;
}
