//! The rank editor for Teams and Commands (ORG-08): CAT-M-07 (D-ORG09 (3),
//! (6), D-ORG22), CAT-M-08 (`RankNames`, D-ORG10) and the [49] [50]
//! fanouts.

use cimmeria_entity::organization::{default_rank_permissions, OrgPermission};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_rank_name_update, build_on_organization_rank_update,
    ON_ORGANIZATION_RANK_NAME_UPDATE, ON_ORGANIZATION_RANK_UPDATE,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::answer::{
    CHANGES_UNHELD_BITS_TEXT, LEADER_PINNED_TEXT, NO_PERMISSION_TEXT, OWN_RANK_TEXT,
    RANK_NOT_IN_TYPE_TEXT, RANK_TOO_LOW_TEXT, TEXT_EMPTY_TEXT,
};
use crate::base::organization::handlers::{
    handle_set_rank_name, handle_set_rank_permissions, OrgReject,
};
use crate::base::organization::persistence::load_ranks;
use crate::test_support::{require_db_or_skip, LogCapture};

fn default_of(org_type: OrgType, rank: OrgRank) -> OrgPermission {
    default_rank_permissions(org_type)
        .into_iter()
        .find(|(r, _)| *r == rank)
        .unwrap()
        .1
}

/// The [49] every member should get: the whole table as stored now.
async fn rank_table(fx: &Fixture, org_id: i32) -> Vec<u8> {
    let ranks = load_ranks(&fx.pool, org_id).await.unwrap();
    let table: Vec<_> = ranks.iter().map(|r| (r.rank, r.permissions)).collect();
    build_on_organization_rank_update(org_id, &table)
}

/// CAT-M-07, D-ORG09 (6) as D-ORG22 applies it: a Senior Officer (holding
/// `AlterPerms` but not `WithdrawCash`) cannot grant `WithdrawCash`, nor
/// strip it once the server set it; an edit that leaves the unheld bit
/// alone and grants a held bit goes through.
#[tokio::test]
async fn set_perms_rejects_grant_of_unheld_bit() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 7, 2, &["Org08 Unheld"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Unheld", 0, &[1]).await;
    fx.set_rank_of(cmd, 1, OrgRank::SENIOR_OFFICER).await;
    fx.online(1);
    let member = default_of(OrgType::Command, OrgRank::MEMBER);
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_permissions(
            &fx.ctx(),
            &fx.player(1),
            cmd,
            2,
            (member | OrgPermission::WITHDRAW_CASH).to_wire()
        )
        .await,
        Err(OrgReject::ChangesUnheldBits)
    );
    let row = one_row(
        &capture,
        "org.set_rank_permissions",
        "rejected",
        Some("changes_unheld_bits"),
    );
    assert!(
        row.has_field(
            "unheld_mask",
            &OrgPermission::WITHDRAW_CASH.bits().to_string()
        ) && row.has_field("from_mask", &member.bits().to_string())
            && row.has_field("actor_rank", "7"),
        "{row:?}"
    );
    assert_eq!(fx.perms_of(cmd, OrgRank::MEMBER).await, member);
    assert_eq!(
        feedback_lines(&fx.calls_to(1)),
        vec![CHANGES_UNHELD_BITS_TEXT]
    );

    // The server grants the unheld bit; the Senior Officer may not strip it
    // but may grant `Invite` beside it.
    let with_cash = member | OrgPermission::WITHDRAW_CASH;
    fx.set_perms_of(cmd, OrgRank::MEMBER, with_cash).await;
    assert_eq!(
        handle_set_rank_permissions(&fx.ctx(), &fx.player(1), cmd, 2, member.to_wire()).await,
        Err(OrgReject::ChangesUnheldBits),
        "revoking an unheld bit is refused too"
    );
    let wire = (with_cash | OrgPermission::INVITE).to_wire();
    let edit = handle_set_rank_permissions(&fx.ctx(), &fx.player(1), cmd, 2, wire)
        .await
        .expect("grant of a held bit");
    assert_eq!(edit.to, with_cash | OrgPermission::INVITE);
    assert_eq!(
        fx.perms_of(cmd, OrgRank::MEMBER).await,
        with_cash | OrgPermission::INVITE
    );
    fx.teardown().await;
}

/// CAT-M-07, D-ORG09 (3): nobody edits their own rank's mask, the Leader
/// included (whose row is pinned anyway).
#[tokio::test]
async fn set_perms_rejects_own_rank() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 8, 2, &["Org08 Own Rank"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Own Rank", 0, &[1]).await;
    fx.set_rank_of(cmd, 1, OrgRank::SENIOR_OFFICER).await;
    fx.online(1);
    let before = fx.perms_of(cmd, OrgRank::SENIOR_OFFICER).await;
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_permissions(
            &fx.ctx(),
            &fx.player(1),
            cmd,
            7,
            (before | OrgPermission::WITHDRAW_BANK).to_wire()
        )
        .await,
        Err(OrgReject::OwnRank)
    );
    one_row(
        &capture,
        "org.set_rank_permissions",
        "rejected",
        Some("own_rank"),
    );
    assert_eq!(fx.perms_of(cmd, OrgRank::SENIOR_OFFICER).await, before);
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![OWN_RANK_TEXT]);
    fx.teardown().await;
}

/// CAT-M-07, D-ORG08: the `Leader` row is refused for any editor, the
/// Leader first.
#[tokio::test]
async fn set_perms_rejects_leader_row() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 9, 1, &["Org08 Crown"]).await;
    let team = fx.org(OrgType::Team, "Org08 Crown", 0, &[]).await;
    fx.online(0);
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_permissions(&fx.ctx(), &fx.player(0), team, 8, 0).await,
        Err(OrgReject::LeaderRowPinned)
    );
    one_row(
        &capture,
        "org.set_rank_permissions",
        "rejected",
        Some("leader_row_pinned"),
    );
    assert_eq!(fx.perms_of(team, OrgRank::LEADER).await, OrgPermission::ALL);
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![LEADER_PINNED_TEXT]);
    fx.teardown().await;
}

/// D-ORG09 (1), (2), (5): no `AlterPerms` is `missing_permission`; an
/// Officer granted `AlterPerms` still cannot edit the Senior Officer rank
/// above it (`rank_too_low`); a rank the type does not use is refused.
#[tokio::test]
async fn set_perms_needs_alter_perms_a_lower_rank_and_a_rank_in_type() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 10, 2, &["Org08 Editor Rules"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org08 Editor Rules", 0, &[1])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.online(1);
    let senior = fx.perms_of(cmd, OrgRank::SENIOR_OFFICER).await;
    let cases = [
        (7, OrgReject::MissingPermission),
        (0, OrgReject::RankNotInType),
        (-1, OrgReject::RankNotInType),
        (264, OrgReject::RankNotInType),
    ];
    for (rank, why) in cases {
        let capture = LogCapture::install();
        assert_eq!(
            handle_set_rank_permissions(&fx.ctx(), &fx.player(1), cmd, rank, 0).await,
            Err(why),
            "rank {rank}"
        );
        one_row(
            &capture,
            "org.set_rank_permissions",
            "rejected",
            Some(why.reason()),
        );
    }
    let officer = fx.perms_of(cmd, OrgRank::OFFICER).await;
    fx.set_perms_of(cmd, OrgRank::OFFICER, officer | OrgPermission::ALTER_PERMS)
        .await;
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_permissions(&fx.ctx(), &fx.player(1), cmd, 7, 0).await,
        Err(OrgReject::RankTooLow)
    );
    one_row(
        &capture,
        "org.set_rank_permissions",
        "rejected",
        Some("rank_too_low"),
    );
    assert_eq!(fx.perms_of(cmd, OrgRank::SENIOR_OFFICER).await, senior);
    let lines = feedback_lines(&fx.calls_to(1));
    assert_eq!(lines[0], NO_PERMISSION_TEXT);
    assert_eq!(lines[1], RANK_NOT_IN_TYPE_TEXT);
    assert_eq!(lines[4], RANK_TOO_LOW_TEXT);
    fx.teardown().await;
}

/// A permission edit stores D-ORG22's mask (the bits the editor does not
/// show keep their value whatever the wire says), then every online member
/// gets [49] with the whole table; one `ok` row carries all three masks.
#[tokio::test]
async fn set_perms_updates_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 11, 3, &["Org08 Perms"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Perms", 0, &[1, 2]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let member = default_of(OrgType::Command, OrgRank::MEMBER);
    // The wire carries only `Invite`: the hidden `RosterNotes` and
    // `ViewBankLogs` survive, the shown bank bits go.
    let wire = OrgPermission::INVITE.to_wire();
    let capture = LogCapture::install();
    let edit = handle_set_rank_permissions(&fx.ctx(), &fx.player(0), cmd, 2, wire)
        .await
        .expect("edit");
    let stored = fx.perms_of(cmd, OrgRank::MEMBER).await;
    assert_eq!(edit.to, stored);
    assert!(stored.contains(OrgPermission::INVITE));
    assert!(stored.contains(OrgPermission::ROSTER_NOTES));
    assert!(stored.contains(OrgPermission::VIEW_BANK_LOGS));
    assert!(!stored.contains(OrgPermission::DEPOSIT_BANK));
    let row = one_row(&capture, "org.set_rank_permissions", "ok", None);
    assert!(
        row.has_field("from_mask", &member.bits().to_string())
            && row.has_field("to_mask", &stored.bits().to_string())
            && row.has_field("wire_mask", &wire.to_string())
            && row.has_field("after", "changed"),
        "{row:?}"
    );
    let table = rank_table(&fx, cmd).await;
    for i in 0..3 {
        assert_eq!(
            fx.calls_of(i, ON_ORGANIZATION_RANK_UPDATE),
            vec![table.clone()],
            "member {i}"
        );
    }
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec!["Permissions for rank 2 saved."]
    );

    // The same edit again changes nothing: no [49], still a line.
    fx.clear_sent();
    let capture = LogCapture::install();
    handle_set_rank_permissions(&fx.ctx(), &fx.player(0), cmd, 2, wire)
        .await
        .expect("unchanged");
    assert!(
        one_row(&capture, "org.set_rank_permissions", "ok", None).has_field("after", "unchanged")
    );
    assert!(fx.calls_of(1, ON_ORGANIZATION_RANK_UPDATE).is_empty());
    assert_eq!(feedback_lines(&fx.calls_to(0)).len(), 1);
    fx.teardown().await;
}

/// CAT-M-08: CM 17 needs `RankNames`; an Officer (6) lacks it.
#[tokio::test]
async fn set_rank_name_rejects_without_perm() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 12, 2, &["Org08 Names Perm"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Names Perm", 0, &[1]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.online(1);
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_name(&fx.ctx(), &fx.player(1), cmd, 2, "Grunt").await,
        Err(OrgReject::MissingPermission)
    );
    one_row(
        &capture,
        "org.set_rank_name",
        "rejected",
        Some("missing_permission"),
    );
    assert_eq!(fx.rank_name_of(cmd, OrgRank::MEMBER).await, None);
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NO_PERMISSION_TEXT]);
    fx.teardown().await;
}

/// CAT-M-08, D-ORG10 / D-ORG23: a rank name over 32 units, or one that is
/// empty once trimmed, is rejected; so is the actor's own rank (D-ORG09
/// (3)), which is why even the Leader cannot rename rank 8.
#[tokio::test]
async fn set_rank_name_rejects_over_cap() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 13, 1, &["Org08 Names Cap"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Names Cap", 0, &[]).await;
    fx.online(0);
    let long = "N".repeat(33);
    let cases: [(i32, &str, &str); 4] = [
        (2, &long, "too_long"),
        (2, "   ", "too_short"),
        (2, "Bad\u{200F}Name", "bidi_control"),
        (8, "Boss", "own_rank"),
    ];
    for (rank, name, reason) in cases {
        let capture = LogCapture::install();
        let got = handle_set_rank_name(&fx.ctx(), &fx.player(0), cmd, rank, name).await;
        assert_eq!(got.map_err(|e| e.reason()), Err(reason), "{reason}");
        one_row(&capture, "org.set_rank_name", "rejected", Some(reason));
    }
    assert_eq!(fx.rank_name_of(cmd, OrgRank::MEMBER).await, None);
    assert_eq!(fx.rank_name_of(cmd, OrgRank::LEADER).await, None);
    let lines = feedback_lines(&fx.calls_to(0));
    assert_eq!(lines[0], "That text is too long (at most 32 characters).");
    assert_eq!(lines[1], TEXT_EMPTY_TEXT);
    assert_eq!(lines[3], OWN_RANK_TEXT);
    fx.teardown().await;
}

/// A rank name is stored trimmed and collapsed, then every online member
/// gets [50] with every custom name (the renamed rank included).
#[tokio::test]
async fn set_rank_name_updates_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 14, 2, &["Org08 Names"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Names", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    handle_set_rank_name(&fx.ctx(), &fx.player(0), cmd, 6, "Lieutenant")
        .await
        .expect("first name");
    fx.clear_sent();
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_rank_name(&fx.ctx(), &fx.player(0), cmd, 2, "  Grunt   Squad ").await,
        Ok(true)
    );
    let row = one_row(&capture, "org.set_rank_name", "ok", None);
    assert!(row.has_field("to_units", "11") && row.has_field("from_units", "0"));
    assert_eq!(
        fx.rank_name_of(cmd, OrgRank::MEMBER).await.as_deref(),
        Some("Grunt Squad")
    );
    let args = build_on_organization_rank_name_update(
        cmd,
        &[
            (OrgRank::MEMBER, "Grunt Squad"),
            (OrgRank::OFFICER, "Lieutenant"),
        ],
    );
    for i in 0..2 {
        assert_eq!(
            fx.calls_of(i, ON_ORGANIZATION_RANK_NAME_UPDATE),
            vec![args.clone()]
        );
    }
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec!["Rank 2 is now named Grunt Squad."]
    );
    fx.teardown().await;
}
