//! `organizationMOTD`, `organizationNote` and `organizationOfficerNote` for
//! Teams and Commands (ORG-08): CAT-M-10 and the [45] [46] [47] fanouts.

use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_motd_update, build_on_organization_note_update,
    build_on_organization_officer_note_update, ON_ORGANIZATION_MOTD_UPDATE,
    ON_ORGANIZATION_NOTE_UPDATE, ON_ORGANIZATION_OFFICER_NOTE_UPDATE,
};

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::answer::{
    NO_PERMISSION_TEXT, RANK_TOO_LOW_TEXT, TARGET_NOT_MEMBER_TEXT, TEXT_NOT_ALLOWED_TEXT,
};
use crate::base::organization::handlers::texts::{MOTD_SAVED_TEXT, NOTE_SAVED_TEXT};
use crate::base::organization::handlers::{handle_set_text, OrgReject, TextEdit};
use crate::test_support::{require_db_or_skip, LogCapture};

/// CAT-M-10: CM 13 needs `MOTD`. A Member (2) of a Command lacks it; the
/// MOTD is not written, nobody else hears anything, and the actor reads why.
#[tokio::test]
async fn live_db_motd_rejects_without_perm() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 0, 2, &["Org08 Motd Perm"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Motd Perm", 0, &[1]).await;
    fx.set_rank_of(cmd, 1, OrgRank::MEMBER).await;
    fx.online(0);
    fx.online(1);
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(1), cmd, TextEdit::Motd, "Hello").await,
        Err(OrgReject::MissingPermission)
    );
    let row = one_row(
        &capture,
        "org.set_text",
        "rejected",
        Some("missing_permission"),
    );
    assert!(row.has_field("field", "motd") && row.has_field("actor_rank", "2"));
    assert_eq!(fx.motd_of(cmd).await, "");
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NO_PERMISSION_TEXT]);
    assert!(fx.calls_to(0).is_empty(), "nothing reaches the others");
    fx.teardown().await;
}

/// CM 13 writes the MOTD under the lock, then every online member gets
/// [45]; the same text again is `ok` / `unchanged` with no fanout, but the
/// actor still gets the line.
#[tokio::test]
async fn live_db_motd_updates_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 1, 3, &["Org08 Motd"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Motd", 0, &[1, 2]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let motd = "Muster at the gate.\nBring ammo.";
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(0), cmd, TextEdit::Motd, motd).await,
        Ok(true)
    );
    let row = one_row(&capture, "org.set_text", "ok", None);
    assert!(
        row.has_field("after", "changed")
            && row.has_field("from_units", "0")
            && row.has_field("to_units", &motd.encode_utf16().count().to_string()),
        "{row:?}"
    );
    assert_eq!(fx.motd_of(cmd).await, motd);
    let args = build_on_organization_motd_update(cmd, motd);
    for i in 0..3 {
        assert_eq!(
            fx.calls_of(i, ON_ORGANIZATION_MOTD_UPDATE),
            vec![args.clone()],
            "member {i}"
        );
    }
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![MOTD_SAVED_TEXT]);

    fx.clear_sent();
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(0), cmd, TextEdit::Motd, motd).await,
        Ok(false)
    );
    assert!(one_row(&capture, "org.set_text", "ok", None).has_field("after", "unchanged"));
    assert!(fx.calls_of(1, ON_ORGANIZATION_MOTD_UPDATE).is_empty());
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![MOTD_SAVED_TEXT]);
    fx.teardown().await;
}

/// CM 14 writes the actor's own note (`RosterNotes`: a Member has it, an
/// Initiate does not) and sends [46] with the stored name to everyone.
#[tokio::test]
async fn live_db_note_updates_own_note_and_fans_out() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 2, 3, &["Org08 Note"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Note", 0, &[1, 2]).await;
    fx.set_rank_of(cmd, 1, OrgRank::MEMBER).await;
    for i in 0..3 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(1), cmd, TextEdit::Note, "Medic").await,
        Ok(true)
    );
    one_row(&capture, "org.set_text", "ok", None);
    assert_eq!(fx.notes_of(cmd, 1).await.0, "Medic");
    let args = build_on_organization_note_update(cmd, &fx.name(1), "Medic");
    for i in 0..3 {
        assert_eq!(
            fx.calls_of(i, ON_ORGANIZATION_NOTE_UPDATE),
            vec![args.clone()]
        );
    }
    assert_eq!(feedback_lines(&fx.calls_to(1)), vec![NOTE_SAVED_TEXT]);

    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(2), cmd, TextEdit::Note, "Hi").await,
        Err(OrgReject::MissingPermission),
        "an Initiate holds no RosterNotes"
    );
    one_row(
        &capture,
        "org.set_text",
        "rejected",
        Some("missing_permission"),
    );
    assert_eq!(fx.notes_of(cmd, 2).await.0, "");
    fx.teardown().await;
}

/// CAT-M-10: an officer note's target resolves among the actor's
/// organization's members only. A character online and in another Command
/// is `target_not_member`, and their note there is untouched.
#[tokio::test]
async fn live_db_officer_note_rejects_target_outside_org() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 3, 3, &["Org08 Home", "Org08 Away"]).await;
    let home = fx.org(OrgType::Command, "Org08 Home", 0, &[1]).await;
    let away = fx.org(OrgType::Command, "Org08 Away", 2, &[]).await;
    for i in 0..3 {
        fx.online(i);
    }
    let edit = TextEdit::OfficerNote {
        target_name: &fx.name(2),
    };
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(&fx.ctx(), &fx.player(0), home, edit, "spy").await,
        Err(OrgReject::TargetNotMember)
    );
    one_row(
        &capture,
        "org.set_text",
        "rejected",
        Some("target_not_member"),
    );
    assert_eq!(fx.notes_of(away, 2).await.1, "");
    assert_eq!(
        feedback_lines(&fx.calls_to(0)),
        vec![TARGET_NOT_MEMBER_TEXT]
    );
    assert!(fx.calls_to(2).is_empty());
    fx.teardown().await;
}

/// CAT-M-10, D-ORG09 (2): an Officer (6) cannot write on a Senior Officer
/// (7) or on a peer, nor on themself.
#[tokio::test]
async fn live_db_officer_note_rejects_higher_rank_target() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 4, 4, &["Org08 Officers"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org08 Officers", 0, &[1, 2, 3])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::SENIOR_OFFICER).await;
    fx.set_rank_of(cmd, 3, OrgRank::OFFICER).await;
    fx.online(1);
    for (target, why) in [
        (2usize, OrgReject::RankTooLow),
        (3, OrgReject::RankTooLow),
        (1, OrgReject::SelfTarget),
    ] {
        let name = fx.name(target);
        let capture = LogCapture::install();
        assert_eq!(
            handle_set_text(
                &fx.ctx(),
                &fx.player(1),
                cmd,
                TextEdit::OfficerNote { target_name: &name },
                "x"
            )
            .await,
            Err(why),
            "target {target}"
        );
        let row = one_row(&capture, "org.set_text", "rejected", Some(why.reason()));
        assert!(row.has_field("target_player_id", &fx.player_id(target).to_string()));
    }
    for i in 1..4 {
        assert_eq!(fx.notes_of(cmd, i).await.1, "");
    }
    assert_eq!(feedback_lines(&fx.calls_to(1))[0], RANK_TOO_LOW_TEXT);
    fx.teardown().await;
}

/// [47] reaches only the members whose rank holds `OfficerNotes`: the
/// Leader and an Officer, not a Member. The name is the stored one, not
/// the case the client typed.
#[tokio::test]
async fn live_db_officer_note_fanout_filtered_by_permission() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 5, 3, &["Org08 Eyes Only"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org08 Eyes Only", 0, &[1, 2])
        .await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    fx.set_rank_of(cmd, 2, OrgRank::MEMBER).await;
    for i in 0..3 {
        fx.online(i);
    }
    let typed = fx.name(2).to_lowercase();
    let capture = LogCapture::install();
    assert_eq!(
        handle_set_text(
            &fx.ctx(),
            &fx.player(0),
            cmd,
            TextEdit::OfficerNote {
                target_name: &typed
            },
            "Reliable."
        )
        .await,
        Ok(true)
    );
    let row = one_row(&capture, "org.set_text", "ok", None);
    assert!(row.has_field("field", "officer_note"));
    assert!(row.has_field("target_player_id", &fx.player_id(2).to_string()));
    assert_eq!(fx.notes_of(cmd, 2).await.1, "Reliable.");
    let args = build_on_organization_officer_note_update(cmd, &fx.name(2), "Reliable.");
    assert_eq!(
        fx.calls_of(0, ON_ORGANIZATION_OFFICER_NOTE_UPDATE),
        vec![args.clone()]
    );
    assert_eq!(
        fx.calls_of(1, ON_ORGANIZATION_OFFICER_NOTE_UPDATE),
        vec![args]
    );
    assert!(
        fx.calls_to(2).is_empty(),
        "a Member without OfficerNotes hears nothing, not even about themself"
    );
    fx.teardown().await;
}

/// CAT-M-10, D-ORG10 / D-ORG23: bidi controls, zero-width and other format
/// characters and over-cap text are rejected, never truncated or stored.
#[tokio::test]
async fn live_db_text_rejects_bidi_and_zero_width() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org08(&pool, 6, 2, &["Org08 Clean"]).await;
    let cmd = fx.org(OrgType::Command, "Org08 Clean", 0, &[1]).await;
    fx.online(0);
    let long = "x".repeat(129);
    let target = fx.name(1);
    let cases: [(TextEdit<'_>, &str, &str); 5] = [
        (TextEdit::Motd, "evil\u{202E}txt", "bidi_control"),
        (TextEdit::Note, "a\u{200B}b", "zero_width"),
        (TextEdit::Note, "soft\u{00AD}hyphen", "format_char"),
        (
            TextEdit::OfficerNote {
                target_name: &target,
            },
            "tag\u{E0041}",
            "format_char",
        ),
        (
            TextEdit::OfficerNote {
                target_name: &target,
            },
            &long,
            "too_long",
        ),
    ];
    for (edit, text, reason) in cases {
        let capture = LogCapture::install();
        let got = handle_set_text(&fx.ctx(), &fx.player(0), cmd, edit, text).await;
        assert!(
            matches!(got, Err(OrgReject::InvalidText(r)) if r.reason() == reason),
            "{reason}: {got:?}"
        );
        one_row(&capture, "org.set_text", "rejected", Some(reason));
    }
    assert_eq!(fx.motd_of(cmd).await, "");
    assert_eq!(fx.notes_of(cmd, 0).await, (String::new(), String::new()));
    assert_eq!(fx.notes_of(cmd, 1).await.1, "");
    let lines = feedback_lines(&fx.calls_to(0));
    assert_eq!(lines[0], TEXT_NOT_ALLOWED_TEXT);
    assert_eq!(lines[4], "That text is too long (at most 128 characters).");
    fx.teardown().await;
}
