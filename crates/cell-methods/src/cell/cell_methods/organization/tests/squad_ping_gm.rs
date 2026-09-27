//! ORG-04: CM 10 `BroadcastMinimapPing` for squads (validated and logged,
//! never fanned out) and the GM console's squad commands.

use std::time::{Duration, Instant};

use cimmeria_entity::organization::{OrgType, SQUAD_ORG_ID_MIN};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_joined, ON_MEMBER_JOINED_ORGANIZATION, ON_ORGANIZATION_INVITE,
    ON_ORGANIZATION_JOINED, ON_ORGANIZATION_ROSTER_INFO, ON_SQUAD_LOOT_TYPE,
};

use super::*;
use crate::test_support::{Captured, LogCapture};

const SID: i32 = SQUAD_ORG_ID_MIN;

/// CM 10 args: the org id, then a finite location.
fn ping_args(org_id: i32) -> Vec<u8> {
    let mut args = org_id.to_le_bytes().to_vec();
    for c in [1.0f32, 2.0, 3.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    args
}

/// The single `squad.ping` outcome row.
fn ping_row(capture: &LogCaptureGuard) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "squad" && c.has_field("event", "squad.ping"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    rows.into_iter().next().unwrap()
}

/// The single `org.gm_action` row.
fn gm_row(capture: &LogCaptureGuard) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", "org.gm_action"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    rows.into_iter().next().unwrap()
}

fn assert_fields(row: &Captured, want: &[(&str, &str)]) {
    for (k, v) in want {
        assert!(row.has_field(k, v), "{k}={v}: {:?}", row.fields);
    }
}

/// A member's ping through the router is accepted, logged with
/// `recipients = 0`, and sends nothing to anyone: no client method shows
/// another member's ping (ORG-E1 Q3).
#[tokio::test]
async fn member_ping_is_logged_and_not_fanned_out() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let (tx, mut rx) = channel();

    assert!(dispatch(11, BROADCAST_MINIMAP_PING, &ping_args(sid), &tx, &mut mgr).await);

    assert!(drain(&mut rx).is_empty());
    let row = ping_row(&capture);
    assert_eq!(row.level, Level::INFO);
    assert_fields(
        &row,
        &[
            ("outcome", "ok"),
            ("player_id", "1"),
            ("account_id", &account_of(1).to_string()),
            ("squad_id", &sid.to_string()),
            ("recipients", "0"),
        ],
    );
}

/// A ping from a player in no squad is refused with feedback.
#[tokio::test]
async fn ping_outside_a_squad_is_refused() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();

    squad::ping_at(11, SID, [0.0; 3], &tx, &mut mgr, Instant::now()).await;

    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "You are not in a squad, so nobody sees your ping.")
    );
    assert_fields(
        &ping_row(&capture),
        &[("outcome", "rejected"), ("reason", "not_in_squad")],
    );
}

/// A ping naming another squad is refused, whatever the caller's own.
#[tokio::test]
async fn ping_naming_another_squad_is_refused() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara", "Dan"]);
    seed_squad(&mut mgr, 11, &[12]);
    let other = seed_squad(&mut mgr, 13, &[14]);
    let (tx, mut rx) = channel();

    squad::ping_at(11, other, [0.0; 3], &tx, &mut mgr, Instant::now()).await;

    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(other, "You are not in that squad.")
    );
    assert_fields(
        &ping_row(&capture),
        &[("outcome", "rejected"), ("reason", "wrong_squad")],
    );
    assert!(to(&drain(&mut rx), 13).is_empty());
}

/// One ping per second per member: the second inside the second is
/// refused silently (the client drew it locally already), the next one at
/// the second is accepted.
#[tokio::test]
async fn ping_rate_limit_is_one_per_second() {
    let mut mgr = world(&["Alice", "Bob"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let (tx, mut rx) = channel();
    let t0 = Instant::now();

    let capture = LogCapture::install();
    squad::ping_at(11, sid, [0.0; 3], &tx, &mut mgr, t0).await;
    assert_fields(&ping_row(&capture), &[("outcome", "ok")]);
    drop(capture);

    let capture = LogCapture::install();
    let early = t0 + Duration::from_millis(500);
    squad::ping_at(11, sid, [0.0; 3], &tx, &mut mgr, early).await;
    assert_fields(
        &ping_row(&capture),
        &[("outcome", "rejected"), ("reason", "rate_limited")],
    );
    drop(capture);

    let capture = LogCapture::install();
    squad::ping_at(
        11,
        sid,
        [0.0; 3],
        &tx,
        &mut mgr,
        t0 + Duration::from_secs(1),
    )
    .await;
    assert_fields(&ping_row(&capture), &[("outcome", "ok")]);
    assert!(
        drain(&mut rx).is_empty(),
        "no ping is ever sent to a client"
    );
}

/// A Team or Command id keeps ORG-01's answer until ORG-07/09.
#[tokio::test]
async fn ping_with_a_base_org_id_is_not_a_squad_ping() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    assert!(dispatch(11, BROADCAST_MINIMAP_PING, &ping_args(5), &tx, &mut mgr).await);
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(5, "Organizations are not available yet.")
    );
    assert!(capture
        .all()
        .iter()
        .all(|c| !c.has_field("event", "squad.ping")));
}

/// `.squad_join` on a squadless player founds a squad they lead, with the
/// GM as a member; both get the whole squad, the GM a confirmation line.
#[tokio::test]
async fn gm_join_founds_a_squad_with_the_target_leading() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Gm", "Sentinel"]);
    let (tx, mut rx) = channel();

    squad::gm_join(11, "Sentinel", &tx, &mut mgr).await;

    let squad = mgr.squads.squad_for(1).expect("GM is in a squad").clone();
    assert_eq!(squad.leader_player_id(), 2);
    assert_eq!(mgr.get_entity(11).unwrap().squad_id, Some(squad.id()));
    assert_eq!(mgr.get_entity(12).unwrap().squad_id, Some(squad.id()));

    let sent = drain(&mut rx);
    let gm: Vec<u16> = to(&sent, 11).into_iter().map(|m| m.0).collect();
    assert_eq!(
        gm,
        [
            ON_ORGANIZATION_JOINED,
            ON_ORGANIZATION_ROSTER_INFO,
            ON_MEMBER_JOINED_ORGANIZATION,
            ON_SQUAD_LOOT_TYPE,
            28
        ]
    );
    assert_eq!(
        to(&sent, 11)[0].1,
        build_on_organization_joined(squad.id(), OrgType::Squad, squad.rank_of(1), true)
    );
    assert_eq!(to(&sent, 11)[4].1, line("You joined Sentinel's squad."));
    assert_eq!(to(&sent, 12)[0].0, ON_ORGANIZATION_JOINED);

    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_join"),
            ("outcome", "ok"),
            ("player_id", "1"),
            ("account_id", &account_of(1).to_string()),
            ("target_player_id", "2"),
            ("target_account_id", &account_of(2).to_string()),
            ("squad_id", &squad.id().to_string()),
        ],
    );
}

/// `.squad_join` on a plain member (not the leader) still brings the GM
/// in: no handshake, no leader check. The existing members get one [37].
#[tokio::test]
async fn gm_join_enters_an_existing_squad_through_any_member() {
    let mut mgr = world(&["Alice", "Bob", "Gm"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let (tx, mut rx) = channel();

    squad::gm_join(13, "Bob", &tx, &mut mgr).await;

    assert_eq!(mgr.squads.squad_of(3), Some(sid));
    let sent = drain(&mut rx);
    for e in [11, 12] {
        let got: Vec<u16> = to(&sent, e).into_iter().map(|m| m.0).collect();
        assert_eq!(got, [ON_MEMBER_JOINED_ORGANIZATION], "entity {e}");
    }
}

/// The GM path keeps the membership rules: a GM already in a squad is
/// refused, and nothing changes.
#[tokio::test]
async fn gm_join_refuses_a_gm_already_in_a_squad() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Gm", "Dan"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let own = seed_squad(&mut mgr, 13, &[14]);
    let (tx, mut rx) = channel();

    squad::gm_join(13, "Alice", &tx, &mut mgr).await;

    assert_eq!(mgr.squads.squad_of(3), Some(own));
    assert_eq!(mgr.squads.squad(sid).unwrap().members().len(), 2);
    assert_eq!(
        to(&drain(&mut rx), 13),
        rejection(sid, "You are already in a squad. Leave it first.")
    );
    assert_fields(
        &gm_row(&capture),
        &[("outcome", "rejected"), ("reason", "already_in_squad")],
    );
}

/// An unknown name is refused before anything else.
#[tokio::test]
async fn gm_join_refuses_an_unknown_name() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Gm"]);
    let (tx, mut rx) = channel();
    squad::gm_join(11, "Nobody", &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "No player named Nobody is online.")
    );
    assert_eq!(mgr.squads.squad_count(), 0);
    assert_fields(
        &gm_row(&capture),
        &[("outcome", "rejected"), ("reason", "target_not_found")],
    );
}

/// `.squad_invite` sends the target the ordinary invite window, and the
/// GM row names the target.
#[tokio::test]
async fn gm_invite_issues_a_real_invite() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Gm", "Bob"]);
    let (tx, mut rx) = channel();

    squad::gm_invite(11, "Bob", &tx, &mut mgr).await;

    assert_eq!(mgr.squads.pending_for(2, Instant::now()), 1);
    let sent = drain(&mut rx);
    assert_eq!(to(&sent, 12)[0].0, ON_ORGANIZATION_INVITE);
    assert_eq!(
        to(&sent, 11),
        vec![(28, line("You invited Bob to your squad."))]
    );
    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_invite"),
            ("outcome", "ok"),
            ("target_player_id", "2"),
        ],
    );
}

/// A refused invite carries the invite handler's reason onto the GM row.
#[tokio::test]
async fn gm_invite_reports_the_invite_refusal() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Gm", "Bob", "Cara"]);
    seed_squad(&mut mgr, 12, &[13]);
    let (tx, _rx) = channel();

    squad::gm_invite(11, "Bob", &tx, &mut mgr).await;

    assert_fields(
        &gm_row(&capture),
        &[("outcome", "rejected"), ("reason", "already_in_squad")],
    );
}

/// `.squad_info` lists the squad: a header, then each member in join order
/// with rank and whereabouts.
#[tokio::test]
async fn gm_info_lists_the_squad() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Gm"]);
    let sid = seed_squad(&mut mgr, 11, &[12]);
    let (tx, mut rx) = channel();

    squad::gm_info(13, Some("Bob"), &tx, &mut mgr).await;

    let want = vec![
        (
            28,
            line(&format!("Squad {sid}: 2 members, loot round robin.")),
        ),
        (28, line("  Alice (leader, level 12, player 1): entity 11")),
        (28, line("  Bob (member, level 12, player 2): entity 12")),
    ];
    assert_eq!(to(&drain(&mut rx), 13), want);
    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_info"),
            ("outcome", "ok"),
            ("target_player_id", "2"),
            ("squad_id", &sid.to_string()),
        ],
    );

    squad::gm_info(13, None, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 13),
        vec![(28, line("Gm is not in a squad."))]
    );
}
