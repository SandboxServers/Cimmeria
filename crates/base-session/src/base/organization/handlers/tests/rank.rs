//! `organizationRankChange` for Teams and Commands (ORG-07): CAT-M-06
//! (D-ORG09 (1), (2), (4), (5)) and the [40] fanout.

use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_rank_changed_organization, ON_MEMBER_RANK_CHANGED_ORGANIZATION,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::answer::{
    LEADER_NOT_ASSIGNABLE_TEXT, NO_PERMISSION_TEXT, RANK_NOT_IN_TYPE_TEXT, RANK_TOO_LOW_TEXT,
};
use crate::base::organization::handlers::{handle_rank_change, OrgReject};
use crate::test_support::{require_db_or_skip, LogCapture};

/// CAT-M-06, D-ORG09 (2): a Senior Officer (7, holding `Promote`) can
/// promote nobody to Senior Officer, and cannot touch a peer.
#[tokio::test]
async fn rank_change_rejects_promote_above_self() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 21, 3, &["Org07 Promote"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Promote", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::SENIOR_OFFICER).await;
    fx.online(1);
    let capture = LogCapture::install();
    assert_eq!(
        handle_rank_change(&fx.ctx(), &fx.player(1), cmd, &fx.name(2), 7).await,
        Err(OrgReject::RankTooLow)
    );
    let row = one_row(
        &capture,
        "org.rank_change",
        "rejected",
        Some("rank_too_low"),
    );
    assert!(row.has_field("actor_rank", "7") && row.has_field("to_rank", "7"));
    assert!(row.has_field("target_rank", "1"), "{row:?}");
    fx.set_rank_of(cmd, 2, OrgRank::SENIOR_OFFICER).await;
    let capture = LogCapture::install();
    assert_eq!(
        handle_rank_change(&fx.ctx(), &fx.player(1), cmd, &fx.name(2), 2).await,
        Err(OrgReject::RankTooLow),
        "a peer is not below the actor"
    );
    one_row(
        &capture,
        "org.rank_change",
        "rejected",
        Some("rank_too_low"),
    );
    assert_eq!(fx.rank_of(cmd, 2).await, Some(7));
    assert_eq!(
        feedback_lines(&fx.calls_to(1)),
        vec![RANK_TOO_LOW_TEXT, RANK_TOO_LOW_TEXT]
    );
    fx.teardown().await;
}

/// CAT-M-06, D-ORG09 (4): `Leader` is never assigned by a rank change, not
/// even by the Leader.
#[tokio::test]
async fn rank_change_rejects_assign_leader() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 22, 2, &["Org07 Crown"]).await;
    let team = fx.org(OrgType::Team, "Org07 Crown", 0, &[1]).await;
    fx.online(0);
    let capture = LogCapture::install();
    assert_eq!(
        handle_rank_change(&fx.ctx(), &fx.player(0), team, &fx.name(1), 8).await,
        Err(OrgReject::LeaderNotAssignable)
    );
    one_row(
        &capture,
        "org.rank_change",
        "rejected",
        Some("leader_not_assignable"),
    );
    assert_eq!(fx.rank_of(team, 1).await, Some(2));
    assert_eq!(fx.rank_of(team, 0).await, Some(8));
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec![LEADER_NOT_ASSIGNABLE_TEXT]
    );
    fx.teardown().await;
}

/// CAT-M-06, D-ORG09 (5): a rank the type does not use is refused: 0 in
/// any type, 5 in a Team, and anything above 8.
#[tokio::test]
async fn rank_change_rejects_rank_not_in_type() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 23, 2, &["Org07 Ladder"]).await;
    let team = fx.org(OrgType::Team, "Org07 Ladder", 0, &[1]).await;
    fx.online(0);
    for rank in [0u8, 5, 9, 255] {
        let capture = LogCapture::install();
        assert_eq!(
            handle_rank_change(&fx.ctx(), &fx.player(0), team, &fx.name(1), rank).await,
            Err(OrgReject::RankNotInType),
            "rank {rank}"
        );
        one_row(
            &capture,
            "org.rank_change",
            "rejected",
            Some("rank_not_in_type"),
        );
    }
    assert_eq!(fx.rank_of(team, 1).await, Some(2));
    assert!(feedback_lines(&fx.calls_to(0))
        .iter()
        .all(|l| l == RANK_NOT_IN_TYPE_TEXT));
    fx.teardown().await;
}

/// D-ORG09 (1): the direction picks the bit. An Officer (6: no `Promote`,
/// no `Demote`) can do neither; a changed rank to the same value is
/// refused before a bit is chosen.
#[tokio::test]
async fn rank_change_needs_the_bit_for_its_direction() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 24, 3, &["Org07 Bits"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Bits", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::MEMBER).await;
    fx.online(1);
    for (to, why) in [
        (3u8, OrgReject::MissingPermission),
        (1, OrgReject::MissingPermission),
        (2, OrgReject::RankUnchanged),
    ] {
        let capture = LogCapture::install();
        assert_eq!(
            handle_rank_change(&fx.ctx(), &fx.player(1), cmd, &fx.name(2), to).await,
            Err(why),
            "to {to}"
        );
        one_row(&capture, "org.rank_change", "rejected", Some(why.reason()));
    }
    assert_eq!(fx.rank_of(cmd, 2).await, Some(2));
    assert_eq!(feedback_lines(&fx.calls_to(1))[0], NO_PERMISSION_TEXT);
    fx.teardown().await;
}

/// A rank change writes the rank under the lock, then every online member,
/// the target included, gets [40] with the target's entity id; the target
/// and the actor each get a line; one `ok` row carries all three ranks.
#[tokio::test]
async fn rank_change_updates_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 25, 3, &["Org07 Promoted"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Promoted", 0, &[1, 2]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    let from = handle_rank_change(&fx.ctx(), &fx.player(0), cmd, &fx.name(1), 6)
        .await
        .expect("rank change");
    assert_eq!(from, OrgRank::INITIATE);
    assert_eq!(fx.rank_of(cmd, 1).await, Some(6));
    let row = one_row(&capture, "org.rank_change", "ok", None);
    assert!(
        row.has_field("actor_rank", "8")
            && row.has_field("target_rank", "1")
            && row.has_field("to_rank", "6"),
        "{row:?}"
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "rank_changed")
            && c.has_field("from_rank", "1")
            && c.has_field("to_rank", "6")));
    let changed = (
        ON_MEMBER_RANK_CHANGED_ORGANIZATION,
        build_on_member_rank_changed_organization(
            fx.entity(1) as i32,
            OrgRank::OFFICER,
            cmd,
            &fx.name(1),
        ),
    );
    for i in 0..3 {
        assert_eq!(fx.calls_to(i)[0], changed, "member {i}");
    }
    assert_eq!(
        feedback_lines(&fx.calls_to(1)),
        vec!["Your rank in Org07 Promoted is now 6.".to_string()]
    );
    fx.teardown().await;
}
