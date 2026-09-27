//! `SquadRegistry` state-machine tests (TESTING.md type 1). Time is
//! injected, so every expiry and rate-window edge is exact.

use std::time::{Duration, Instant};

use cimmeria_entity::organization::{
    OrgLeaveReason, OrgRank, SquadLootType, BASE_INVITE_REQUEST_FLAG, MAX_SQUAD_SIZE,
    SQUAD_ORG_ID_MAX, SQUAD_ORG_ID_MIN,
};

use super::*;

fn m(player_id: i32) -> SquadMember {
    SquadMember {
        player_id,
        name: format!("P{player_id}"),
        level: 10,
        archetype: 1,
    }
}

/// Invite `invitee` from `inviter` and accept it at `now`.
fn join(reg: &mut SquadRegistry, inviter: i32, invitee: i32, now: Instant) -> JoinOutcome {
    let issued = reg
        .invite(inviter, &format!("P{inviter}"), invitee, now)
        .expect("invite");
    let inv = reg
        .take_invite(invitee, issued.request_id, now)
        .expect("pending");
    reg.accept(&inv, m(invitee), Some(m(inviter)))
        .expect("accept")
}

/// A squad led by 1 with members 1..=n, in that join order.
fn squad_of(n: i32, now: Instant) -> (SquadRegistry, i32) {
    let mut reg = SquadRegistry::new();
    let sid = join(&mut reg, 1, 2, now).squad_id;
    for p in 3..=n {
        join(&mut reg, 1, p, now);
    }
    (reg, sid)
}

fn member_ids(reg: &SquadRegistry, sid: i32) -> Vec<i32> {
    reg.squad(sid)
        .unwrap()
        .members()
        .iter()
        .map(|m| m.player_id)
        .collect()
}

#[test]
fn first_accept_creates_the_squad_with_the_inviter_leading() {
    let now = Instant::now();
    let mut reg = SquadRegistry::new();
    let issued = reg.invite(1, "P1", 2, now).unwrap();
    assert_eq!(issued.squad_id, None, "no squad until someone accepts");
    assert_eq!(reg.squad_count(), 0);
    let inv = reg.take_invite(2, issued.request_id, now).unwrap();
    let out = reg.accept(&inv, m(2), Some(m(1))).unwrap();
    assert_eq!(
        out,
        JoinOutcome {
            squad_id: SQUAD_ORG_ID_MIN,
            created: true
        }
    );
    let squad = reg.squad(out.squad_id).unwrap();
    assert_eq!(squad.leader_player_id(), 1);
    assert_eq!(squad.rank_of(1), OrgRank::LEADER);
    assert_eq!(squad.rank_of(2), OrgRank::MEMBER);
    assert_eq!(squad.loot(), SquadLootType::RoundRobin);
    assert_eq!(member_ids(&reg, out.squad_id), [1, 2]);
    assert_eq!(reg.squad_of(1), Some(out.squad_id));
    assert_eq!(reg.squad_of(2), Some(out.squad_id));
}

/// A squadless inviter with two pending invites: the second accept joins
/// the squad the first one created instead of founding another.
#[test]
fn second_accept_joins_the_squad_the_first_created() {
    let now = Instant::now();
    let mut reg = SquadRegistry::new();
    let a = reg.invite(1, "P1", 2, now).unwrap();
    let b = reg.invite(1, "P1", 3, now).unwrap();
    let inv_a = reg.take_invite(2, a.request_id, now).unwrap();
    let first = reg.accept(&inv_a, m(2), Some(m(1))).unwrap();
    let inv_b = reg.take_invite(3, b.request_id, now).unwrap();
    let second = reg.accept(&inv_b, m(3), Some(m(1))).unwrap();
    assert_eq!(second.squad_id, first.squad_id);
    assert!(!second.created);
    assert_eq!(reg.squad_count(), 1);
    assert_eq!(member_ids(&reg, first.squad_id), [1, 2, 3]);
}

/// The creation path needs the inviter online; a squadless inviter who
/// logged off cannot found a squad.
#[test]
fn accept_rejects_offline_squadless_inviter() {
    let now = Instant::now();
    let mut reg = SquadRegistry::new();
    let a = reg.invite(1, "P1", 2, now).unwrap();
    let inv = reg.take_invite(2, a.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv, m(2), None),
        Err(ResponseReject::InviterOffline)
    );
    assert_eq!(reg.squad_count(), 0);
}

#[test]
fn accept_revalidates_the_inviter_and_the_squad() {
    let now = Instant::now();
    // Inviter left the squad the invite named (it lives on with 2 and 3).
    let (mut reg, sid) = squad_of(3, now);
    let i = reg.invite(1, "P1", 9, now).unwrap();
    reg.leave(1).unwrap();
    let inv = reg.take_invite(9, i.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv, m(9), Some(m(1))),
        Err(ResponseReject::InviterLeft)
    );
    assert_eq!(member_ids(&reg, sid), [2, 3]);

    // The squad the invite named disbanded.
    let (mut reg, _) = squad_of(2, now);
    let i = reg.invite(1, "P1", 9, now).unwrap();
    // Disband drops the invite eagerly; rebuild the entry to test accept.
    let inv = PendingInvite {
        inviter_player_id: 1,
        inviter_name: "P1".into(),
        squad_id: Some(SQUAD_ORG_ID_MIN),
        expires_at: now + INVITE_TTL,
    };
    reg.leave(2).unwrap();
    assert!(reg.take_invite(9, i.request_id, now).is_err());
    assert_eq!(
        reg.accept(&inv, m(9), Some(m(1))),
        Err(ResponseReject::SquadGone)
    );

    // Inviter was demoted: the leader left and 2 now leads.
    let (mut reg, sid) = squad_of(3, now);
    let i = reg.invite(1, "P1", 9, now).unwrap();
    reg.leave(1).unwrap();
    join(&mut reg, 2, 1, now);
    let inv = reg.take_invite(9, i.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv, m(9), Some(m(1))),
        Err(ResponseReject::InviterNotLeader)
    );
    assert_eq!(member_ids(&reg, sid), [2, 3, 1]);
}

/// Two accepts into a five-member squad leave six, never seven.
#[test]
fn accept_rejects_the_seventh_member() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(5, now);
    // Past the rate window the four founding invites opened.
    let now = now + INVITE_RATE_WINDOW;
    let a = reg.invite(1, "P1", 6, now).unwrap();
    let b = reg.invite(1, "P1", 7, now).unwrap();
    let inv_a = reg.take_invite(6, a.request_id, now).unwrap();
    reg.accept(&inv_a, m(6), Some(m(1))).unwrap();
    let inv_b = reg.take_invite(7, b.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv_b, m(7), Some(m(1))),
        Err(ResponseReject::SquadFull)
    );
    assert_eq!(reg.squad(sid).unwrap().members().len(), MAX_SQUAD_SIZE);
    assert_eq!(reg.squad_of(7), None);
}

#[test]
fn accept_rejects_an_invitee_who_joined_elsewhere() {
    let now = Instant::now();
    let mut reg = SquadRegistry::new();
    let a = reg.invite(1, "P1", 3, now).unwrap();
    join(&mut reg, 2, 3, now);
    let inv = reg.take_invite(3, a.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv, m(3), Some(m(1))),
        Err(ResponseReject::InviteeInSquad)
    );
}

#[test]
fn invite_rejects_each_limit() {
    let now = Instant::now();
    // Target already squadded.
    let (mut reg, _) = squad_of(2, now);
    assert_eq!(
        reg.invite(5, "P5", 2, now),
        Err(InviteReject::TargetInSquad)
    );
    // Inviter is a non-leader member.
    assert_eq!(
        reg.invite(2, "P2", 5, now),
        Err(InviteReject::InviterNotLeader)
    );
    // Squad full.
    let (mut reg, _) = squad_of(6, now);
    assert_eq!(reg.invite(1, "P1", 9, now), Err(InviteReject::SquadFull));
    // One pending per pair.
    let mut reg = SquadRegistry::new();
    reg.invite(1, "P1", 2, now).unwrap();
    assert_eq!(
        reg.invite(1, "P1", 2, now),
        Err(InviteReject::DuplicatePending)
    );
    // At most five pending per invitee.
    let mut reg = SquadRegistry::new();
    for inviter in 10..15 {
        reg.invite(inviter, "x", 2, now).unwrap();
    }
    assert_eq!(
        reg.invite(15, "x", 2, now),
        Err(InviteReject::InviteeInboxFull)
    );
}

/// Five invites per inviter per 30 s, then allowed again once the oldest
/// slides out of the window.
#[test]
fn invite_rate_limit_slides() {
    let t0 = Instant::now();
    let mut reg = SquadRegistry::new();
    for (i, invitee) in (20..25).enumerate() {
        reg.invite(1, "P1", invitee, t0 + Duration::from_secs(i as u64))
            .unwrap();
    }
    let just_before = t0 + INVITE_RATE_WINDOW - Duration::from_millis(1);
    assert_eq!(
        reg.invite(1, "P1", 30, just_before),
        Err(InviteReject::RateLimited)
    );
    // The first send (t0) is now 30 s old and leaves the window.
    reg.invite(1, "P1", 30, t0 + INVITE_RATE_WINDOW).unwrap();
    // Another inviter is not affected.
    reg.invite(2, "P2", 31, just_before).unwrap();
}

/// Answerable until just before 60 s; at 60 s the miss says `Expired`,
/// and the expiry is recorded once for the transition log.
#[test]
fn invite_expires_at_sixty_seconds() {
    let t0 = Instant::now();
    let mut reg = SquadRegistry::new();
    let a = reg.invite(1, "P1", 2, t0).unwrap();
    let b = reg.invite(3, "P3", 2, t0).unwrap();
    let edge = t0 + INVITE_TTL - Duration::from_millis(1);
    assert!(reg.take_invite(2, a.request_id, edge).is_ok());
    assert_eq!(
        reg.take_invite(2, b.request_id, t0 + INVITE_TTL),
        Err(TakeMiss::Expired)
    );
    assert_eq!(
        reg.drain_expired(),
        [ExpiredInvite {
            invitee_player_id: 2,
            request_id: b.request_id,
            inviter_player_id: 3,
            squad_id: None,
        }]
    );
    assert!(reg.drain_expired().is_empty());
    assert_eq!(
        reg.take_invite(2, b.request_id, t0 + INVITE_TTL),
        Err(TakeMiss::Unknown)
    );
}

/// Expired invites stop counting toward the invitee cap.
#[test]
fn expired_invites_free_the_inbox() {
    let t0 = Instant::now();
    let mut reg = SquadRegistry::new();
    for inviter in 10..15 {
        reg.invite(inviter, "x", 2, t0).unwrap();
    }
    assert_eq!(reg.pending_for(2, t0), 5);
    let later = t0 + INVITE_TTL;
    assert_eq!(reg.pending_for(2, later), 0);
    reg.invite(15, "x", 2, later).unwrap();
}

/// Consumed on the first response, whatever it was, and a request id is
/// found only under the invitee it was issued to.
#[test]
fn invites_are_single_use_and_keyed_by_invitee() {
    let now = Instant::now();
    let mut reg = SquadRegistry::new();
    let a = reg.invite(1, "P1", 2, now).unwrap();
    let c = reg.invite(1, "P1", 3, now).unwrap();
    // 3 answers 2's request id: a foreign miss, and 2's invite survives.
    assert_eq!(
        reg.take_invite(3, a.request_id, now),
        Err(TakeMiss::Foreign)
    );
    assert_eq!(reg.pending_for(2, now), 1);
    // Decline (take and drop) consumes; a replay finds nothing.
    assert!(reg.take_invite(2, a.request_id, now).is_ok());
    assert_eq!(
        reg.take_invite(2, a.request_id, now),
        Err(TakeMiss::Unknown)
    );
    // 3's own invite is untouched.
    assert!(reg.take_invite(3, c.request_id, now).is_ok());
}

#[test]
fn request_ids_never_repeat_and_never_reach_the_base_flag() {
    let now = Instant::now();
    let mut reg = SquadRegistry::with_next_ids(SQUAD_ORG_ID_MIN, BASE_INVITE_REQUEST_FLAG - 2);
    let a = reg.invite(1, "P1", 2, now).unwrap();
    let b = reg.invite(1, "P1", 3, now).unwrap();
    assert_eq!(a.request_id, BASE_INVITE_REQUEST_FLAG - 2);
    assert_eq!(b.request_id, BASE_INVITE_REQUEST_FLAG - 1);
    assert_eq!(
        reg.invite(1, "P1", 4, now),
        Err(InviteReject::RequestIdsExhausted)
    );
}

/// Squad ids come from a counter and are not reused after a disband; the
/// last id is usable and then creation is refused.
#[test]
fn squad_ids_are_never_reused() {
    let now = Instant::now();
    let mut reg = SquadRegistry::with_next_ids(SQUAD_ORG_ID_MAX - 1, 1);
    let first = join(&mut reg, 1, 2, now).squad_id;
    reg.leave(2).unwrap();
    let second = join(&mut reg, 1, 2, now).squad_id;
    assert_eq!((first, second), (SQUAD_ORG_ID_MAX - 1, SQUAD_ORG_ID_MAX));
    reg.leave(2).unwrap();
    let i = reg.invite(1, "P1", 2, now).unwrap();
    let inv = reg.take_invite(2, i.request_id, now).unwrap();
    assert_eq!(
        reg.accept(&inv, m(2), Some(m(1))),
        Err(ResponseReject::SquadIdsExhausted)
    );
}

/// The leader leaving promotes the longest-standing member (D-ORG12).
#[test]
fn leader_leave_promotes_the_longest_standing_member() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(4, now);
    let d = reg.leave(1).unwrap();
    assert_eq!(d.reason, OrgLeaveReason::Requested);
    assert_eq!(d.new_leader, Some(2));
    assert!(!d.disbanded);
    assert_eq!(member_ids(&reg, sid), [2, 3, 4]);
    assert_eq!(reg.squad(sid).unwrap().leader_player_id(), 2);
    // A non-leader leaving promotes nobody.
    let d = reg.leave(4).unwrap();
    assert_eq!(d.new_leader, None);
}

/// A leader leaving a squad of two disbands it and promotes nobody.
#[test]
fn leader_leaving_a_pair_disbands_without_promotion() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(2, now);
    let d = reg.leave(1).unwrap();
    assert!(d.disbanded);
    assert_eq!(d.new_leader, None);
    assert_eq!(d.remaining, [m(2)]);
    assert!(reg.squad(sid).is_none());
    assert_eq!(reg.squad_of(2), None);
    assert!(reg.leave(2).is_none(), "the survivor is squadless");
}

#[test]
fn kick_rules() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(3, now);
    assert_eq!(reg.kick(2, sid, "P3"), Err(KickReject::NotLeader));
    assert_eq!(reg.kick(1, sid + 1, "P3"), Err(KickReject::NotInThatSquad));
    assert_eq!(reg.kick(9, sid, "P3"), Err(KickReject::NotInThatSquad));
    assert_eq!(
        reg.kick(1, sid, "Nobody"),
        Err(KickReject::TargetNotInSquad)
    );
    assert_eq!(reg.kick(1, sid, "P1"), Err(KickReject::SelfKick));
    assert_eq!(member_ids(&reg, sid), [1, 2, 3]);
    let d = reg.kick(1, sid, "P3").unwrap();
    assert_eq!(d.reason, OrgLeaveReason::Kicked);
    assert_eq!(d.departed.player_id, 3);
    let d = reg.kick(1, sid, "P2").unwrap();
    assert!(d.disbanded, "the leader alone dissolves the squad");
}

#[test]
fn loot_mode_is_leader_only_and_range_checked() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(2, now);
    assert_eq!(reg.set_loot(9, 1), Err(LootReject::NotInSquad));
    assert_eq!(
        reg.set_loot(2, 1),
        Err(LootReject::NotLeader {
            squad_id: sid,
            current: SquadLootType::RoundRobin
        })
    );
    for bad in [2, -1, i32::MAX] {
        assert_eq!(
            reg.set_loot(1, bad),
            Err(LootReject::OutOfRange {
                squad_id: sid,
                current: SquadLootType::RoundRobin
            })
        );
    }
    assert_eq!(reg.set_loot(1, 1), Ok(sid));
    assert_eq!(reg.squad(sid).unwrap().loot(), SquadLootType::FreeForAll);
}

/// Going offline removes the member with `Logout` and drops every invite
/// they sent or hold.
#[test]
fn remove_player_drops_membership_and_both_sides_of_invites() {
    let now = Instant::now();
    let (mut reg, sid) = squad_of(3, now);
    let sent = reg.invite(1, "P1", 7, now).unwrap();
    let kept = reg.invite(1, "P1", 8, now).unwrap();
    reg.owe_left(1, sid, OrgLeaveReason::Kicked);
    let d = reg.remove_player(1).unwrap();
    assert_eq!(d.reason, OrgLeaveReason::Logout);
    assert_eq!(d.new_leader, Some(2));
    assert!(reg.take_invite(7, sent.request_id, now).is_err());
    assert!(reg.take_invite(8, kept.request_id, now).is_err());
    assert_eq!(reg.take_owed_left(1), None);
    // A squadless player's invites go too, sent and held; nobody else's.
    let sent = reg.invite(9, "P9", 10, now).unwrap();
    let held = reg.invite(11, "P11", 9, now).unwrap();
    let other = reg.invite(11, "P11", 12, now).unwrap();
    assert!(reg.remove_player(9).is_none());
    assert!(reg.take_invite(10, sent.request_id, now).is_err());
    assert!(reg.take_invite(9, held.request_id, now).is_err());
    assert!(reg.take_invite(12, other.request_id, now).is_ok());
}

#[test]
fn owed_left_is_delivered_once() {
    let mut reg = SquadRegistry::new();
    reg.owe_left(5, SQUAD_ORG_ID_MIN, OrgLeaveReason::Kicked);
    assert_eq!(
        reg.take_owed_left(5),
        Some((SQUAD_ORG_ID_MIN, OrgLeaveReason::Kicked))
    );
    assert_eq!(reg.take_owed_left(5), None);
}
