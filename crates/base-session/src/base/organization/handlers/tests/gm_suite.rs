//! ORG-10: the rest of the GM suite on the base (`.org_info`, `.org_list`,
//! `.org_set_perms`, `gmReloadOrganizations`), and the `org.gm_action`
//! audit row every GM organization command writes (D-ORG13).
//!
//! Live-DB (TESTING.md type 3) with the client-bound calls captured
//! (types 2 and 8) and every row checked with `LogCapture` (type 12).

use cimmeria_entity::organization::OrgPermission;
use cimmeria_wire::cell::client_methods::organization::{
    ON_ORGANIZATION_JOINED, ON_ORGANIZATION_RANK_UPDATE,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::{
    gm_disband, gm_info, gm_join, gm_list, gm_rank, gm_reload, gm_set_perms, GmCaller, OrgReject,
};
use crate::test_support::{require_db_or_skip, LogCapture};

impl Fixture {
    /// Put character `i` in the world as a GameMaster.
    fn gm_online(&self, i: usize) -> GmCaller {
        self.online(i);
        self.gm(i, 2)
    }

    /// `rank`'s stored mask in `org_id`.
    async fn mask_of(&self, org_id: i32, rank: u8) -> OrgPermission {
        let bits: i32 = sqlx::query_scalar(
            "SELECT permissions FROM sgw_organization_ranks WHERE org_id = $1 AND rank = $2",
        )
        .bind(org_id)
        .bind(i16::from(rank))
        .fetch_one(&self.pool)
        .await
        .unwrap();
        OrgPermission::from_wire(bits)
    }

    /// The feedback lines character `i` has received.
    fn lines_to(&self, i: usize) -> Vec<String> {
        feedback_lines(&self.calls_to(i))
    }
}

/// `.org_info <player>` lists every membership of an **offline** character
/// with its rank and mask; with no name it reports the GM. One `ok`
/// `org.gm_action` row carries the target and the count.
#[tokio::test]
async fn live_db_gm_info_lists_every_membership_with_rank_and_mask() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 0, 3, &["Org10 Info Team", "Org10 Info Command"]).await;
    let team = fx.org(OrgType::Team, "Org10 Info Team", 0, &[1]).await;
    let command = fx.org(OrgType::Command, "Org10 Info Command", 1, &[]).await;
    let gm = fx.gm_online(2);
    let capture = LogCapture::install();
    assert_eq!(gm_info(&fx.ctx(), gm, Some(&fx.name(1))).await, Ok(2));
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_org_info"));
    assert!(row.has_field("count", "2"));
    assert!(row.has_field("target_player_id", &fx.player_id(1).to_string()));
    assert!(row.has_field("player_id", &fx.player_id(2).to_string()));
    let lines = fx.lines_to(2);
    assert!(lines[0].contains("2 organization(s)"), "{lines:?}");
    let team_rank = fx.rank_of(team, 1).await.unwrap() as u8;
    let team_mask = fx.mask_of(team, team_rank).await.bits();
    assert!(
        lines.iter().any(|l| l.contains(&format!("(id {team})"))
            && l.contains(&format!("rank {team_rank},"))
            && l.contains(&format!("{team_mask:#09x}"))),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&format!("(id {command})")) && l.contains("rank 8,")),
        "{lines:?}"
    );

    fx.clear_sent();
    let capture = LogCapture::install();
    assert_eq!(
        gm_info(&fx.ctx(), gm, None).await,
        Ok(0),
        "the GM is in none"
    );
    one_row(&capture, "org.gm_action", "ok", None);

    let capture = LogCapture::install();
    assert_eq!(
        gm_info(&fx.ctx(), gm, Some("Org10 Nobody")).await,
        Err(OrgReject::TargetNotFound)
    );
    one_row(
        &capture,
        "org.gm_action",
        "rejected",
        Some("target_not_found"),
    );

    let player = fx.gm(2, 0);
    let capture = LogCapture::install();
    assert_eq!(
        gm_info(&fx.ctx(), player, None).await,
        Err(OrgReject::NotGm)
    );
    one_row(&capture, "org.gm_action", "rejected", Some("not_gm"));
    fx.teardown().await;
}

/// `.org_list` names every organization with its member count and leader.
#[tokio::test]
async fn live_db_gm_list_lists_every_organization() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 1, 3, &["Org10 List Team", "Org10 List Command"]).await;
    let team = fx.org(OrgType::Team, "Org10 List Team", 0, &[1]).await;
    let command = fx.org(OrgType::Command, "Org10 List Command", 1, &[]).await;
    let gm = fx.gm_online(2);
    let capture = LogCapture::install();
    let listed = gm_list(&fx.ctx(), gm).await.expect("list");
    assert!(listed >= 2);
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_org_list"));
    let lines = fx.lines_to(2);
    assert!(
        lines.iter().any(|l| l.contains(&format!(
            "Team 'Org10 List Team' (id {team}): 2 member(s), leader {}",
            fx.name(0)
        ))),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains(&format!(
            "Command 'Org10 List Command' (id {command}): 1 member(s), leader {}",
            fx.name(1)
        ))),
        "{lines:?}"
    );
    fx.teardown().await;
}

/// `.org_set_perms` goes through `apply_edit`: bits the type's editor does
/// not show keep their stored value (the D-ORG09 (6) clamp), the GM reads
/// which bits were ignored, and every online member gets the new rank table
/// [49].
#[tokio::test]
async fn live_db_gm_set_perms_clamps_to_the_editor_bits_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 2, 3, &["Org10 Perms"]).await;
    let team = fx.org(OrgType::Team, "Org10 Perms", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    let gm = fx.gm_online(2);
    let rank = fx.rank_of(team, 1).await.unwrap() as u8;
    let old = fx.mask_of(team, rank).await;
    let editable = OrgPermission::editable_for(OrgType::Team);
    // Every bit: the editable ones turn on, the hidden ones stay as stored.
    let want = OrgPermission::from_bits_truncate((old.bits() & !editable.bits()) | editable.bits());
    assert_ne!(want, old, "the fixture rank must lack an editable bit");
    let capture = LogCapture::install();
    let edit = gm_set_perms(&fx.ctx(), gm, team, rank, u32::MAX)
        .await
        .expect("set perms");
    assert_eq!((edit.from, edit.to), (old, want));
    assert_eq!(fx.mask_of(team, rank).await, want, "stored");
    assert_eq!(
        edit.ignored.bits(),
        OrgPermission::ALL.bits() & !editable.bits(),
        "every non-editable bit of the GM's mask was ignored"
    );
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_org_set_perms"));
    assert!(row.has_field("rank", &rank.to_string()));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "permissions_changed")
            && c.has_field("from_mask", &old.bits().to_string())
            && c.has_field("to_mask", &want.bits().to_string())));
    for i in [0, 1] {
        assert!(
            fx.calls_to(i)
                .iter()
                .any(|c| c.0 == ON_ORGANIZATION_RANK_UPDATE),
            "member {i} got the rank table"
        );
    }
    assert!(
        fx.lines_to(2)[0].contains("were ignored"),
        "{:?}",
        fx.lines_to(2)
    );

    // The same mask again changes nothing.
    let capture = LogCapture::install();
    assert_eq!(
        gm_set_perms(&fx.ctx(), gm, team, rank, u32::MAX).await,
        Err(OrgReject::PermissionsUnchanged)
    );
    one_row(
        &capture,
        "org.gm_action",
        "rejected",
        Some("permissions_unchanged"),
    );
    fx.teardown().await;
}

/// The `Leader` row, a rank the type does not use, an unknown organization
/// and a non-GM are refused, and nothing is written.
#[tokio::test]
async fn live_db_gm_set_perms_refuses_the_leader_row_and_unused_ranks() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 3, 2, &["Org10 Leader Row"]).await;
    let team = fx.org(OrgType::Team, "Org10 Leader Row", 0, &[]).await;
    let gm = fx.gm_online(1);
    let leader_mask = fx.mask_of(team, 8).await;
    for (org_id, rank, why) in [
        (team, 8u8, OrgReject::LeaderRowPinned),
        (team, 5, OrgReject::RankNotInType),
        (team, 0, OrgReject::RankNotInType),
        (i32::MAX >> 2, 2, OrgReject::NoSuchOrg),
    ] {
        let capture = LogCapture::install();
        assert_eq!(
            gm_set_perms(&fx.ctx(), gm, org_id, rank, 0).await,
            Err(why),
            "rank {rank}"
        );
        one_row(&capture, "org.gm_action", "rejected", Some(why.reason()));
    }
    assert_eq!(
        fx.mask_of(team, 8).await,
        leader_mask,
        "Leader row untouched"
    );
    assert_eq!(leader_mask, OrgPermission::ALL);

    let player = fx.gm(1, 0);
    let capture = LogCapture::install();
    assert_eq!(
        gm_set_perms(&fx.ctx(), player, team, 2, 0).await,
        Err(OrgReject::NotGm)
    );
    one_row(&capture, "org.gm_action", "rejected", Some("not_gm"));
    fx.teardown().await;
}

/// `gmReloadOrganizations` re-sends the GM's own push for each Team and
/// Command (one [35] each) and tells nobody else anything.
#[tokio::test]
async fn live_db_gm_reload_resends_the_login_push() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 4, 2, &["Org10 Reload Team", "Org10 Reload Command"]).await;
    fx.org(OrgType::Team, "Org10 Reload Team", 0, &[1]).await;
    fx.org(OrgType::Command, "Org10 Reload Command", 0, &[])
        .await;
    fx.online(1);
    let gm = fx.gm_online(0);
    let capture = LogCapture::install();
    assert_eq!(gm_reload(&fx.ctx(), gm).await, Ok(2));
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_reload_organizations"));
    assert!(row.has_field("count", "2"));
    let joined = fx
        .calls_to(0)
        .iter()
        .filter(|c| c.0 == ON_ORGANIZATION_JOINED)
        .count();
    assert_eq!(joined, 2, "one [35] per organization");
    assert!(fx.lines_to(0).iter().any(|l| l.contains("re-sent 2")));
    assert!(
        fx.calls_to(1).is_empty(),
        "the other member is told nothing"
    );

    let player = fx.gm(0, 0);
    fx.clear_sent();
    let capture = LogCapture::install();
    assert_eq!(gm_reload(&fx.ctx(), player).await, Err(OrgReject::NotGm));
    one_row(&capture, "org.gm_action", "rejected", Some("not_gm"));
    assert!(
        !fx.calls_to(0).iter().any(|c| c.0 == ON_ORGANIZATION_JOINED),
        "a refused reload pushes nothing"
    );
    fx.teardown().await;
}

/// D-ORG13 audit gap (ORG-10): `.org_join`, `.org_rank` and `.org_disband`
/// each write exactly one `org.gm_action` row with the GM and the result,
/// refused or not, beside their own outcome row; before, a refusal ahead of
/// the lock (`not_gm`) left none, and a success left only the lock-time
/// audit with no result.
#[tokio::test]
async fn live_db_every_gm_org_command_writes_one_gm_action_row() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org10(&pool, 5, 3, &["Org10 Audit"]).await;
    let team = fx.org(OrgType::Team, "Org10 Audit", 0, &[]).await;
    fx.online(1);
    let gm = fx.gm_online(2);

    let capture = LogCapture::install();
    gm_join(&fx.ctx(), gm, team, Some(&fx.name(1)))
        .await
        .expect("join");
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_org_join"));
    assert!(row.has_field("target_player_id", &fx.player_id(1).to_string()));
    assert!(row.has_field("player_id", &fx.player_id(2).to_string()));
    one_row(&capture, "org.gm_join", "ok", None);
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "org.gm_access")),
        "the lock-time audit keeps its own event"
    );

    let capture = LogCapture::install();
    gm_rank(&fx.ctx(), gm, &fx.name(1), 3, Some(team))
        .await
        .expect("rank");
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "gm_org_rank"));

    let player = fx.gm(1, 0);
    for (name, event) in [
        ("join", "org.gm_join"),
        ("rank", "org.gm_rank"),
        ("disband", "org.disband"),
    ] {
        let capture = LogCapture::install();
        let refused = match name {
            "join" => gm_join(&fx.ctx(), player, team, None).await.err(),
            "rank" => gm_rank(&fx.ctx(), player, &fx.name(0), 2, Some(team))
                .await
                .err(),
            _ => gm_disband(&fx.ctx(), player, team).await.err(),
        };
        assert_eq!(refused, Some(OrgReject::NotGm), "{name}");
        one_row(&capture, "org.gm_action", "rejected", Some("not_gm"));
        one_row(&capture, event, "rejected", Some("not_gm"));
    }

    let capture = LogCapture::install();
    gm_disband(&fx.ctx(), gm, team).await.expect("disband");
    let row = one_row(&capture, "org.gm_action", "ok", None);
    assert!(row.has_field("action", "disband"));
    fx.teardown().await;
}
