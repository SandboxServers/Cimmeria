//! `organizationKick` for Teams and Commands (ORG-07): CAT-M-05 (D-ORG09
//! (1)-(2)), the kick fanout, and the Bank's `OrgMembershipEnded` hook.

use cimmeria_entity::organization::OrgLeaveReason;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_left_organization, build_on_organization_left, ON_MEMBER_LEFT_ORGANIZATION,
    ON_ORGANIZATION_LEFT,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::answer::{
    NO_PERMISSION_TEXT, RANK_TOO_LOW_TEXT, TARGET_NOT_MEMBER_TEXT,
};
use crate::base::organization::handlers::{handle_kick, OrgReject};
use crate::test_support::{require_db_or_skip, LogCapture};

/// CAT-M-05: the actor's rank must be strictly above the target's. An
/// Officer (6) cannot kick another Officer, nor the Leader; both stay.
#[tokio::test]
async fn org_kick_rejects_equal_or_higher_rank() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 17, 3, &["Org07 Peers"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 Peers", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::OFFICER).await;
    fx.online(1);
    for target in [2, 0] {
        let capture = LogCapture::install();
        let r = handle_kick(&fx.ctx(), &fx.player(1), cmd, &fx.name(target)).await;
        assert_eq!(r, Err(OrgReject::RankTooLow), "target {target}");
        let row = one_row(&capture, "org.kick", "rejected", Some("rank_too_low"));
        assert!(row.has_field("actor_rank", "6"), "{row:?}");
        assert!(
            row.has_field("target_player_id", &fx.player_id(target).to_string()),
            "{row:?}"
        );
    }
    assert_eq!(
        fx.member_ids(cmd).await,
        vec![fx.player_id(0), fx.player_id(1), fx.player_id(2)]
    );
    assert_eq!(
        feedback_lines(&fx.calls_to(1)),
        vec![RANK_TOO_LOW_TEXT, RANK_TOO_LOW_TEXT]
    );
    assert!(fx.memberships_ended().is_empty());
    fx.teardown().await;
}

/// D-ORG09 (1): a rank without `Eject` (a Command Veteran) is refused even
/// against a lower rank; a name that is not a member, and the actor's own
/// name, are refused too.
#[tokio::test]
async fn org_kick_needs_eject_a_member_target_and_not_self() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 18, 4, &["Org07 NoEject"]).await;
    let cmd = fx.org(OrgType::Command, "Org07 NoEject", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::VETERAN).await;
    fx.online(1);
    fx.online(0);
    let capture = LogCapture::install();
    assert_eq!(
        handle_kick(&fx.ctx(), &fx.player(1), cmd, &fx.name(2)).await,
        Err(OrgReject::MissingPermission)
    );
    one_row(&capture, "org.kick", "rejected", Some("missing_permission"));
    let capture = LogCapture::install();
    assert_eq!(
        handle_kick(&fx.ctx(), &fx.player(0), cmd, &fx.name(3)).await,
        Err(OrgReject::TargetNotMember)
    );
    one_row(&capture, "org.kick", "rejected", Some("target_not_member"));
    let capture = LogCapture::install();
    assert_eq!(
        handle_kick(&fx.ctx(), &fx.player(0), cmd, &fx.name(0)).await,
        Err(OrgReject::SelfTarget)
    );
    one_row(&capture, "org.kick", "rejected", Some("self_target"));
    assert_eq!(fx.member_ids(cmd).await.len(), 3);
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NO_PERMISSION_TEXT]);
    assert_eq!(
        feedback_lines(&fx.calls_to(0))[0],
        TARGET_NOT_MEMBER_TEXT.to_string()
    );
    fx.teardown().await;
}

/// A kick removes the member under the lock; then the kicked player gets
/// [36] `Kicked` and a line, the cell gets `OrgMembershipEnded` (`Kicked`,
/// the Bank's vault-session hook), and every remaining online member, the
/// actor included, gets [39] `Kicked` with the kicked player's live entity
/// id. The target is matched case-insensitively among members.
#[tokio::test]
async fn org_kick_removes_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 19, 3, &["Org07 Kick"]).await;
    let team = fx.org(OrgType::Team, "Org07 Kick", 0, &[1, 2]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    let kicked = handle_kick(&fx.ctx(), &fx.player(0), team, &fx.name(1).to_uppercase())
        .await
        .expect("kick");
    assert_eq!(kicked, fx.player_id(1));
    assert_eq!(
        fx.member_ids(team).await,
        vec![fx.player_id(0), fx.player_id(2)]
    );
    let row = one_row(&capture, "org.kick", "ok", None);
    assert!(row.has_field("target_rank", "2") && row.has_field("actor_rank", "8"));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "member_left") && c.has_field("reason", "kicked")));

    let to_kicked = fx.calls_to(1);
    assert_eq!(
        to_kicked[0],
        (
            ON_ORGANIZATION_LEFT,
            build_on_organization_left(OrgLeaveReason::Kicked, team)
        )
    );
    assert_eq!(
        feedback_lines(&to_kicked),
        vec!["You were removed from Org07 Kick.".to_string()]
    );
    assert_eq!(
        fx.memberships_ended(),
        vec![(fx.player_id(1), fx.entity(1), team, OrgLeaveReason::Kicked)]
    );
    let left = (
        ON_MEMBER_LEFT_ORGANIZATION,
        build_on_member_left_organization(
            fx.entity(1) as i32,
            OrgLeaveReason::Kicked,
            team,
            &fx.name(1),
        ),
    );
    assert_eq!(fx.calls_to(2), vec![left.clone()]);
    let to_actor = fx.calls_to(0);
    assert_eq!(to_actor[0], left);
    assert_eq!(
        feedback_lines(&to_actor),
        vec![format!("You removed {} from Org07 Kick.", fx.name(1))]
    );
    fx.teardown().await;
}

/// An offline member can be kicked: the remaining members' [39] carries
/// entity id 0, and nobody is told a membership ended on the cell.
#[tokio::test]
async fn org_kick_of_an_offline_member_uses_id_zero() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 20, 2, &["Org07 Offline Kick"]).await;
    let team = fx.org(OrgType::Team, "Org07 Offline Kick", 0, &[1]).await;
    fx.online(0);
    handle_kick(&fx.ctx(), &fx.player(0), team, &fx.name(1))
        .await
        .expect("kick");
    assert_eq!(fx.calls_to(0)[0].1, {
        build_on_member_left_organization(0, OrgLeaveReason::Kicked, team, &fx.name(1))
    });
    assert!(fx.memberships_ended().is_empty());
    fx.teardown().await;
}
