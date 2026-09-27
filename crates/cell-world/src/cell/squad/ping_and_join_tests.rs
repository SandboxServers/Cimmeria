//! ORG-04 registry tests: the minimap ping check and the GM join
//! (TESTING.md type 1). Time is injected, so the one-second edge is exact.

use std::time::{Duration, Instant};

use cimmeria_entity::organization::{OrgRank, MAX_SQUAD_SIZE, SQUAD_ORG_ID_MIN};

use super::*;

fn m(player_id: i32) -> SquadMember {
    SquadMember {
        player_id,
        name: format!("P{player_id}"),
        level: 10,
        archetype: 1,
    }
}

/// A squad of `1` (leader) and `2`, made by the GM path.
fn pair() -> (SquadRegistry, i32) {
    let mut reg = SquadRegistry::new();
    let sid = reg.force_join(m(2), m(1)).expect("join").squad_id;
    (reg, sid)
}

#[test]
fn ping_is_members_only_and_names_their_own_squad() {
    let (mut reg, sid) = pair();
    let now = Instant::now();
    assert_eq!(reg.check_ping(9, sid, now), Err(PingReject::NotInSquad));
    assert_eq!(reg.check_ping(1, sid + 1, now), Err(PingReject::WrongSquad));
    assert_eq!(reg.check_ping(1, sid, now), Ok(()));
}

/// At most one accepted ping per second per member: 999 ms later is
/// refused, exactly 1 s after the last accepted one is allowed, and a
/// refused ping does not restart the interval.
#[test]
fn ping_is_limited_to_one_per_second_per_member() {
    let (mut reg, sid) = pair();
    let t0 = Instant::now();
    assert_eq!(reg.check_ping(1, sid, t0), Ok(()));
    let early = t0 + Duration::from_millis(999);
    assert_eq!(reg.check_ping(1, sid, early), Err(PingReject::RateLimited));
    // The limit is per member: the other member is unaffected.
    assert_eq!(reg.check_ping(2, sid, early), Ok(()));
    assert_eq!(reg.check_ping(1, sid, t0 + PING_MIN_INTERVAL), Ok(()));
}

/// Leaving drops the ping window with the membership.
#[test]
fn ping_window_is_cleared_on_departure() {
    let (mut reg, sid) = pair();
    let now = Instant::now();
    reg.check_ping(1, sid, now).unwrap();
    reg.leave(1);
    assert!(reg.last_ping.is_empty(), "{:?}", reg.last_ping);
}

/// A host in no squad founds one and leads it; the GM is a member.
#[test]
fn force_join_founds_a_squad_led_by_the_host() {
    let mut reg = SquadRegistry::new();
    let out = reg.force_join(m(2), m(1)).unwrap();
    assert_eq!(
        out,
        JoinOutcome {
            squad_id: SQUAD_ORG_ID_MIN,
            created: true
        }
    );
    let squad = reg.squad(out.squad_id).unwrap();
    assert_eq!(squad.leader_player_id(), 1);
    assert_eq!(squad.rank_of(2), OrgRank::MEMBER);
    assert_eq!(reg.squad_of(2), Some(out.squad_id));
}

/// A host who is a plain member (not the leader) still brings the GM in.
#[test]
fn force_join_enters_the_hosts_existing_squad_without_the_leader_check() {
    let (mut reg, sid) = pair();
    let out = reg.force_join(m(3), m(2)).unwrap();
    assert_eq!(
        out,
        JoinOutcome {
            squad_id: sid,
            created: false
        }
    );
    assert_eq!(reg.squad(sid).unwrap().members().len(), 3);
    assert_eq!(reg.squad(sid).unwrap().leader_player_id(), 1);
}

#[test]
fn force_join_keeps_the_membership_rules() {
    let (mut reg, sid) = pair();
    assert_eq!(reg.force_join(m(1), m(1)), Err(ForceJoinReject::SelfTarget));
    // Already in a squad (even another one's host would not matter).
    assert_eq!(
        reg.force_join(m(2), m(7)),
        Err(ForceJoinReject::AlreadyInSquad)
    );
    for p in 3..=MAX_SQUAD_SIZE as i32 {
        reg.force_join(m(p), m(1)).unwrap();
    }
    assert!(reg.squad(sid).unwrap().is_full());
    assert_eq!(reg.force_join(m(99), m(1)), Err(ForceJoinReject::SquadFull));
    assert_eq!(reg.squad_of(99), None);
}
