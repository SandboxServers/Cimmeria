//! Team, Command and officer chat (ORG-09): who gets the line, byte for
//! byte, and the refusals with their feedback and `org.chat` rows.
//!
//! Live-DB (type 3) for the membership read, fanout byte tests (type 8)
//! against `TestTransport`, and `LogCapture` (type 12) for every refusal and
//! the speaker-copy seam. ORG-09's sentinel block, `0x7000_5600`.

use cimmeria_wire::cell::chat::{
    serialize_on_player_communication, CHAN_COMMAND, CHAN_FEEDBACK, CHAN_OFFICER, CHAN_TEAM,
};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::mercury::method_idx::ON_CHAT_JOINED;
use tracing::Level;

use super::org07_support::one_row;
use super::*;
use crate::base::organization::handlers::chat::{
    relay_org_chat, ChatSpeaker, NOT_IN_COMMAND_TEXT, NOT_IN_TEAM_TEXT, NO_OFFICER_CHAT_TEXT,
    ORG_CHAT_UNAVAILABLE_TEXT,
};
use crate::test_support::{require_db_or_skip, LogCapture};

impl Fixture {
    fn speaker(&self, i: usize) -> (String, SocketAddr, i32, u32) {
        (
            self.name(i),
            self.addr(i),
            self.player_id(i),
            self.entity(i),
        )
    }

    /// Character `i` says `text` on `channel`.
    async fn chat(&self, i: usize, channel: u8, text: &str) {
        let (name, addr, player_id, entity_id) = self.speaker(i);
        let speaker = ChatSpeaker {
            addr,
            name: &name,
            flags: 0,
            account_id: Some(self.account_id as u32),
            player_id: Some(player_id),
            entity_id: Some(entity_id),
            identity: cimmeria_entity::cell_entity::PlayerIdentity::UNKNOWN,
        };
        relay_org_chat(&self.ctx(), speaker, channel, text).await;
    }

    /// Move character `i` to another world in the database. The base keeps
    /// no space per session, so the fanout must not care.
    async fn move_to_world(&self, i: usize, world: &str) {
        sqlx::query("UPDATE sgw_player SET world_location = $2 WHERE player_id = $1")
            .bind(self.player_id(i))
            .bind(world)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

/// The `onPlayerCommunication` [28] args a member receives.
fn line(speaker: &str, channel: u8, text: &str) -> Call {
    (
        ON_PLAYER_COMMUNICATION,
        serialize_on_player_communication(speaker, 0, channel, text),
    )
}

/// Team (3) and Command (5): every online member, the speaker included and
/// a member in another world included, gets the line byte for byte; an
/// offline member and an online non-member get nothing. One `org.chat ok`
/// row counts the members reached, not the speaker's own copy. Nothing is
/// forwarded to the cell.
#[tokio::test]
async fn live_db_team_and_command_chat_reach_every_online_member() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org09(&pool, 0, 5, &["Org09 Team", "Org09 Command"]).await;
    let team = fx.org(OrgType::Team, "Org09 Team", 0, &[1, 2, 3]).await;
    let cmd = fx.org(OrgType::Command, "Org09 Command", 3, &[0, 1]).await;
    fx.move_to_world(2, "Castle_CellBlock").await;
    for i in [0, 1, 2, 4] {
        fx.online(i); // 3 stays offline; 4 is in neither organization
    }

    for (channel, org_id, members) in [
        (CHAN_TEAM, team, vec![0usize, 1, 2]),
        (CHAN_COMMAND, cmd, vec![0, 1]),
    ] {
        fx.clear_sent();
        let capture = LogCapture::install();
        fx.chat(1, channel, "hello org").await;
        let expected = line(&fx.name(1), channel, "hello org");
        for i in 0..5 {
            let want = if members.contains(&i) {
                vec![expected.clone()]
            } else {
                Vec::new()
            };
            assert_eq!(fx.calls_to(i), want, "channel {channel}, character {i}");
        }
        let row = one_row(&capture, "org.chat", "ok", None);
        assert!(row.has_field("org_id", &org_id.to_string()), "{row:?}");
        assert!(row.has_field("channel", &channel.to_string()), "{row:?}");
        assert!(
            row.has_field("recipients", &(members.len() - 1).to_string()),
            "{row:?}"
        );
        assert!(row.has_field("text_units", "9"), "{row:?}");
        assert!(row.has_field("player_id", &fx.player_id(1).to_string()));
        assert!(!format!("{row:?}").contains("hello org"), "never the text");
    }
    assert!(
        fx.cell_rx.lock().unwrap().try_recv().is_err(),
        "org chat never reaches the cell"
    );
    fx.teardown().await;
}

/// Officer (6): only the Command members whose rank holds `OfficerChat`
/// (Leader and Officer by default, D-ORG08) get the line; an Initiate in
/// the same Command gets nothing, and a Team never hears it. No member is
/// ever sent `onChatJoined`: the client hardcodes 6 (D-ORG14).
#[tokio::test]
async fn live_db_officer_chat_reaches_only_officer_chat_ranks() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org09(&pool, 1, 4, &["Org09 Officers", "Org09 Officers Team"]).await;
    let cmd = fx.org(OrgType::Command, "Org09 Officers", 0, &[1, 2]).await;
    fx.org(OrgType::Team, "Org09 Officers Team", 3, &[1]).await;
    fx.set_rank_of(cmd, 1, OrgRank::OFFICER).await;
    for i in 0..4 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    fx.chat(1, CHAN_OFFICER, "officers only").await;

    let expected = line(&fx.name(1), CHAN_OFFICER, "officers only");
    assert_eq!(fx.calls_to(0), vec![expected.clone()], "the Leader");
    assert_eq!(fx.calls_to(1), vec![expected], "the speaker's own copy");
    assert!(
        fx.calls_to(2).is_empty(),
        "an Initiate holds no OfficerChat"
    );
    assert!(
        fx.calls_to(3).is_empty(),
        "a Team leader, not in the Command"
    );
    for i in 0..4 {
        assert!(
            fx.calls_to(i).iter().all(|c| c.0 != ON_CHAT_JOINED),
            "onChatJoined sent to {i}"
        );
    }
    let row = one_row(&capture, "org.chat", "ok", None);
    assert!(row.has_field("recipients", "1"), "{row:?}");
    assert!(row.has_field("org_type", "command"), "{row:?}");
    fx.teardown().await;
}

/// An officer line from a rank without `OfficerChat` is refused: nobody
/// else hears it, the speaker reads why, and the row says
/// `missing_permission`.
#[tokio::test]
async fn live_db_officer_chat_without_officer_chat_is_refused_with_feedback() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org09(&pool, 2, 3, &["Org09 No Officer"]).await;
    let cmd = fx
        .org(OrgType::Command, "Org09 No Officer", 0, &[1, 2])
        .await;
    for i in 0..3 {
        fx.online(i);
    }
    let capture = LogCapture::install();
    fx.chat(2, CHAN_OFFICER, "let me in").await;

    assert!(fx.calls_to(0).is_empty() && fx.calls_to(1).is_empty());
    let own = fx.calls_to(2);
    assert_eq!(feedback_lines(&own), vec![NO_OFFICER_CHAT_TEXT]);
    assert_eq!(
        own[0].1,
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_OFFICER_CHAT_TEXT)
    );
    let row = one_row(&capture, "org.chat", "rejected", Some("missing_permission"));
    assert!(row.has_field("org_id", &cmd.to_string()), "{row:?}");
    assert!(row.has_field("recipients", "0"), "{row:?}");
    fx.teardown().await;
}

/// A speaker in no organization of the channel's type reads "You are not in
/// a team." / "... command.", and the row says `not_in_org`. Being in the
/// other type does not count: a Command member has no Team.
#[tokio::test]
async fn live_db_speaker_in_no_org_of_that_type_gets_feedback() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org09(&pool, 3, 2, &["Org09 Only Command"]).await;
    fx.org(OrgType::Command, "Org09 Only Command", 0, &[1])
        .await;
    fx.online(0);
    fx.online(1);
    fx.move_to_world(1, "Castle_CellBlock").await;

    let capture = LogCapture::install();
    fx.chat(0, CHAN_TEAM, "anyone?").await;
    assert_eq!(feedback_lines(&fx.calls_to(0)), vec![NOT_IN_TEAM_TEXT]);
    assert!(fx.calls_to(1).is_empty());
    let row = one_row(&capture, "org.chat", "rejected", Some("not_in_org"));
    assert!(row.has_field("org_type", "team"), "{row:?}");

    // A second character with no Command at all.
    let solo = Fixture::org09(&pool, 4, 1, &[]).await;
    solo.online(0);
    let capture = LogCapture::install();
    solo.chat(0, CHAN_COMMAND, "anyone?").await;
    solo.chat(0, CHAN_OFFICER, "anyone?").await;
    assert_eq!(
        feedback_lines(&solo.calls_to(0)),
        vec![NOT_IN_COMMAND_TEXT, NOT_IN_COMMAND_TEXT]
    );
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "org.chat") && c.has_field("reason", "not_in_org"))
        .collect();
    assert_eq!(rows.len(), 2, "{rows:#?}");
    solo.teardown().await;
    fx.teardown().await;
}

/// Negative seams: the speaker's own copy that cannot be sent is WARN
/// `org.send_failed` (`what = chat_echo`) while the members still get the
/// line; with no database the line is refused (`no_db`) with feedback.
#[tokio::test]
async fn live_db_org_chat_seams_warn_with_reason() {
    let pool = require_db_or_skip!();
    let fx = Fixture::org09(&pool, 5, 2, &["Org09 Seams"]).await;
    fx.org(OrgType::Team, "Org09 Seams", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    fx.entity_to_addr.lock().unwrap().remove(&fx.entity(1));
    let capture = LogCapture::install();
    fx.chat(1, CHAN_TEAM, "echo lost").await;
    assert_eq!(
        fx.calls_to(0),
        vec![line(&fx.name(1), CHAN_TEAM, "echo lost")]
    );
    let warn = capture
        .find_event(
            Level::WARN,
            "speaker's own copy could not be sent",
            "entity_to_addr_miss",
        )
        .expect("org.send_failed chat_echo");
    assert!(warn.has_field("what", "chat_echo"), "{warn:?}");
    assert!(warn.has_field("player_id", &fx.player_id(1).to_string()));
    one_row(&capture, "org.chat", "ok", None);

    fx.online(1);
    fx.clear_sent();
    let no_db: Option<Arc<PgPool>> = None;
    let ctx = OrgCtx {
        db_pool: &no_db,
        ..fx.ctx()
    };
    let (name, addr, player_id, entity_id) = fx.speaker(1);
    let capture = LogCapture::install();
    relay_org_chat(
        &ctx,
        ChatSpeaker {
            addr,
            name: &name,
            flags: 0,
            account_id: Some(fx.account_id as u32),
            player_id: Some(player_id),
            entity_id: Some(entity_id),
            identity: cimmeria_entity::cell_entity::PlayerIdentity::UNKNOWN,
        },
        CHAN_TEAM,
        "no db",
    )
    .await;
    assert!(fx.calls_to(0).is_empty());
    assert_eq!(
        feedback_lines(&fx.calls_to(1)),
        vec![ORG_CHAT_UNAVAILABLE_TEXT]
    );
    one_row(&capture, "org.chat", "rejected", Some("no_db"));
    fx.teardown().await;
}
