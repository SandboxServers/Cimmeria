//! Officer-note visibility when who may read them changes (ORG-08,
//! CAT-M-10): a rank-permission edit that moves `OfficerNotes`, a rank
//! change between ranks that differ in it, and the login push.

use cimmeria_entity::organization::OrgPermission;
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_officer_note_update, build_on_organization_roster_info, RosterInfo,
    ON_MEMBER_RANK_CHANGED_ORGANIZATION, ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
    ON_ORGANIZATION_RANK_UPDATE, ON_ORGANIZATION_ROSTER_INFO,
};
use tracing::Level;

use super::*;
use crate::base::organization::api::OrgHeader;
use crate::base::organization::handlers::{
    handle_rank_change, handle_set_rank_permissions, org_state_messages,
};
use crate::base::organization::persistence::{OrgMembership, RankRow, RosterMember};
use crate::test_support::{require_db_or_skip, LogCapture};

/// Revoking `OfficerNotes` from the Officer rank sends its online members
/// an empty [47] for every stored note, after the [49]; a Member (whose
/// access did not change) gets only the [49]. Granting it back sends the
/// text.
#[tokio::test]
async fn revoking_officer_notes_blanks_them_for_that_rank() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 15, 3, &["Org08 Revoke"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Revoke", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::MEMBER).await;
    fx.set_officer_note_of(cmd, 2, "watch").await;
    for i in 0..3 {
        fx.online(i);
    }
    let officer = fx.perms_of(cmd, OrgRank::OFFICER).await;
    let without =
        OrgPermission::from_bits_truncate(officer.bits() & !OrgPermission::OFFICER_NOTES.bits());
    handle_set_rank_permissions(&fx.ctx(), &fx.player(0), cmd, 6, without.to_wire())
        .await
        .expect("revoke");
    let blank = (
        ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
        build_on_organization_officer_note_update(cmd, &fx.name(2), ""),
    );
    let officer_calls = fx.calls_to(1);
    let methods: Vec<u16> = officer_calls.iter().map(|c| c.0).collect();
    assert_eq!(
        methods,
        vec![
            ON_ORGANIZATION_RANK_UPDATE,
            ON_ORGANIZATION_OFFICER_NOTE_UPDATE
        ],
        "[49], then the blank note"
    );
    assert_eq!(officer_calls[1], blank);
    assert!(fx
        .calls_of(2, ON_ORGANIZATION_OFFICER_NOTE_UPDATE)
        .is_empty());
    assert!(
        fx.calls_of(0, ON_ORGANIZATION_OFFICER_NOTE_UPDATE)
            .is_empty(),
        "the Leader's rank did not change"
    );

    fx.clear_sent();
    handle_set_rank_permissions(&fx.ctx(), &fx.player(0), cmd, 6, officer.to_wire())
        .await
        .expect("grant");
    assert_eq!(
        fx.calls_of(1, ON_ORGANIZATION_OFFICER_NOTE_UPDATE),
        vec![build_on_organization_officer_note_update(
            cmd,
            &fx.name(2),
            "watch"
        )]
    );
    fx.teardown().await;
}

/// A rank change that moves an online member out of a rank holding
/// `OfficerNotes` blanks the notes on their client, after the [40]; a move
/// between two ranks that both hold it sends none.
#[tokio::test]
async fn rank_change_out_of_officer_notes_blanks_them() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 16, 3, &["Org08 Demoted"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Demoted", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_officer_note_of(cmd, 2, "keep an eye").await;
    fx.online(0);
    fx.online(1);
    handle_rank_change(&fx.ctx(), &fx.player(0), cmd, &fx.name(1), 7)
        .await
        .expect("to Senior Officer");
    assert!(
        fx.calls_of(1, ON_ORGANIZATION_OFFICER_NOTE_UPDATE)
            .is_empty(),
        "6 and 7 both hold OfficerNotes"
    );
    fx.clear_sent();
    let capture = LogCapture::install();
    handle_rank_change(&fx.ctx(), &fx.player(0), cmd, &fx.name(1), 2)
        .await
        .expect("to Member");
    let methods: Vec<u16> = fx.calls_to(1).iter().map(|c| c.0).collect();
    assert_eq!(
        &methods[..2],
        &[
            ON_MEMBER_RANK_CHANGED_ORGANIZATION,
            ON_ORGANIZATION_OFFICER_NOTE_UPDATE
        ]
    );
    assert_eq!(
        fx.calls_of(1, ON_ORGANIZATION_OFFICER_NOTE_UPDATE),
        vec![build_on_organization_officer_note_update(
            cmd,
            &fx.name(2),
            ""
        )]
    );
    assert!(capture.all().iter().any(|c| c.level == Level::DEBUG
        && c.has_field("event", "org.officer_note_sync")
        && c.has_field("show", "false")));
    fx.teardown().await;
}

/// ORG-10's `.org_set_perms` goes through the same permission-edit path
/// (ORG-08): a GM revoke of `OfficerNotes` from the Officer rank sends that
/// rank's online members the [49] and then the blank [47]s, exactly like a
/// member's CM 16.
#[tokio::test]
async fn gm_set_perms_moving_officer_notes_sends_the_note_sync() {
    use crate::base::organization::handlers::gm_set_perms;

    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 19, 4, &["Org08 Gm Perms"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Gm Perms", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::MEMBER).await;
    fx.set_officer_note_of(cmd, 2, "watch").await;
    fx.online(1);
    fx.online(2);
    fx.online(3);
    let gm = fx.gm(3, 2);
    let officer = fx.perms_of(cmd, OrgRank::OFFICER).await;
    let without = officer.bits() & !OrgPermission::OFFICER_NOTES.bits();
    let edit = gm_set_perms(&fx.ctx(), gm, cmd, 6, without)
        .await
        .expect("GM revoke");
    assert!(!edit.to.contains(OrgPermission::OFFICER_NOTES));
    let methods: Vec<u16> = fx.calls_to(1).iter().map(|c| c.0).collect();
    assert_eq!(
        methods,
        vec![
            ON_ORGANIZATION_RANK_UPDATE,
            ON_ORGANIZATION_OFFICER_NOTE_UPDATE
        ],
        "[49], then the blank note"
    );
    assert_eq!(
        fx.calls_of(1, ON_ORGANIZATION_OFFICER_NOTE_UPDATE),
        vec![build_on_organization_officer_note_update(
            cmd,
            &fx.name(2),
            ""
        )]
    );
    assert!(
        fx.calls_of(2, ON_ORGANIZATION_OFFICER_NOTE_UPDATE)
            .is_empty(),
        "the Member's access did not change"
    );
    assert!(
        fx.calls_of(3, ON_ORGANIZATION_OFFICER_NOTE_UPDATE)
            .is_empty(),
        "the GM is not a member"
    );
    fx.teardown().await;
}

/// Negative seam: a member whose session cannot be reached during the sync
/// is WARN `org.send_failed` with `what = officer_note_sync` and the reason.
#[tokio::test]
async fn an_unsendable_officer_note_sync_warns_with_reason() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 17, 3, &["Org08 Sync Fail"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org08 Sync Fail", 0, &[1, 2])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_officer_note_of(cmd, 2, "n").await;
    fx.online(0);
    fx.online(1);
    fx.entity_to_addr.lock().unwrap().remove(&fx.entity(1));
    let officer = fx.perms_of(cmd, OrgRank::OFFICER).await;
    let without =
        OrgPermission::from_bits_truncate(officer.bits() & !OrgPermission::OFFICER_NOTES.bits());
    let capture = LogCapture::install();
    handle_set_rank_permissions(&fx.ctx(), &fx.player(0), cmd, 6, without.to_wire())
        .await
        .expect("revoke");
    let warn = capture
        .all()
        .into_iter()
        .find(|c| {
            c.level == Level::WARN
                && c.has_field("event", "org.send_failed")
                && c.has_field("what", "officer_note_sync")
        })
        .expect("org.send_failed for the sync");
    assert!(warn.has_field("reason", "entity_to_addr_miss"), "{warn:?}");
    assert!(warn.has_field("target_player_id", &fx.player_id(1).to_string()));
    fx.teardown().await;
}

/// The order guard (`handlers::order`): while another edit of the same
/// organization holds it, a CM 15, a CM 16 and a rank change all wait
/// before they open their transaction, so their post-commit sends cannot
/// cross that edit's. So does the GM `.org_set_perms` (ORG-10), which
/// shares CM 16's path. Each is started with the guard held and must not
/// finish (nor write) until it is released.
#[tokio::test]
async fn edits_wait_for_the_org_order_guard() {
    use std::time::Duration;

    use crate::base::organization::handlers::order::org_order_guard;
    use crate::base::organization::handlers::{handle_set_text, TextEdit};

    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 18, 3, &["Org08 Order"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Order", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.online(0);
    let ctx = fx.ctx();
    let actor = fx.player(0);
    let target = fx.name(2);
    let wait = Duration::from_millis(300);

    let held = org_order_guard(cmd).await;
    let note = TextEdit::OfficerNote {
        target_name: &target,
    };
    assert!(
        tokio::time::timeout(wait, handle_set_text(&ctx, &actor, cmd, note, "x"))
            .await
            .is_err(),
        "CM 15 ran while the guard was held"
    );
    assert!(
        tokio::time::timeout(wait, handle_set_rank_permissions(&ctx, &actor, cmd, 2, 1))
            .await
            .is_err(),
        "CM 16 ran while the guard was held"
    );
    assert!(
        tokio::time::timeout(wait, handle_rank_change(&ctx, &actor, cmd, &target, 2))
            .await
            .is_err(),
        "the rank change ran while the guard was held"
    );
    fx.online(1);
    let gm = fx.gm(1, 2);
    assert!(
        tokio::time::timeout(
            wait,
            crate::base::organization::handlers::gm_set_perms(&ctx, gm, cmd, 2, 1)
        )
        .await
        .is_err(),
        "the GM .org_set_perms ran while the guard was held"
    );
    assert_eq!(fx.notes_of(cmd, 2).await.1, "");
    assert_eq!(fx.rank_of(cmd, 2).await, Some(1));
    assert!(fx.calls_to(0).is_empty());
    drop(held);
    handle_set_text(&ctx, &actor, cmd, note, "x")
        .await
        .expect("CM 15 after the guard");
    handle_rank_change(&ctx, &actor, cmd, &target, 2)
        .await
        .expect("rank change after the guard");
    fx.teardown().await;
}

fn member(player_id: i32, name: &str, rank: OrgRank, officer_note: &str) -> RosterMember {
    RosterMember {
        player_id,
        name: name.into(),
        level: 12,
        archetype: 3,
        rank,
        note: "n".into(),
        officer_note: officer_note.into(),
    }
}

/// The [38] roster in a login push.
fn roster_of(messages: &[(u16, Vec<u8>)]) -> Vec<u8> {
    messages
        .iter()
        .find(|m| m.0 == ON_ORGANIZATION_ROSTER_INFO)
        .unwrap()
        .1
        .clone()
}

/// CAT-M-10 in the login push: officer notes reach a recipient only if
/// the push's own rank read gives their rank `OfficerNotes`; a rank with
/// no row reads nothing (fail closed). Roster notes always go.
#[test]
fn login_push_hides_officer_notes_without_the_permission() {
    let header = OrgHeader {
        org_id: 9,
        org_type: OrgType::Command,
        name: "Ab".into(),
        motd: String::new(),
        cash: 0,
        experience: 0,
    };
    let roster = vec![
        member(100, "A", OrgRank::LEADER, "boss"),
        member(101, "B", OrgRank::MEMBER, "slacker"),
    ];
    let ranks = vec![
        RankRow {
            rank: OrgRank::MEMBER,
            name: None,
            permissions: OrgPermission::ROSTER_NOTES,
        },
        RankRow {
            rank: OrgRank::LEADER,
            name: None,
            permissions: OrgPermission::ALL,
        },
    ];
    let info = |shown: bool| -> Vec<u8> {
        let rows: Vec<RosterInfo> = roster
            .iter()
            .map(|m| RosterInfo {
                name: m.name.clone(),
                level: 12,
                archetype: 3,
                rank: m.rank,
                note: m.note.clone(),
                officer_note: if shown {
                    m.officer_note.clone()
                } else {
                    String::new()
                },
            })
            .collect();
        build_on_organization_roster_info(9, &rows)
    };
    let as_rank = |rank: OrgRank, ranks: &[RankRow]| {
        let membership = OrgMembership {
            header: header.clone(),
            rank,
            // Deliberately every bit: the push must judge from the rank
            // table it read, not from this display value.
            display_permissions: OrgPermission::ALL,
        };
        roster_of(&org_state_messages(&membership, ranks, &roster, &[], false))
    };
    assert_eq!(as_rank(OrgRank::LEADER, &ranks), info(true));
    assert_eq!(as_rank(OrgRank::MEMBER, &ranks), info(false));
    assert_eq!(
        as_rank(OrgRank::OFFICER, &ranks),
        info(false),
        "no rank row: fail closed"
    );
}
