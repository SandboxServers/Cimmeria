//! Leaving a squad: CM 9, the leader's kick, disconnects, promotion and
//! disband.

use cimmeria_cell_world::cell::squad::entity_squad_id;
use cimmeria_cell_world::cell::squad::SquadResources;
use cimmeria_entity::organization::{OrgLeaveReason, OrgRank, SQUAD_ORG_ID_MIN};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_left_organization, build_on_member_rank_changed_organization,
    build_on_organization_left,
};

use super::*;
use crate::test_support::LogCapture;

const SID: i32 = SQUAD_ORG_ID_MIN;

fn left(reason: OrgLeaveReason) -> (u16, Vec<u8>) {
    (36, build_on_organization_left(reason, SID))
}

fn member_left(id: i32, reason: OrgLeaveReason, name: &str) -> (u16, Vec<u8>) {
    (39, build_on_member_left_organization(id, reason, SID, name))
}

fn new_leader(id: i32, name: &str) -> (u16, Vec<u8>) {
    (
        40,
        build_on_member_rank_changed_organization(id, OrgRank::LEADER, SID, name),
    )
}

/// Take entity `e` out of every space's player set: the gate-transit state
/// in which the player resolves to no live entity.
fn put_in_transit(mgr: &mut SpaceManager, e: u32) {
    for space in mgr.spaces.values_mut() {
        space.players.remove(&e);
    }
}

#[tokio::test]
async fn member_leave_sends_36_to_the_leaver_and_39_to_the_rest() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::leave(13, SID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(to(&sent, 13), [left(OrgLeaveReason::Requested)]);
    let bye = member_left(13, OrgLeaveReason::Requested, "Cara");
    assert_eq!(to(&sent, 11), std::slice::from_ref(&bye));
    assert_eq!(to(&sent, 12), [bye]);
    assert_eq!(sent.len(), 3);
    assert_eq!(entity_squad_id(mgr.get_entity(13).unwrap()), None);
    assert_eq!(mgr.resources.squads().squad_of(3), None);
}

/// The leader leaving promotes the longest-standing member (D-ORG12): the
/// rest get [39] then [40] naming the new leader.
#[tokio::test]
async fn leader_leave_promotes_the_longest_standing_member() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::leave(11, SID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let expect = [
        member_left(11, OrgLeaveReason::Requested, "Alice"),
        new_leader(12, "Bob"),
    ];
    assert_eq!(to(&sent, 12), expect);
    assert_eq!(to(&sent, 13), expect);
    assert_eq!(to(&sent, 11), [left(OrgLeaveReason::Requested)]);
    assert_eq!(
        mgr.resources
            .squads()
            .squad(SID)
            .unwrap()
            .leader_player_id(),
        2
    );
}

/// A leader leaving a pair: no promotion; the one left gets [39] for the
/// leaver, then [36] `Disbanded`.
#[tokio::test]
async fn leaving_a_pair_disbands_it() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::leave(11, SID, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(
        to(&sent, 12),
        [
            member_left(11, OrgLeaveReason::Requested, "Alice"),
            left(OrgLeaveReason::Disbanded)
        ]
    );
    assert_eq!(to(&sent, 11), [left(OrgLeaveReason::Requested)]);
    assert_eq!(mgr.resources.squads().squad_count(), 0);
    assert_eq!(entity_squad_id(mgr.get_entity(12).unwrap()), None);
}

/// CAT-M-04: leave names a squad the caller is not in. Refused, logged at
/// WARN, and neither squad changes.
#[tokio::test]
async fn squad_leave_rejects_foreign_squad_id() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara", "Dan"]);
    let (tx, mut rx) = channel();
    let mine = seed_squad(&mut mgr, 11, &[12]);
    let theirs = seed_squad(&mut mgr, 13, &[14]);
    assert_ne!(mine, theirs);
    squad::leave(12, theirs, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 2);
    assert_eq!(
        to(&sent, 12),
        rejection(theirs, "You are not in that squad.")
    );
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.leave",
        "wrong_squad"
    ));
    assert_eq!(
        mgr.resources.squads().squad(mine).unwrap().members().len(),
        2
    );
    assert_eq!(
        mgr.resources
            .squads()
            .squad(theirs)
            .unwrap()
            .members()
            .len(),
        2
    );
    assert_eq!(mgr.resources.squads().squad_of(2), Some(mine));
}

#[tokio::test]
async fn leader_kick_removes_the_member_with_kicked() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::handle_kick(1, 11, SID, "Bob", &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(to(&sent, 12), [left(OrgLeaveReason::Kicked)]);
    let bye = member_left(12, OrgLeaveReason::Kicked, "Bob");
    assert_eq!(to(&sent, 11), std::slice::from_ref(&bye));
    assert_eq!(to(&sent, 13), [bye]);
    assert_eq!(mgr.resources.squads().squad_of(2), None);
}

/// Every kick refusal: feedback to the actor, membership unchanged.
#[tokio::test]
async fn kick_refusals_change_nothing() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara", "Dan"]);
    let (tx, mut rx) = channel();
    let sid = seed_squad(&mut mgr, 11, &[12, 13]);
    let cases = [
        // Bob is not the leader.
        (
            2,
            12,
            sid,
            "Cara",
            "Only the squad leader can remove members.",
            "not_leader",
            Level::INFO,
        ),
        (
            1,
            11,
            sid,
            "Dan",
            "Dan is not in your squad.",
            "target_not_in_squad",
            Level::INFO,
        ),
        (
            1,
            11,
            sid,
            "Alice",
            "Leave the squad instead of removing yourself.",
            "self_target",
            Level::INFO,
        ),
        // Dan names Alice's squad.
        (
            4,
            14,
            sid,
            "Bob",
            "You are not in that squad.",
            "not_in_squad",
            Level::INFO,
        ),
    ];
    for (pid, e, org, target, text, reason, level) in cases {
        squad::handle_kick(pid, e, org, target, &tx, &mut mgr).await;
        assert_eq!(to(&drain(&mut rx), e), rejection(org, text), "{reason}");
        assert!(
            squad_event(&capture, level, "squad.kick", reason),
            "{reason}"
        );
    }
    assert_eq!(
        mgr.resources.squads().squad(sid).unwrap().members().len(),
        3
    );
}

/// A forwarded call whose entity is no longer that character is dropped.
#[tokio::test]
async fn forwarded_kick_with_a_stale_entity_is_dropped() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    // Base says character 9 is on entity 11; the cell knows 11 is Alice (1).
    squad::handle_kick(9, 11, SID, "Bob", &tx, &mut mgr).await;
    assert!(drain(&mut rx).is_empty());
    assert_eq!(
        mgr.resources.squads().squad(SID).unwrap().members().len(),
        2
    );
}

/// The leader of three disconnects: the rest get [39] `Logout` and the
/// promotion; nothing is sent to the departing client.
#[tokio::test]
async fn leader_disconnect_promotes_and_says_logout() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::on_disconnect(11, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let expect = [
        member_left(11, OrgLeaveReason::Logout, "Alice"),
        new_leader(12, "Bob"),
    ];
    assert_eq!(to(&sent, 12), expect);
    assert_eq!(to(&sent, 13), expect);
    assert!(to(&sent, 11).is_empty());
    assert_eq!(mgr.resources.squads().squad_of(1), None);
}

#[tokio::test]
async fn disconnect_from_a_pair_disbands_it() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::on_disconnect(12, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(
        to(&sent, 11),
        [
            member_left(12, OrgLeaveReason::Logout, "Bob"),
            left(OrgLeaveReason::Disbanded)
        ]
    );
    assert_eq!(mgr.resources.squads().squad_count(), 0);
}

/// A member kicked while in gate transit cannot be told then; the [36] is
/// queued and delivered on their world entry, and the others see [39] with
/// member id 0 (no live entity).
#[tokio::test]
async fn kick_during_transit_is_delivered_on_world_entry() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    put_in_transit(&mut mgr, 12);
    squad::handle_kick(1, 11, SID, "Bob", &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert!(to(&sent, 12).is_empty());
    assert_eq!(
        to(&sent, 13),
        [member_left(0, OrgLeaveReason::Kicked, "Bob")]
    );
    mgr.connect_entity(12);
    squad::on_world_entry(12, 2, &tx, &mut mgr).await;
    assert_eq!(to(&drain(&mut rx), 12), [left(OrgLeaveReason::Kicked)]);
}
