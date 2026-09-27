//! `.org_join` and `.org_rank` on the base (ORG-07, D-ORG13): the access
//! level is re-read from the GM's own session; the member permission and
//! rank checks are skipped, the type, one-per-type and rank rules are not.

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::{gm_join, gm_rank, GmCaller, OrgReject};
use crate::test_support::{require_db_or_skip, LogCapture};

impl Fixture {
    fn gm(&self, i: usize, access_level: u32) -> GmCaller {
        self.connected
            .lock()
            .unwrap()
            .get_mut(&self.addr(i))
            .unwrap()
            .access_level = access_level;
        GmCaller {
            entity_id: self.entity(i),
            player_id: self.player_id(i),
        }
    }
}

/// `.org_join`: a GM who is not a member adds an online player at the
/// type's entry rank; the joiner gets the state push, the members the
/// [37]; a second Team for the same player is refused (D-ORG18); a
/// non-GM session is refused before anything is read.
#[tokio::test]
async fn gm_join_adds_at_entry_rank_and_keeps_the_type_rule() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 28, 4, &["Org07 GmJoin", "Org07 GmJoin Two"]).await;
    let team = fx.org(OrgType::Team, "Org07 GmJoin", 0, &[]).await;
    let other = fx.org(OrgType::Team, "Org07 GmJoin Two", 3, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let gm = fx.gm(1, 2);
    let capture = LogCapture::install();
    let rank = gm_join(&fx.ctx(), gm, team, Some(&fx.name(2)))
        .await
        .expect("join");
    assert_eq!(rank, OrgRank::MEMBER);
    assert_eq!(fx.rank_of(team, 2).await, Some(2));
    let row = one_row(&capture, "org.gm_join", "ok", None);
    assert!(row.has_field("target_player_id", &fx.player_id(2).to_string()));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "org.gm_action") && c.has_field("command", "org_join")));
    assert_eq!(fx.calls_to(0).len(), 1, "the leader's [37]");

    let capture = LogCapture::install();
    assert_eq!(
        gm_join(&fx.ctx(), gm, other, Some(&fx.name(2))).await,
        Err(OrgReject::AlreadyInOrgType)
    );
    one_row(
        &capture,
        "org.gm_join",
        "rejected",
        Some("already_in_org_type"),
    );
    assert_eq!(fx.rank_of(other, 2).await, None);

    let player = fx.gm(0, 0);
    let capture = LogCapture::install();
    assert_eq!(
        gm_join(&fx.ctx(), player, team, None).await,
        Err(OrgReject::NotGm)
    );
    one_row(&capture, "org.gm_join", "rejected", Some("not_gm"));
    fx.teardown().await;
}

/// `.org_rank`: a GM sets a member's rank without being a member (D-ORG09
/// (1)-(2) skipped), resolving the organization from the member's only
/// one; `Leader` and a rank the type does not use are still refused.
#[tokio::test]
async fn gm_rank_skips_authority_but_not_the_rank_rules() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org07(&pool, 29, 3, &["Org07 GmRank"]).await;
    let team = fx.org(OrgType::Team, "Org07 GmRank", 0, &[1]).await;
    fx.online(1);
    fx.online(2);
    let gm = fx.gm(2, 2);
    let capture = LogCapture::install();
    let from = gm_rank(&fx.ctx(), gm, &fx.name(1), 3, None)
        .await
        .expect("rank");
    assert_eq!(from, OrgRank::MEMBER);
    assert_eq!(fx.rank_of(team, 1).await, Some(3));
    one_row(&capture, "org.gm_rank", "ok", None);
    assert_eq!(fx.calls_to(1)[0].0, 40, "the member's [40]");
    for (rank, why) in [
        (8u8, OrgReject::LeaderNotAssignable),
        (5, OrgReject::RankNotInType),
    ] {
        let capture = LogCapture::install();
        assert_eq!(
            gm_rank(&fx.ctx(), gm, &fx.name(1), rank, Some(team)).await,
            Err(why)
        );
        one_row(&capture, "org.gm_rank", "rejected", Some(why.reason()));
    }
    let capture = LogCapture::install();
    assert_eq!(
        gm_rank(&fx.ctx(), gm, &fx.name(0), 2, Some(team)).await,
        Err(OrgReject::LeaderNotAssignable),
        "the Leader is never moved off Leader"
    );
    one_row(
        &capture,
        "org.gm_rank",
        "rejected",
        Some("leader_not_assignable"),
    );
    assert_eq!(fx.rank_of(team, 1).await, Some(3));
    assert_eq!(fx.rank_of(team, 0).await, Some(8));
    fx.teardown().await;
}
